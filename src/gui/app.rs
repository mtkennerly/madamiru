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
        common::{Flags, Message, PaneEvent, UndoSubject},
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
    viewing_pane_controls: Option<grid::Id>,
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

        let text_histories = TextHistories::new(&config, &sources);

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

        let (mut grids, grid_id) = pane_grid::State::new(Grid::new(&sources));
        grids.split(pane_grid::Axis::Vertical, grid_id, Grid::new(&[]));

        if sources.is_empty() {
            modals.push(Modal::new_sources(grid_id, sources.clone()));
        } else {
            commands.push(Self::find_media(sources, media::RefreshContext::Launch))
        }

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
                viewing_pane_controls: None,
            },
            Task::batch(commands),
        )
    }

    pub fn title(&self) -> String {
        lang::window_title()
    }

    pub fn theme(&self) -> crate::gui::style::Theme {
        crate::gui::style::Theme::from(self.config.theme)
    }

    fn grid(&self, id: grid::Id) -> Option<&Grid> {
        self.grids.get(id)
    }

    fn grid_mut(&mut self, id: grid::Id) -> Option<&mut Grid> {
        self.grids.get_mut(id)
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

    fn all_paused(&self) -> bool {
        self.grids.iter().all(|(_id, grid)| grid.all_paused())
    }

    fn all_muted(&self) -> bool {
        self.grids.iter().all(|(_id, grid)| grid.all_muted())
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
        if sources.is_empty() {
            return Task::none();
        }

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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ignore => Task::none(),
            Message::Exit => {
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
                        if let Ok(value) = value.parse::<NonZeroUsize>() {
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
                            if let Ok(value) = self
                                .text_histories
                                .max_initial_media
                                .apply(shortcut)
                                .parse::<NonZeroUsize>()
                            {
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
                self.config.playback.paused = flag;

                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::SetPause(flag), &mut self.media, &self.config.playback);
                }

                Task::none()
            }
            Message::SetMute(flag) => {
                self.config.playback.muted = flag;
                self.save_config();

                for (_grid_id, grid) in self.grids.iter_mut() {
                    grid.update_all_players(player::Event::SetMute(flag), &mut self.media, &self.config.playback);
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
                    let Some(grid) = self.grid(grid_id) else {
                        return Task::none();
                    };

                    match update {
                        grid::Update::PauseChanged { .. } => {
                            self.config.playback.paused = self.all_paused();
                            self.save_config();
                        }
                        grid::Update::MuteChanged { .. } => {
                            self.config.playback.muted = self.all_muted();
                            self.save_config();
                        }
                        grid::Update::PlayerClosed => {
                            if grid.is_idle() {
                                self.show_modal(Modal::new_sources(grid_id, grid.sources().to_vec()));
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
                            modal::Update::SavedSources { grid_id, sources } => {
                                self.modals.pop();
                                if let Some(grid) = self.grid_mut(grid_id) {
                                    grid.set_sources(sources.clone());
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
            Message::FileDragDrop(path) => match self.modals.last_mut() {
                Some(Modal::Sources { sources, histories, .. }) => {
                    histories.push(TextHistory::path(&path));
                    sources.push(media::Source::new_path(path));
                    modal::scroll_down()
                }
                _ => {
                    // TODO: Update hovered grid.
                    let (grid_id, grid) = self.grids.iter().last().unwrap();

                    let mut sources = grid.sources().to_vec();
                    sources.push(media::Source::new_path(path));

                    self.show_modal(Modal::new_sources(*grid_id, sources));
                    modal::scroll_down()
                }
            },
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
                            self.dragging_pane = false;
                            self.grids.drop(pane, target);
                        }
                        pane_grid::DragEvent::Canceled { .. } => {
                            self.dragging_pane = false;
                        }
                    },
                    PaneEvent::Resize(event) => {
                        self.grids.resize(event.split, event.ratio);
                    }
                    PaneEvent::Split { grid_id, axis } => {
                        let sources = vec![];
                        if let Some((grid_id, _split)) = self.grids.split(axis, grid_id, Grid::new(&sources)) {
                            self.show_modal(Modal::new_sources(grid_id, sources));
                        }
                    }
                    PaneEvent::Close { grid_id } => {
                        self.grids.close(grid_id);
                    }
                    PaneEvent::AddPlayer { grid_id } => {
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
                    PaneEvent::ShowSources { grid_id } => {
                        let sources = self
                            .grid(grid_id)
                            .map(|grid| grid.sources().to_vec())
                            .unwrap_or_default();
                        self.show_modal(Modal::new_sources(grid_id, sources));
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
                }
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Exit),
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

        if !self.pending_save.is_empty() {
            subscriptions.push(iced::time::every(Duration::from_millis(200)).map(|_| Message::Save));
        }

        if self.config.release.check {
            subscriptions.push(iced::time::every(Duration::from_secs(60 * 60 * 24)).map(|_| Message::CheckAppRelease));
        }

        iced::Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element {
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
                                        button::icon(Icon::Settings)
                                            .on_press(Message::ShowSettings)
                                            .obscured(obscured)
                                            .tooltip_below(lang::thing::settings()),
                                    )
                                    .align_right(Length::Fill),
                                )
                                .push_maybe(STEAM_DECK.then(|| {
                                    Container::new(
                                        button::icon(Icon::LogOut)
                                            .on_press(Message::Exit)
                                            .obscured(obscured)
                                            .tooltip_below(lang::action::exit_app()),
                                    )
                                    .align_left(Length::Fill)
                                }))
                                .push(
                                    Container::new(
                                        Container::new(
                                            Row::new()
                                                .spacing(5)
                                                .push(
                                                    button::icon(if self.config.playback.muted {
                                                        Icon::Mute
                                                    } else {
                                                        Icon::VolumeHigh
                                                    })
                                                    .on_press(Message::SetMute(!self.config.playback.muted))
                                                    .enabled(!self.all_idle())
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
                                                    .enabled(!self.all_idle())
                                                    .obscured(obscured)
                                                    .tooltip_below(if self.config.playback.paused {
                                                        lang::action::play()
                                                    } else {
                                                        lang::action::pause()
                                                    }),
                                                )
                                                .push(
                                                    button::icon(Icon::Refresh)
                                                        .on_press(Message::Refresh)
                                                        .enabled(!self.all_idle())
                                                        .obscured(obscured)
                                                        .tooltip_below(lang::action::shuffle_media()),
                                                )
                                                .push(
                                                    button::icon(Icon::TimerRefresh)
                                                        .on_press(Message::AllPlayers {
                                                            event: player::Event::SeekRandom,
                                                        })
                                                        .enabled(!self.all_idle())
                                                        .obscured(obscured)
                                                        .tooltip_below(lang::action::jump_position()),
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
                                    Container::new(grid.view(grid_id, obscured))
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
