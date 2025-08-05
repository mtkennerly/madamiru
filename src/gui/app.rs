use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use iced::{keyboard, widget::pane_grid, Length, Subscription, Task};
use itertools::Itertools;

use crate::{
    gui::{
        button,
        common::{BrowseFileSubject, Flags, Message, PaneEvent, Selection, Step, UndoSubject},
        grid::{self, Grid},
        icon::Icon,
        modal::{self, Modal},
        player::{self, Player},
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{Column, Container, DropDown, Element, PaneGrid, Responsive, Row, Stack},
    },
    lang, media,
    path::StrictPath,
    prelude::{Change, Error, STEAM_DECK},
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
    dragged_files: HashSet<StrictPath>,
    viewing_menu: bool,
    viewing_pane_controls: Option<grid::Id>,
    playlist_path: Option<StrictPath>,
    playlist_dirty: bool,
    selection: Selection,
    #[cfg_attr(not(feature = "audio"), allow(unused))]
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
        lang::set(config.view.language);

        let sources = flags.sources.clone();

        let text_histories = TextHistories::new(&config);

        log::debug!("Config on startup: {config:?}");

        let mut commands = vec![
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::TEXT_DATA)).map(|_| Message::Ignore),
            iced::font::load(std::borrow::Cow::Borrowed(crate::gui::font::ICONS_DATA)).map(|_| Message::Ignore),
            iced::window::get_oldest().and_then(iced::window::gain_focus),
            iced::window::get_oldest().and_then(|id| iced::window::resize(id, iced::Size::new(930.0, 600.0))),
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
                    commands.push(Self::find_media(
                        playlist.sources(),
                        media::RefreshContext::Launch,
                        playlist_path.clone(),
                    ));
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
                commands.push(Self::find_media(
                    sources,
                    media::RefreshContext::Launch,
                    playlist_path.clone(),
                ));
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
                dragged_files: Default::default(),
                viewing_menu: false,
                viewing_pane_controls: None,
                playlist_path,
                playlist_dirty,
                selection: Default::default(),
                #[cfg(feature = "audio")]
                default_audio_output_device: Self::get_audio_device(),
                #[cfg(not(feature = "audio"))]
                default_audio_output_device: None,
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
        crate::gui::style::Theme::from(self.config.view.theme)
    }

    fn refresh(&mut self, context: media::RefreshContext) {
        self.media.prune(&self.all_sources());
        for (_id, grid) in self.grids.iter_mut() {
            grid.refresh(&mut self.media, &self.config.playback, context);
        }
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

    fn generate_event_in_selection(
        &mut self,
        from_app: impl FnOnce(&Self) -> Option<Message>,
        from_grid: impl FnOnce(grid::Id, &Grid) -> Option<PaneEvent>,
        from_player: impl FnOnce(&Player) -> Option<player::Event>,
    ) -> Task<Message> {
        self.generate_event_in_selection_maybe(from_app, from_grid, from_player)
            .unwrap_or_else(Task::none)
    }

    fn generate_event_in_selection_maybe(
        &mut self,
        from_app: impl FnOnce(&Self) -> Option<Message>,
        from_grid: impl FnOnce(grid::Id, &Grid) -> Option<PaneEvent>,
        from_player: impl FnOnce(&Player) -> Option<player::Event>,
    ) -> Option<Task<Message>> {
        match self.selection.pair() {
            Some((grid_id, player_id)) => {
                let grid = self.grids.get_mut(grid_id)?;
                match player_id {
                    Some(player_id) => {
                        let player = grid.player(player_id)?;
                        let event = from_player(player)?;
                        Some(self.update(Message::Player {
                            grid_id,
                            player_id,
                            event,
                        }))
                    }
                    None => {
                        let event = from_grid(grid_id, grid)?;
                        Some(self.update(Message::Pane { event }))
                    }
                }
            }
            None => {
                let message = from_app(self)?;
                Some(self.update(message))
            }
        }
    }

    fn set_muted(&mut self, muted: bool) {
        self.config.playback.muted = muted;
        self.save_config();

        for (_grid_id, grid) in self.grids.iter_mut() {
            grid.update_all_players(player::Event::SetMute(muted), &mut self.media, &self.config.playback);
        }
    }

    fn set_volume(&mut self, volume: f32) {
        self.config.playback.volume = volume;
        self.save_config();

        for (_grid_id, grid) in self.grids.iter_mut() {
            grid.update_all_players(player::Event::SetVolume(volume), &mut self.media, &self.config.playback);
        }
    }

    fn set_synchronized(&mut self, synchronized: bool) {
        self.config.playback.synchronized = synchronized;
        self.save_config();
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

    fn find_media(
        sources: Vec<media::Source>,
        context: media::RefreshContext,
        playlist: Option<StrictPath>,
    ) -> Task<Message> {
        log::info!("Finding media ({context:?})");
        let mut tasks = vec![];

        for source in sources {
            let playlist = playlist.clone();
            tasks.push(Task::future(async move {
                match tokio::task::spawn_blocking(move || {
                    media::Collection::find(media::Scan::Source {
                        source,
                        playlist,
                        context,
                    })
                })
                .await
                {
                    Ok(scans) => Message::MediaScanned(scans),
                    Err(error) => {
                        log::error!("Failed to join task for media scan: {error:?}");
                        Message::Ignore
                    }
                }
            }));
        }

        Task::batch(tasks)
    }

    fn find_media_one(scan: media::Scan) -> Task<Message> {
        Task::future(async move {
            match tokio::task::spawn_blocking(move || media::Collection::find(scan)).await {
                Ok(scans) => Message::MediaScanned(scans),
                Err(_) => Message::Ignore,
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

    #[cfg(feature = "audio")]
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
    #[cfg(feature = "audio")]
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

    fn update_playback(&mut self) {
        if let Some(paused) = self.all_paused() {
            self.config.playback.paused = paused;
        }

        if let Some(muted) = self.all_muted() {
            if self.config.playback.muted != muted {
                self.config.playback.muted = muted;
                self.save_config();
            }
        }
    }

    fn synchronize_players(&mut self, originator: grid::Id, category: player::Category, event: player::Event) {
        if !self.config.playback.synchronized {
            return;
        }
        for (other_grid_id, grid) in self.grids.iter_mut() {
            if *other_grid_id == originator {
                continue;
            }
            grid.synchronize_players(None, category, event.clone(), &self.config.playback);
        }
    }

    fn selectables(&self) -> Vec<(grid::Id, Option<player::Id>)> {
        let mut out = vec![];

        for (grid_id, grid) in self.grids.iter() {
            let player_ids = grid.player_ids();
            if player_ids.len() != 1 {
                out.push((*grid_id, None));
            }
            for player_id in player_ids {
                out.push((*grid_id, Some(player_id)));
            }
        }

        out
    }

    fn selectables_in_grid(&self) -> Vec<(grid::Id, player::Id)> {
        let mut out = vec![];

        for (grid_id, grid) in self.grids.iter() {
            if self.selection.is_grid_selected(*grid_id) {
                for player_id in grid.player_ids() {
                    out.push((*grid_id, player_id));
                }
                break;
            }
        }

        out
    }

    fn handle_grid_update(&mut self, update: grid::Update, grid_id: grid::Id) {
        match update {
            grid::Update::PauseChanged { category, paused } => {
                self.update_playback();
                self.synchronize_players(grid_id, category, player::Event::SetPause(paused));
            }
            grid::Update::MuteChanged => {
                self.update_playback();
            }
            grid::Update::RelativePositionChanged { category, position } => {
                self.synchronize_players(grid_id, category, player::Event::SeekRelative(position));
            }
            grid::Update::Step { category, step } => {
                self.synchronize_players(grid_id, category, player::Event::Step(step));
            }
            grid::Update::PlayerClosed => {
                self.playlist_dirty = true;
                self.update_playback();
                self.selection.ensure_valid_in_grid(self.selectables_in_grid());

                if let Some(grid) = self.grids.get(grid_id) {
                    if grid.is_idle() {
                        self.show_modal(Modal::new_grid_settings(grid_id, grid.settings()));
                    }
                };
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ignore => Task::none(),
            Message::Exit { force } => {
                if self.playlist_dirty && !force && self.config.view.confirm_discard_playlist {
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
            #[cfg(feature = "audio")]
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
                        self.config.view.theme = value;
                    }
                    config::Event::Language(value) => {
                        lang::set(value);
                        self.config.view.language = value;
                    }
                    config::Event::CheckRelease(value) => {
                        self.config.release.check = value;
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
                    config::Event::ConfirmWhenDiscardingUnsavedPlaylist(value) => {
                        self.config.view.confirm_discard_playlist = value;
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
                let path = match path.parent_if_file() {
                    Ok(path) => path,
                    Err(_) => {
                        self.show_error(Error::UnableToOpenPath(path));
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
                            Message::OpenPathFailure { path: path2 }
                        }
                    }
                })
            }
            Message::OpenFile { path } => {
                let path2 = path.clone();
                Task::future(async move {
                    let result = async { opener::open(path.resolve()) }.await;

                    match result {
                        Ok(_) => Message::Ignore,
                        Err(e) => {
                            log::error!("Unable to open file: `{}` - {:?}", path2.resolve(), e);
                            Message::OpenPathFailure { path: path2 }
                        }
                    }
                })
            }
            Message::OpenPathFailure { path } => {
                self.show_modal(Modal::Error {
                    variant: Error::UnableToOpenPath(path),
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
                use iced::keyboard::{self, key, Key, Modifiers};

                match event {
                    keyboard::Event::KeyPressed { key, modifiers, .. } => match key {
                        Key::Named(key::Named::Tab) => {
                            if !self.modals.is_empty() {
                                if modifiers.shift() {
                                    iced::widget::focus_previous()
                                } else {
                                    iced::widget::focus_next()
                                }
                            } else {
                                self.selection.cycle(self.selectables(), modifiers.shift());
                                Task::none()
                            }
                        }
                        Key::Named(key::Named::Escape) => {
                            if !self.modals.is_empty() {
                                self.modals.pop();
                            } else if !self.dragged_files.is_empty() {
                                self.dragged_files.clear();
                            } else if self.selection.is_any_selected() {
                                self.selection.clear();
                            }
                            Task::none()
                        }
                        Key::Named(key::Named::Space) => {
                            if self.modals.is_empty() {
                                self.generate_event_in_selection(
                                    |app| Some(Message::SetPause(!app.config.playback.paused)),
                                    |grid_id, grid| {
                                        Some(PaneEvent::SetPause {
                                            grid_id,
                                            paused: !grid.all_paused().unwrap_or_default(),
                                        })
                                    },
                                    |player| Some(player::Event::SetPause(!player.is_paused().unwrap_or_default())),
                                )
                            } else {
                                Task::none()
                            }
                        }
                        Key::Named(key::Named::ArrowLeft) => {
                            if self.modals.is_empty() {
                                let step = Step::Earlier;
                                self.generate_event_in_selection(
                                    |_| Some(Message::Step(step)),
                                    |grid_id, _| Some(PaneEvent::Step { grid_id, step }),
                                    |_| Some(player::Event::Step(step)),
                                )
                            } else {
                                Task::none()
                            }
                        }
                        Key::Named(key::Named::ArrowRight) => {
                            if self.modals.is_empty() {
                                let step = Step::Later;
                                self.generate_event_in_selection(
                                    |_| Some(Message::Step(step)),
                                    |grid_id, _| Some(PaneEvent::Step { grid_id, step }),
                                    |_| Some(player::Event::Step(step)),
                                )
                            } else {
                                Task::none()
                            }
                        }
                        Key::Named(key::Named::Backspace | key::Named::Delete) => {
                            if self.modals.is_empty() {
                                self.generate_event_in_selection(
                                    |_| None,
                                    |grid_id, _| Some(PaneEvent::Close { grid_id }),
                                    |_| Some(player::Event::Close),
                                )
                            } else {
                                Task::none()
                            }
                        }
                        Key::Character(c) => {
                            let command = modifiers == Modifiers::COMMAND;
                            let command_shift = modifiers == Modifiers::COMMAND | Modifiers::SHIFT;

                            if self.modals.is_empty() {
                                match c.as_str() {
                                    "J" | "j" => self.generate_event_in_selection(
                                        |_| Some(Message::SeekRandom),
                                        |grid_id, _| Some(PaneEvent::SeekRandom { grid_id }),
                                        |_| Some(player::Event::SeekRandom),
                                    ),
                                    "L" | "l" => {
                                        self.update(Message::SetSynchronized(!self.config.playback.synchronized))
                                    }
                                    "M" | "m" => self.generate_event_in_selection(
                                        |app| Some(Message::SetMute(!app.config.playback.muted)),
                                        |grid_id, grid| {
                                            Some(PaneEvent::SetMute {
                                                grid_id,
                                                muted: !grid.all_muted().unwrap_or_default(),
                                            })
                                        },
                                        |player| Some(player::Event::SetMute(!player.is_muted().unwrap_or_default())),
                                    ),
                                    "N" | "n" if modifiers.is_empty() => {
                                        if let Some((grid_id, _)) = self.selection.pair() {
                                            self.update(Message::Pane {
                                                event: PaneEvent::AddPlayer { grid_id },
                                            })
                                        } else {
                                            Task::none()
                                        }
                                    }
                                    "N" | "n" if command => self.update(Message::PlaylistReset { force: false }),
                                    "O" | "o" if command => self.update(Message::PlaylistSelect { force: false }),
                                    "R" | "r" => self.generate_event_in_selection(
                                        |_| Some(Message::Refresh),
                                        |grid_id, _| Some(PaneEvent::Refresh { grid_id }),
                                        |_| Some(player::Event::Refresh),
                                    ),
                                    "S" | "s" if command => self.update(Message::PlaylistSave),
                                    "S" | "s" if command_shift => self.update(Message::PlaylistSaveAs),
                                    _ => Task::none(),
                                }
                            } else {
                                Task::none()
                            }
                        }
                        _ => Task::none(),
                    },
                    keyboard::Event::KeyReleased { .. } => Task::none(),
                    keyboard::Event::ModifiersChanged(modifiers) => {
                        self.modifiers = modifiers;
                        Task::none()
                    }
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
            Message::Refresh => {
                self.refresh(media::RefreshContext::Manual);
                Task::none()
            }
            Message::SetPause(flag) => {
                self.set_paused(flag);
                Task::none()
            }
            Message::SetMute(flag) => {
                self.set_muted(flag);
                Task::none()
            }
            Message::SetVolume { volume } => {
                self.set_volume(volume);
                Task::none()
            }
            Message::SetSynchronized(flag) => {
                self.set_synchronized(flag);
                Task::none()
            }
            Message::SeekRandom => {
                let event = if self.config.playback.synchronized {
                    player::Event::seek_random_relative()
                } else {
                    player::Event::SeekRandom
                };

                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(event.clone(), &mut self.media, &self.config.playback);
                }

                Task::none()
            }
            Message::Step(step) => {
                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::Step(step), &mut self.media, &self.config.playback);
                }
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
                    self.handle_grid_update(update, grid_id);
                }
                Task::none()
            }
            Message::Modal { event } => {
                if let Some(modal) = self.modals.last_mut() {
                    if let Some(update) = modal.update(event) {
                        match update {
                            modal::Update::SavedGridSettings { grid_id, settings } => {
                                let context = media::RefreshContext::Edit;
                                self.modals.pop();
                                let sources = settings.sources.clone();
                                if let Some(grid) = self.grids.get_mut(grid_id) {
                                    match grid.set_settings(settings) {
                                        Change::Same => {}
                                        Change::Different => {
                                            self.playlist_dirty = true;
                                        }
                                    }
                                }
                                self.refresh(context);
                                return Self::find_media(sources, context, self.playlist_path.clone());
                            }
                            modal::Update::PlayMedia { grid_id, media } => {
                                if let Some(grid) = self.grids.get_mut(grid_id) {
                                    grid.add_player_with_media(media, &mut self.media, &self.config.playback);
                                    self.playlist_dirty = true;
                                }
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
            Message::FindMedia => Self::find_media(
                self.all_sources(),
                media::RefreshContext::Automatic,
                self.playlist_path.clone(),
            ),
            Message::MediaScanned(scans) => {
                let mut tasks = vec![];
                for scan in scans {
                    match scan {
                        media::Scan::Found { source, media, context } => {
                            self.media.insert(source, media);
                            self.refresh(context);
                        }
                        scan => {
                            tasks.push(Self::find_media_one(scan));
                        }
                    }
                }
                Task::batch(tasks)
            }
            Message::FileDragDrop(path) => {
                if path.file_extension().is_some_and(|ext| ext == Playlist::EXTENSION) {
                    match self.modals.last() {
                        Some(_) => Task::none(),
                        None => {
                            if self.playlist_dirty && self.config.view.confirm_discard_playlist {
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
                                self.dragged_files.insert(path);
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

                let settings = grid
                    .settings()
                    .with_sources(self.dragged_files.drain().map(media::Source::new_path).collect());

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
                        let idle = self.grids.get(grid_id).is_some_and(|grid| grid.is_idle());
                        let settings = grid::Settings::default();
                        if let Some((grid_id, _split)) = self.grids.split(axis, grid_id, Grid::new(&settings)) {
                            self.playlist_dirty = true;
                            if !idle {
                                self.show_modal(Modal::new_grid_settings(grid_id, settings));
                            }
                        }
                    }
                    PaneEvent::Close { grid_id } => {
                        self.playlist_dirty = true;
                        self.grids.close(grid_id);
                        self.update_playback();
                        self.selection.clear();
                    }
                    PaneEvent::AddPlayer { grid_id } => {
                        let Some(grid) = self.grids.get_mut(grid_id) else {
                            return Task::none();
                        };

                        match grid.add_player(&mut self.media, &self.config.playback) {
                            Ok(_) => {
                                self.playlist_dirty = true;
                            }
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
                    PaneEvent::ShowMedia { grid_id } => {
                        if let Some(grid) = self.grids.get(grid_id) {
                            self.show_modal(Modal::GridMedia {
                                grid_id,
                                sources: grid.sources().to_vec(),
                            });
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

                            self.update_playback();
                        }
                    }
                    PaneEvent::SetPause { grid_id, paused } => {
                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(
                                player::Event::SetPause(paused),
                                &mut self.media,
                                &self.config.playback,
                            );

                            self.update_playback();
                        }
                    }
                    PaneEvent::SeekRandom { grid_id } => {
                        let event = if self.config.playback.synchronized {
                            player::Event::seek_random_relative()
                        } else {
                            player::Event::SeekRandom
                        };

                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(event.clone(), &mut self.media, &self.config.playback);

                            for category in grid.categories() {
                                self.synchronize_players(grid_id, category, event.clone());
                            }
                        }
                    }
                    PaneEvent::Step { grid_id, step } => {
                        let event = player::Event::Step(step);

                        if let Some(grid) = self.grids.get_mut(grid_id) {
                            grid.update_all_players(event.clone(), &mut self.media, &self.config.playback);

                            for category in grid.categories() {
                                self.synchronize_players(grid_id, category, event.clone());
                            }
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
                if self.playlist_dirty && !force && self.config.view.confirm_discard_playlist {
                    self.show_modal(Modal::ConfirmDiscardPlaylist { exit: false });
                    return Task::none();
                }

                self.close_modal();
                let (grids, _grid_id) = pane_grid::State::new(Grid::new(&grid::Settings::default()));
                self.grids = grids;
                self.playlist_dirty = false;
                self.playlist_path = None;
                self.media.clear();

                Task::none()
            }
            Message::PlaylistSelect { force } => {
                if self.playlist_dirty && !force && self.config.view.confirm_discard_playlist {
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
                self.modals.clear();

                match Playlist::load_from(&path) {
                    Ok(playlist) => {
                        self.playlist_dirty = false;
                        self.playlist_path = Some(path.clone());

                        let context = media::RefreshContext::Playlist;
                        self.grids = Self::load_playlist(playlist);
                        self.refresh(context);
                        Self::find_media(self.all_sources(), context, self.playlist_path.clone())
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
                        Self::find_media(
                            self.all_sources()
                                .into_iter()
                                .filter(|x| x.has_playlist_placeholder())
                                .collect(),
                            media::RefreshContext::Edit,
                            self.playlist_path.clone(),
                        )
                    }
                    Err(e) => {
                        self.show_error(e);
                        Task::none()
                    }
                }
            }
            Message::ShowMenu { show } => {
                self.viewing_menu = show.unwrap_or(!self.viewing_menu);
                Task::none()
            }
            Message::Menu { message } => {
                self.viewing_menu = false;
                self.update(*message)
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
            iced::time::every(Duration::from_secs(60 * 10)).map(|_| Message::FindMedia),
        ];

        #[cfg(feature = "audio")]
        subscriptions.push(iced::time::every(Duration::from_millis(1000)).map(|_| Message::CheckAudio));

        if !self.pending_save.is_empty() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::Save));
        }

        if self.config.release.check {
            subscriptions.push(iced::time::every(Duration::from_secs(60 * 60 * 24)).map(|_| Message::CheckAppRelease));
        }

        iced::Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element {
        let dragging_file = !self.dragged_files.is_empty();
        let obscured = !self.modals.is_empty();

        Responsive::new(move |viewport| {
            let left_controls = DropDown::new(
                button::icon(Icon::Menu)
                    .on_press(Message::ShowMenu { show: None })
                    .obscured(obscured),
                Container::new(
                    Column::new()
                        .push(
                            button::menu(Icon::FolderOpen, lang::action::open_playlist())
                                .on_press(Message::menu(Message::PlaylistSelect { force: false }))
                                .padding(4),
                        )
                        .push(
                            button::menu(Icon::Save, lang::action::save_playlist())
                                .on_press(Message::menu(Message::PlaylistSave))
                                .enabled(self.playlist_dirty && self.playlist_path.is_some())
                                .padding(4),
                        )
                        .push(
                            button::menu(Icon::SaveAs, lang::action::save_playlist_as_new_file())
                                .on_press(Message::menu(Message::PlaylistSaveAs))
                                .padding(4),
                        )
                        .push(
                            button::menu(Icon::PlaylistRemove, lang::action::start_new_playlist())
                                .on_press(Message::menu(Message::PlaylistReset { force: false }))
                                .enabled(self.playlist_dirty || self.playlist_path.is_some())
                                .padding(4),
                        )
                        .push_maybe(STEAM_DECK.then(|| {
                            button::menu(Icon::LogOut, lang::action::exit_app())
                                .on_press(Message::menu(Message::Exit { force: false }))
                                .padding(4)
                        }))
                        // .spacing(10)
                        .padding(4),
                )
                .class(style::Container::Tooltip),
                self.viewing_menu,
            )
            .on_dismiss(Message::ShowMenu { show: Some(false) });

            let right_controls = Row::new().push(
                button::icon(Icon::Settings)
                    .on_press(Message::ShowSettings)
                    .obscured(obscured)
                    .tooltip_below(lang::thing::settings()),
            );

            let center_controls = Container::new(
                Row::new()
                    .push(
                        button::icon(if self.config.playback.synchronized {
                            Icon::Link
                        } else {
                            Icon::Unlink
                        })
                        .on_press(Message::SetSynchronized(!self.config.playback.synchronized))
                        .obscured(obscured)
                        .tooltip_below(if self.config.playback.synchronized {
                            lang::action::desynchronize()
                        } else {
                            lang::action::synchronize()
                        }),
                    )
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
                            .on_press(Message::SeekRandom)
                            .enabled(!self.all_idle() && self.can_jump())
                            .obscured(obscured)
                            .tooltip_below(lang::action::jump_position()),
                    )
                    .push(
                        button::icon(Icon::Refresh)
                            .on_press(Message::Refresh)
                            .enabled(!self.all_idle())
                            .obscured(obscured)
                            .tooltip_below(lang::action::shuffle()),
                    ),
            )
            .class(style::Container::Player { selected: false });

            let controls = Stack::new()
                .push(Container::new(left_controls).align_left(Length::Fill))
                .push(Container::new(right_controls).align_right(Length::Fill))
                .push(Container::new(center_controls).center(Length::Fill));

            let grids = PaneGrid::new(&self.grids, |grid_id, grid, _maximized| {
                let selected = self.selection.is_grid_only_selected(grid_id);
                pane_grid::Content::new(
                    Container::new(grid.view(
                        grid_id,
                        selected,
                        self.selection.player_for_grid(grid_id),
                        obscured,
                        dragging_file,
                    ))
                    .padding(5)
                    .class(style::Container::PlayerGroup { selected }),
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
            });

            let content =
                Container::new(Column::new().spacing(5).push(controls).push(grids)).class(style::Container::Primary);

            let stack = Stack::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(content)
                .push_maybe(self.modals.last().map(|modal| {
                    modal.view(
                        viewport,
                        &self.config,
                        &self.text_histories,
                        &self.modifiers,
                        self.playlist_path.as_ref(),
                        &self.media,
                        modal
                            .grid_id()
                            .and_then(|grid_id| self.grids.get(grid_id).map(|grid| grid.active_media()))
                            .unwrap_or_default(),
                    )
                }));

            Container::new(stack)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(5.0)
                .into()
        })
        .into()
    }
}
