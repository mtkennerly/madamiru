use std::{
    collections::HashMap,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use iced::{keyboard, widget::pane_grid, Length, Subscription, Task};
use itertools::Itertools;

use crate::{
    gui::{
        button,
        common::{BrowseFileSubject, Flags, Message, PaneEvent, UndoSubject},
        grid::{self, Grid},
        icon::Icon,
        modal::{self, Modal},
        player::{self},
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{Column, Container, DropDown, Element, PaneGrid, Responsive, Row, Stack},
    },
    lang, media,
    path::StrictPath,
    prelude::{Error, STEAM_DECK},
    resource::{
        cache::Cache,
        config::{self, Config},
        playlist::{self, Playlist},
        ResourceFile, SaveableResourceFile,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SaveKind {
    Config,
    Cache,
}

pub struct App {
    config: Config,
    cache: Cache,
    modals: Vec<Modal>,
    text_histories: TextHistories,
    pending_save: HashMap<SaveKind, Instant>,
    modifiers: keyboard::Modifiers,
    grids: pane_grid::State<Grid>,
    media: media::Collection,
    last_tick: Instant,
    #[allow(unused)] // TODO: https://github.com/iced-rs/iced/pull/2691
    dragging_pane: bool,
    dragged_file: Option<StrictPath>,
    viewing_pane_controls: Option<grid::Id>,
    playlist_path: Option<StrictPath>,
    playlist_dirty: bool,
    default_audio_output_device: Option<String>,
}

impl App {
    fn show_modal(&mut self, modal: Modal) {
        self.viewing_pane_controls = None;
        self.modals.push(modal);
    }

    fn close_modal(&mut self) {
        self.modals.pop();
    }

    fn show_error(&mut self, error: Error) {
        self.show_modal(Modal::Error { variant: error })
    }

    fn save(&mut self) {
        let threshold = Duration::from_secs(1);
        let now = Instant::now();

        self.pending_save.retain(|item, then| {
            if (now - *then) < threshold {
                return true;
            }

            match item {
                SaveKind::Config => self.config.save(),
                SaveKind::Cache => self.cache.save(),
            }

            false
        });
    }

    fn save_config(&mut self) {
        self.pending_save.insert(SaveKind::Config, Instant::now());
    }

    fn save_cache(&mut self) {
        self.pending_save.insert(SaveKind::Cache, Instant::now());
    }

    fn open_url(url: String) -> Task<Message> {
        let url2 = url.clone();
        Task::future(async move {
            let result = async { opener::open(url) }.await;

            match result {
                Ok(_) => Message::Ignore,
                Err(e) => {
                    log::error!("Unable to open URL: `{}` - {}", &url2, e);
                    Message::OpenUrlFailure { url: url2 }
                }
            }
        })
    }

    pub fn new(flags: Flags) -> (Self, Task<Message>) {
        let mut errors = vec![];

        let mut modals = vec![];
        let mut config = match Config::load() {
            Ok(x) => x,
            Err(x) => {
                errors.push(x);
                let _ = Config::archive_invalid();
                Config::default()
            }
        };
        let cache = Cache::load().unwrap_or_default().migrate_config(&mut config);
        lang::set(config.language);

        let sources = flags.sources.clone();
        if let Some(max) = flags.max_initial_media {
            config.playback.max_initial_media = max;
        }

        let text_histories = TextHistories::new(&config);

        log::debug!("Config on startup: {config:?}");

        let mut commands = vec![
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::TEXT_DATA)).map(|_| Message::Ignore),
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::ICONS_DATA)).map(|_| Message::Ignore),
            iced::window::get_oldest().and_then(iced::window::gain_focus),
            iced::window::get_oldest().and_then(|id| iced::window::resize(id, iced::Size::new(960.0, 600.0))),
        ];

        if config.release.check && cache.should_check_app_update() {
            commands.push(Task::future(async move {
                let result = crate::metadata::Release::fetch().await;

                Message::AppReleaseChecked(result.map_err(|x| x.to_string()))
            }))
        }

        let mut playlist_dirty = false;
        let mut playlist_path = sources.first().and_then(|source| match source {
            media::Source::Path { path } => path
                .file_extension()
                .is_some_and(|ext| ext == Playlist::EXTENSION)
                .then_some(path.clone()),
            media::Source::Glob { .. } => None,
        });

        let grids = match playlist_path.as_ref() {
            Some(path) => match Playlist::load_from(path) {
                Ok(playlist) => {
                    commands.push(Self::find_media(playlist.sources(), media::RefreshContext::Launch));
                    Self::load_playlist(playlist)
                }
                Err(e) => {
                    playlist_path = None;
                    errors.push(e);
                    let (grids, _grid_id) = pane_grid::State::new(Grid::new(&grid::Settings::default()));
                    grids
                }
            },
            None => {
                let grid_settings = grid::Settings::default().with_sources(sources.clone());
                let (grids, grid_id) = pane_grid::State::new(Grid::new(&grid_settings));

                if sources.is_empty() {
                    modals.push(Modal::new_grid_settings(grid_id, grid_settings));
                } else {
                    playlist_dirty = true;
                }
                commands.push(Self::find_media(sources, media::RefreshContext::Launch));
                grids
            }
        };

        if !errors.is_empty() {
            modals.push(Modal::Errors { errors });
        }

        (
            Self {
                config,
                cache,
                modals,
                text_histories,
                pending_save: Default::default(),
                modifiers: Default::default(),
                grids,
                media: Default::default(),
                last_tick: Instant::now(),
                dragging_pane: false,
                dragged_file: None,
                viewing_pane_controls: None,
                playlist_path,
                playlist_dirty,
                default_audio_output_device: Self::get_audio_device(),
            },
            Task::batch(commands),
        )
    }

    pub fn title(&self) -> String {
        let base = lang::window_title();

        match self.playlist_path.as_ref().map(|x| x.render()) {
            Some(playlist) => format!("{base} | {}{playlist}", if self.playlist_dirty { "*" } else { "" }),
            None => base,
        }
    }

    pub fn theme(&self) -> crate::gui::style::Theme {
        crate::gui::style::Theme::from(self.config.theme)
    }

    fn refresh(&mut self) -> Task<Message> {
        for (_id, grid) in self.grids.iter_mut() {
            grid.refresh(&mut self.media, &self.config.playback);
        }
        Task::none()
    }

    fn all_idle(&self) -> bool {
        self.grids.iter().all(|(_id, grid)| grid.is_idle())
    }

    fn all_paused(&self) -> Option<bool> {
        let mut relevant = false;
        for (_grid_id, grid) in self.grids.iter() {
            match grid.all_paused() {
                Some(true) => {
                    relevant = true;
                }
                Some(false) => {
                    return Some(false);
                }
                None => {}
            }
        }

        relevant.then_some(true)
    }

    fn all_muted(&self) -> Option<bool> {
        let mut relevant = false;
        for (_grid_id, grid) in self.grids.iter() {
            match grid.all_muted() {
                Some(true) => {
                    relevant = true;
                }
                Some(false) => {
                    return Some(false);
                }
                None => {}
            }
        }

        relevant.then_some(true)
    }

    fn set_paused(&mut self, paused: bool) {
        self.config.playback.paused = paused;
        self.save_config();

        for (_grid_id, grid) in self.grids.iter_mut() {
            grid.update_all_players(player::Event::SetPause(paused), &mut self.media, &self.config.playback);
        }
    }

    fn set_muted(&mut self, muted: bool) {
        self.config.playback.muted = muted;
        self.save_config();

        for (_grid_id, grid) in self.grids.iter_mut() {
            grid.update_all_players(player::Event::SetMute(muted), &mut self.media, &self.config.playback);
        }
    }

    fn can_jump(&self) -> bool {
        self.grids.iter().any(|(_grid_id, grid)| grid.can_jump())
    }

    fn all_sources(&self) -> Vec<media::Source> {
        self.grids
            .iter()
            .flat_map(|(_grid_id, grid)| grid.sources())
            .unique()
            .cloned()
            .collect()
    }

    fn find_media(sources: Vec<media::Source>, context: media::RefreshContext) -> Task<Message> {
        Task::future(async move {
            match tokio::task::spawn_blocking(move || media::Collection::find(&sources)).await {
                Ok(media) => {
                    log::debug!("Found media: {media:?}");
                    Message::MediaFound { context, media }
                }
                Err(e) => {
                    log::error!("Unable to find media: {e:?}");
                    Message::Ignore
                }
            }
        })
    }

    fn build_playlist(&self) -> Playlist {
        Playlist::new(Self::build_playlist_layout(&self.grids, self.grids.layout()))
    }

    fn build_playlist_layout(panes: &pane_grid::State<Grid>, node: &pane_grid::Node) -> playlist::Layout {
        match node {
            pane_grid::Node::Split {
                axis,
                ratio,
                a: first,
                b: second,
                ..
            } => playlist::Layout::Split(playlist::Split {
                axis: match axis {
                    pane_grid::Axis::Horizontal => playlist::SplitAxis::Horizontal,
                    pane_grid::Axis::Vertical => playlist::SplitAxis::Vertical,
                },
                ratio: *ratio,
                first: Box::new(Self::build_playlist_layout(panes, first)),
                second: Box::new(Self::build_playlist_layout(panes, second)),
            }),
            pane_grid::Node::Pane(pane) => match panes.get(*pane) {
                Some(grid) => {
                    let grid::Settings {
                        sources,
                        content_fit,
                        orientation,
                        orientation_limit,
                    } = grid.settings();
                    playlist::Layout::Group(playlist::Group {
                        sources,
                        max_media: grid.total_players(),
                        content_fit,
                        orientation,
                        orientation_limit,
                    })
                }
                None => playlist::Layout::Group(playlist::Group::default()),
            },
        }
    }

    fn load_playlist(playlist: Playlist) -> pane_grid::State<Grid> {
        let configuration = Self::load_playlist_layout(playlist.layout);
        pane_grid::State::with_configuration(configuration)
    }

    fn load_playlist_layout(layout: playlist::Layout) -> pane_grid::Configuration<Grid> {
        match layout {
            playlist::Layout::Split(playlist::Split {
                axis,
                ratio,
                first,
                second,
            }) => pane_grid::Configuration::Split {
                axis: match axis {
                    playlist::SplitAxis::Horizontal => pane_grid::Axis::Horizontal,
                    playlist::SplitAxis::Vertical => pane_grid::Axis::Vertical,
                },
                ratio,
                a: Box::new(Self::load_playlist_layout(*first)),
                b: Box::new(Self::load_playlist_layout(*second)),
            },
            playlist::Layout::Group(playlist::Group {
                sources,
                max_media,
                content_fit,
                orientation,
                orientation_limit,
            }) => {
                let settings = grid::Settings {
                    sources,
                    content_fit,
                    orientation,
                    orientation_limit,
                };
                pane_grid::Configuration::Pane(Grid::new_with_players(&settings, max_media))
            }
        }
    }

    fn get_audio_device() -> Option<String> {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        host.default_output_device().and_then(|d| d.name().ok())
    }

    /// Rodio/CPAL don't automatically follow changes to the default output device,
    /// so we need to reload the streams if that happens.
    /// More info:
    /// * https://github.com/RustAudio/cpal/issues/740
    /// * https://github.com/RustAudio/rodio/issues/327
    /// * https://github.com/RustAudio/rodio/issues/544
    fn did_audio_device_change(&mut self) -> bool {
        let device = Self::get_audio_device();

        if self.default_audio_output_device != device {
            log::info!(
                "Default audio device changed: {:?} -> {:?}",
                self.default_audio_output_device.as_ref(),
                device.as_ref()
            );
            self.default_audio_output_device = device;
            true
        } else {
            false
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ignore => Task::none(),
            Message::Exit { force } => {
                if self.playlist_dirty && !force {
                    self.show_modal(Modal::ConfirmDiscardPlaylist { exit: true });
                    return Task::none();
                }

                // If we don't pause first, you may still hear the videos for a moment after the app closes.
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::SetPause(true), &mut self.media, &self.config.playback);
                }
                std::process::exit(0)
            }
            Message::Tick(instant) => {
                let elapsed = instant - self.last_tick;
                self.last_tick = instant;

                for (_id, grid) in self.grids.iter_mut() {
                    grid.tick(elapsed, &mut self.media, &self.config.playback);
                }
                Task::none()
            }
            Message::CheckAudio => {
                if self.did_audio_device_change() {
                    for (_id, grid) in self.grids.iter_mut() {
                        grid.reload_audio(&self.config.playback);
                    }
                }
                Task::none()
            }
            Message::Save => {
                self.save();
                Task::none()
            }
            Message::CloseModal => {
                self.close_modal();

                if self
                    .text_histories
                    .max_initial_media
                    .current()
                    .parse::<NonZeroUsize>()
                    .is_err()
                {
                    self.text_histories
                        .max_initial_media
                        .push(&self.config.playback.max_initial_media.to_string());
                }

                if self
                    .text_histories
                    .image_duration
                    .current()
                    .parse::<NonZeroUsize>()
                    .is_err()
                {
                    self.text_histories
                        .image_duration
                        .push(&self.config.playback.image_duration.to_string());
                }

                Task::none()
            }
            Message::Config { event } => {
                match event {
                    config::Event::Theme(value) => {
                        self.config.theme = value;
                    }
                    config::Event::Language(value) => {
                        lang::set(value);
                        self.config.language = value;
                    }
                    config::Event::CheckRelease(value) => {
                        self.config.release.check = value;
                    }
                    config::Event::MaxInitialMediaRaw(value) => {
                        self.text_histories.max_initial_media.push(&value.to_string());
                        if let Ok(value) = value.parse::<usize>() {
                            self.config.playback.max_initial_media = value;
                        }
                    }
                    config::Event::ImageDurationRaw(value) => {
                        self.text_histories.image_duration.push(&value.to_string());
                        if let Ok(value) = value.parse::<NonZeroUsize>() {
                            self.config.playback.image_duration = value;
                        }
                    }
                    config::Event::PauseWhenWindowLosesFocus(value) => {
                        self.config.playback.pause_on_unfocus = value;
                    }
                }
                self.save_config();
                Task::none()
            }
            Message::CheckAppRelease => {
                if !self.cache.should_check_app_update() {
                    return Task::none();
                }

                Task::future(async move {
                    let result = crate::metadata::Release::fetch().await;

                    Message::AppReleaseChecked(result.map_err(|x| x.to_string()))
                })
            }
            Message::AppReleaseChecked(outcome) => {
                self.save_cache();
                self.cache.release.checked = chrono::offset::Utc::now();

                match outcome {
                    Ok(release) => {
                        let previous_latest = self.cache.release.latest.clone();
                        self.cache.release.latest = Some(release.version.clone());

                        if previous_latest.as_ref() != Some(&release.version) {
                            // The latest available version has changed (or this is our first time checking)
                            if release.is_update() {
                                self.show_modal(Modal::AppUpdate { release });
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("App update check failed: {e:?}");
                    }
                }

                Task::none()
            }
            Message::BrowseDir(subject) => Task::future(async move {
                let choice = async move { rfd::AsyncFileDialog::new().pick_folder().await }.await;

                Message::browsed_dir(subject, choice.map(|x| x.path().to_path_buf()))
            }),
            Message::BrowseFile(subject) => Task::future(async move {
                let choice = async move { rfd::AsyncFileDialog::new().pick_file().await }.await;

                Message::browsed_file(subject, choice.map(|x| x.path().to_path_buf()))
            }),
            Message::OpenDir { path } => {
                let path2 = path.clone();
                Task::future(async move {
                    let result = async { opener::open(path.resolve()) }.await;

                    match result {
                        Ok(_) => Message::Ignore,
                        Err(e) => {
                            log::error!("Unable to open directory: `{}` - {:?}", path2.resolve(), e);
                            Message::OpenDirFailure { path: path2 }
                        }
                    }
                })
            }
            Message::OpenFile { path } => {
                let path = match path.parent_if_file() {
                    Ok(path) => path,
                    Err(_) => {
                        self.show_error(Error::UnableToOpenDir(path));
                        return Task::none();
                    }
                };

                let path2 = path.clone();
                Task::future(async move {
                    let result = async { opener::open(path.resolve()) }.await;

                    match result {
                        Ok(_) => Message::Ignore,
                        Err(e) => {
                            log::error!("Unable to open directory: `{}` - {:?}", path2.resolve(), e);
                            Message::OpenDirFailure { path: path2 }
                        }
                    }
                })
            }
            Message::OpenDirFailure { path } => {
                self.show_modal(Modal::Error {
                    variant: Error::UnableToOpenDir(path),
                });
                Task::none()
            }
            Message::OpenUrlFailure { url } => {
                self.show_modal(Modal::Error {
                    variant: Error::UnableToOpenUrl(url),
                });
                Task::none()
            }
            Message::KeyboardEvent(event) => {
                if let iced::keyboard::Event::ModifiersChanged(modifiers) = event {
                    self.modifiers = modifiers;
                }
                match event {
                    iced::keyboard::Event::KeyPressed {
                        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab),
                        modifiers,
                        ..
                    } => {
                        if modifiers.shift() {
                            iced::widget::focus_previous()
                        } else {
                            iced::widget::focus_next()
                        }
                    }
                    iced::keyboard::Event::KeyPressed { key, .. } => {
                        if self.modals.is_empty() {
                            match key {
                                iced::keyboard::Key::Named(iced::keyboard::key::Named::Space) => {
                                    self.set_paused(!self.config.playback.paused);
                                }
                                iced::keyboard::Key::Character(c) => match c.as_str() {
                                    "M" | "m" => {
                                        self.set_muted(!self.config.playback.muted);
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
            Message::UndoRedo(action, subject) => {
                let shortcut = Shortcut::from(action);
                let captured = self
                    .modals
                    .last_mut()
                    .map(|modal| modal.apply_shortcut(subject, shortcut))
                    .unwrap_or(false);

                if !captured {
                    match subject {
                        UndoSubject::MaxInitialMedia => {
                            if let Ok(value) = self.text_histories.max_initial_media.apply(shortcut).parse::<usize>() {
                                self.config.playback.max_initial_media = value;
                            }
                        }
                        UndoSubject::ImageDuration => {
                            if let Ok(value) = self
                                .text_histories
                                .image_duration
                                .apply(shortcut)
                                .parse::<NonZeroUsize>()
                            {
                                self.config.playback.image_duration = value;
                            }
                        }
                        UndoSubject::Source { .. } => {}
                        UndoSubject::OrientationLimit => {}
                    }
                }

                self.save_config();
                Task::none()
            }
            Message::OpenUrl(url) => Self::open_url(url),
            Message::OpenUrlAndCloseModal(url) => {
                self.close_modal();
                Self::open_url(url)
            }
            Message::Refresh => self.refresh(),
            Message::SetPause(flag) => {
                self.set_paused(flag);
                Task::none()
            }
            Message::SetMute(flag) => {
                self.set_muted(flag);
                Task::none()
            }
            Message::Player {
                grid_id,
                player_id,
                event,
            } => {
                let Some(grid) = self.grids.get_mut(grid_id) else {
                    return Task::none();
                };

                if let Some(update) = grid.update(
                    grid::Event::Player { player_id, event },
                    &mut self.media,
                    &self.config.playback,
                ) {
                    let Some(grid) = self.grids.get(grid_id) else {
                        return Task::none();
                    };

                    match update {
                        grid::Update::PauseChanged { .. } => {
                            if let Some(paused) = self.all_paused() {
                                self.config.playback.paused = paused;
                                self.save_config();
                            }
                        }
                        grid::Update::MuteChanged { .. } => {
                            if let Some(muted) = self.all_muted() {
                                self.config.playback.muted = muted;
                                self.save_config();
                            }
                        }
                        grid::Update::PlayerClosed => {
                            self.playlist_dirty = true;
                            if grid.is_idle() {
                                self.show_modal(Modal::new_grid_settings(grid_id, grid.settings()));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::AllPlayers { event } => {
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(event.clone(), &mut self.media, &self.config.playback);
                }
                Task::none()
            }
            Message::Modal { event } => {
                if let Some(modal) = self.modals.last_mut() {
                    if let Some(update) = modal.update(event) {
                        match update {
                            modal::Update::SavedGridSettings { grid_id, settings } => {
                                self.modals.pop();
                                let sources = settings.sources.clone();
                                if let Some(grid) = self.grids.get_mut(grid_id) {
                                    self.playlist_dirty = true;
                                    grid.set_settings(settings);
                                }
                                return Self::find_media(sources, media::RefreshContext::Edit);
                            }
                            modal::Update::Task(task) => {
                                return task;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ShowSettings => {
                self.show_modal(Modal::Settings);
                Task::none()
            }
            Message::FindMedia => Self::find_media(self.all_sources(), media::RefreshContext::Automatic),
            Message::MediaFound { context, media } => {
                self.media.update(media, context);
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.refresh_on_media_collection_changed(context, &mut self.media, &self.config.playback);
                }
                Task::none()
            }
            Message::FileDragDrop(path) => {
                if path.file_extension().is_some_and(|ext| ext == Playlist::EXTENSION) {
                    match self.modals.last() {
                        Some(_) => Task::none(),
                        None => {
                            if self.playlist_dirty {
                                self.show_modal(Modal::ConfirmLoadPlaylist { path: Some(path) });
                                Task::none()
                            } else {
                                Task::done(Message::PlaylistLoad { path })
                            }
                        }
                    }
                } else {
                    match self.modals.last_mut() {
                        Some(Modal::GridSettings {
                            settings, histories, ..
                        }) => {
                            histories.sources.push(TextHistory::path(&path));
                            settings.sources.push(media::Source::new_path(path));
                            Task::batch([
                                iced::window::get_oldest().and_then(iced::window::gain_focus),
                                modal::scroll_down(),
                            ])
                        }
                        Some(_) => Task::none(),
                        None => {
                            if self.grids.len() == 1 {
                                let (grid_id, grid) = self.grids.iter().last().unwrap();

                                let settings = grid.settings().with_source(media::Source::new_path(path));

                                self.show_modal(Modal::new_grid_settings(*grid_id, settings));
                                Task::batch([
                                    iced::window::get_oldest().and_then(iced::window::gain_focus),
                                    modal::scroll_down(),
                                ])
                            } else {
                                self.dragged_file = Some(path);
                                iced::window::get_oldest().and_then(iced::window::gain_focus)
                            }
                        }
                    }
                }
            }
            Message::FileDragDropGridSelected(grid_id) => {
                let Some(grid) = self.grids.get(grid_id) else {
                    return Task::none();
                };

                let Some(path) = self.dragged_file.take() else {
                    return Task::none();
                };

                let settings = grid.settings().with_source(media::Source::new_path(path));

                self.show_modal(Modal::new_grid_settings(grid_id, settings));
                modal::scroll_down()
            }
            Message::WindowFocused => {
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::WindowFocused, &mut self.media, &self.config.playback);
                }
                Task::none()
            }
            Message::WindowUnfocused => {
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::WindowUnfocused, &mut self.media, &self.config.playback);
                }
                Task::none()
            }
            Message::Pane { event } => {
                match event {
                    PaneEvent::Drag(event) => match event {
                        pane_grid::DragEvent::Picked { .. } => {
                            self.dragging_pane = true;
                        }
                        pane_grid::DragEvent::Dropped { pane, target } => {
                            self.playlist_dirty = true;
                            self.dragging_pane = false;
                            self.grids.drop(pane, target);
                        }
                        pane_grid::DragEvent::Canceled { .. } => {
                            self.dragging_pane = false;
                        }
                    },
                    PaneEvent::Resize(event) => {
                        self.playlist_dirty = true;
                        self.grids.resize(event.split, event.ratio);
                    }
                    PaneEvent::Split { grid_id, axis } => {
                        self.playlist_dirty = true;
                        let idle = self.grids.get(grid_id).is_some_and(|grid| grid.is_idle());
                        let settings = grid::Settings::default();
                        if let Some((grid_id, _split)) = self.grids.split(axis, grid_id, Grid::new(&settings)) {
                            if !idle {
                                self.show_modal(Modal::new_grid_settings(grid_id, settings));
                            }
                        }
                    }
                    PaneEvent::Close { grid_id } => {
                        self.playlist_dirty = true;
                        self.grids.close(grid_id);
                    }
                    PaneEvent::AddPlayer { grid_id } => {
                        self.playlist_dirty = true;
                        let Some(grid) = self.grids.get_mut(grid_id) else {
                            return Task::none();
                        };

                        match grid.add_player(&mut self.media, &self.config.playback) {
                            Ok(_) => {}
                            Err(e) => match e {
                                grid::Error::NoMediaAvailable => {
                                    self.show_modal(Modal::Error {
                                        variant: Error::NoMediaFound,
                                    });
                                }
                            },
                        }
                    }
                    PaneEvent::ShowSettings { grid_id } => {
                        if let Some(grid) = self.grids.get(grid_id) {
                            self.show_modal(Modal::new_grid_settings(grid_id, grid.settings()));
                        }
                    }
                    PaneEvent::ShowControls { grid_id } => {
                        if self.viewing_pane_controls.is_some_and(|x| x == grid_id) {
                            self.viewing_pane_controls = None;
                        } else {
                            self.viewing_pane_controls = Some(grid_id);
                        }
                    }
                    PaneEvent::CloseControls => {
                        self.viewing_pane_controls = None;
                    }
                    PaneEvent::SetMute { grid_id, muted } => {
                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(
                                player::Event::SetMute(muted),
                                &mut self.media,
                                &self.config.playback,
                            );

                            if let Some(muted) = self.all_muted() {
                                self.config.playback.muted = muted;
                                self.save_config();
                            }
                        }
                    }
                    PaneEvent::SetPause { grid_id, paused } => {
                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(
                                player::Event::SetPause(paused),
                                &mut self.media,
                                &self.config.playback,
                            );

                            if let Some(paused) = self.all_paused() {
                                self.config.playback.paused = paused;
                                self.save_config();
                            }
                        }
                    }
                    PaneEvent::SeekRandom { grid_id } => {
                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(player::Event::SeekRandom, &mut self.media, &self.config.playback);
                        }
                    }
                    PaneEvent::Refresh { grid_id } => {
                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(player::Event::Refresh, &mut self.media, &self.config.playback);
                        }
                    }
                }
                Task::none()
            }
            Message::PlaylistReset { force } => {
                if self.playlist_dirty && !force {
                    self.show_modal(Modal::ConfirmDiscardPlaylist { exit: false });
                    return Task::none();
                }

                self.close_modal();
                let (grids, _grid_id) = pane_grid::State::new(Grid::new(&grid::Settings::default()));
                self.grids = grids;
                self.playlist_dirty = false;
                self.playlist_path = None;

                Task::none()
            }
            Message::PlaylistSelect { force } => {
                if self.playlist_dirty && !force {
                    self.show_modal(Modal::ConfirmLoadPlaylist { path: None });
                    return Task::none();
                }

                self.close_modal();

                Task::future(async move {
                    let choice = async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter(lang::thing::playlist(), &[Playlist::EXTENSION])
                            .pick_file()
                            .await
                    }
                    .await;

                    Message::browsed_file(
                        BrowseFileSubject::Playlist { save: false },
                        choice.map(|x| x.path().to_path_buf()),
                    )
                })
            }
            Message::PlaylistLoad { path } => {
                self.playlist_dirty = false;
                self.playlist_path = Some(path.clone());
                self.modals.clear();

                match Playlist::load_from(&path) {
                    Ok(playlist) => {
                        self.grids = Self::load_playlist(playlist);
                        Self::find_media(self.all_sources(), media::RefreshContext::Playlist)
                    }
                    Err(e) => {
                        self.show_error(e);
                        Task::none()
                    }
                }
            }
            Message::PlaylistSave => {
                if let Some(path) = self.playlist_path.as_ref() {
                    let playlist = self.build_playlist();
                    match playlist.save_to(path) {
                        Ok(_) => {
                            self.playlist_dirty = false;
                        }
                        Err(e) => {
                            self.show_error(e);
                        }
                    }
                }

                Task::none()
            }
            Message::PlaylistSaveAs => Task::future(async move {
                let choice = async move {
                    rfd::AsyncFileDialog::new()
                        .set_file_name(Playlist::FILE_NAME)
                        .add_filter(lang::thing::playlist(), &[Playlist::EXTENSION])
                        .save_file()
                        .await
                }
                .await;

                Message::browsed_file(
                    BrowseFileSubject::Playlist { save: true },
                    choice.map(|x| x.path().to_path_buf()),
                )
            }),
            Message::PlaylistSavedAs { path } => {
                self.playlist_path = Some(path.clone());

                let playlist = self.build_playlist();
                match playlist.save_to(&path) {
                    Ok(_) => {
                        self.playlist_dirty = false;
                    }
                    Err(e) => {
                        self.show_error(e);
                    }
                }

                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Exit { force: false }),
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    Some(Message::FileDragDrop(StrictPath::from(path)))
                }
                iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused),
                iced::Event::Window(iced::window::Event::Unfocused) => Some(Message::WindowUnfocused),
                _ => None,
            }),
            iced::time::every(Duration::from_millis(100)).map(Message::Tick),
            iced::time::every(Duration::from_millis(1000)).map(|_| Message::CheckAudio),
            iced::time::every(Duration::from_secs(60 * 10)).map(|_| Message::FindMedia),
        ];

        if !self.pending_save.is_empty() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::Save));
        }

        if self.config.release.check {
            subscriptions.push(iced::time::every(Duration::from_secs(60 * 60 * 24)).map(|_| Message::CheckAppRelease));
        }

        iced::Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element {
        let dragging_file = self.dragged_file.is_some();
        let obscured = !self.modals.is_empty();

        Responsive::new(move |viewport| {
            let content =
                Container::new(
                    Column::new()
                        .spacing(5)
                        .push(
                            Stack::new()
                                .push(
                                    Container::new(
                                        Row::new().push(
                                            button::icon(Icon::Settings)
                                                .on_press(Message::ShowSettings)
                                                .obscured(obscured)
                                                .tooltip_below(lang::thing::settings()),
                                        ),
                                    )
                                    .align_right(Length::Fill),
                                )
                                .push(
                                    Container::new(
                                        Row::new()
                                            .push_maybe(STEAM_DECK.then(|| {
                                                button::icon(Icon::LogOut)
                                                    .on_press(Message::Exit { force: false })
                                                    .obscured(obscured)
                                                    .tooltip_below(lang::action::exit_app())
                                            }))
                                            .push(
                                                button::icon(Icon::PlaylistRemove)
                                                    .on_press(Message::PlaylistReset { force: false })
                                                    .enabled(self.playlist_dirty || self.playlist_path.is_some())
                                                    .obscured(obscured)
                                                    .tooltip_below(lang::action::start_new_playlist()),
                                            )
                                            .push(
                                                button::icon(Icon::FolderOpen)
                                                    .on_press(Message::PlaylistSelect { force: false })
                                                    .obscured(obscured)
                                                    .tooltip_below(lang::action::open_playlist()),
                                            )
                                            .push(
                                                button::icon(Icon::Save)
                                                    .on_press(Message::PlaylistSave)
                                                    .enabled(self.playlist_dirty && self.playlist_path.is_some())
                                                    .obscured(obscured)
                                                    .tooltip_below(lang::action::save_playlist()),
                                            )
                                            .push(
                                                button::icon(Icon::SaveAs)
                                                    .on_press(Message::PlaylistSaveAs)
                                                    .obscured(obscured)
                                                    .tooltip_below(lang::action::save_playlist_as_new_file()),
                                            ),
                                    )
                                    .align_left(Length::Fill),
                                )
                                .push(
                                    Container::new(
                                        Container::new(
                                            Row::new()
                                                .push(
                                                    button::icon(if self.config.playback.muted {
                                                        Icon::Mute
                                                    } else {
                                                        Icon::VolumeHigh
                                                    })
                                                    .on_press(Message::SetMute(!self.config.playback.muted))
                                                    .obscured(obscured)
                                                    .tooltip_below(if self.config.playback.muted {
                                                        lang::action::unmute()
                                                    } else {
                                                        lang::action::mute()
                                                    }),
                                                )
                                                .push(
                                                    button::icon(if self.config.playback.paused {
                                                        Icon::Play
                                                    } else {
                                                        Icon::Pause
                                                    })
                                                    .on_press(Message::SetPause(!self.config.playback.paused))
                                                    .obscured(obscured)
                                                    .tooltip_below(if self.config.playback.paused {
                                                        lang::action::play()
                                                    } else {
                                                        lang::action::pause()
                                                    }),
                                                )
                                                .push(
                                                    button::icon(Icon::TimerRefresh)
                                                        .on_press(Message::AllPlayers {
                                                            event: player::Event::SeekRandom,
                                                        })
                                                        .enabled(!self.all_idle() && self.can_jump())
                                                        .obscured(obscured)
                                                        .tooltip_below(lang::action::jump_position()),
                                                )
                                                .push(
                                                    button::icon(Icon::Refresh)
                                                        .on_press(Message::Refresh)
                                                        .enabled(!self.all_idle())
                                                        .obscured(obscured)
                                                        .tooltip_below(lang::action::shuffle_media()),
                                                ),
                                        )
                                        .class(style::Container::Player),
                                    )
                                    .center(Length::Fill),
                                ),
                        )
                        .push(
                            PaneGrid::new(&self.grids, |grid_id, grid, _maximized| {
                                pane_grid::Content::new(
                                    Container::new(grid.view(grid_id, obscured, dragging_file))
                                        .padding(5)
                                        .class(style::Container::PlayerGroup),
                                )
                                .title_bar({
                                    let mut bar = pane_grid::TitleBar::new(" ")
                                        .class(style::Container::PlayerGroupTitle)
                                        .controls(pane_grid::Controls::dynamic(
                                            grid.controls(grid_id, obscured, self.grids.len() > 1),
                                            DropDown::new(
                                                button::mini_icon(Icon::MoreVert)
                                                    .on_press(Message::Pane {
                                                        event: PaneEvent::ShowControls { grid_id },
                                                    })
                                                    .obscured(obscured),
                                                Container::new(grid.controls(grid_id, obscured, self.grids.len() > 1))
                                                    .class(style::Container::PlayerGroupControls),
                                                self.viewing_pane_controls.is_some_and(|x| x == grid_id),
                                            )
                                            .on_dismiss(Message::Pane {
                                                event: PaneEvent::CloseControls,
                                            }),
                                        ));

                                    if grid.is_idle() {
                                        bar = bar.always_show_controls();
                                    }

                                    bar
                                })
                            })
                            .spacing(5)
                            .on_drag(|event| Message::Pane {
                                event: PaneEvent::Drag(event),
                            })
                            .on_resize(5, |event| Message::Pane {
                                event: PaneEvent::Resize(event),
                            }),
                        ),
                );

            let stack = Stack::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(content.class(style::Container::Primary))
                .push_maybe(
                    self.modals
                        .last()
                        .map(|modal| modal.view(viewport, &self.config, &self.text_histories, &self.modifiers)),
                );

            Container::new(stack)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(5.0)
                .into()
        })
        .into()
    }
}
