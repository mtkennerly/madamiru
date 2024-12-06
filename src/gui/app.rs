use std::{
    collections::HashMap,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use iced::{alignment, keyboard, widget::horizontal_space, Length, Subscription, Task};

use crate::{
    gui::{
        button,
        common::{BrowseFileSubject, BrowseSubject, Flags, Message, UndoSubject},
        grid::{self, Grid},
        icon::Icon,
        modal::{self, Modal},
        player::{self},
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{Column, Container, Element, Responsive, Row, Stack},
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
    grid: Grid,
    media: media::Collection,
    last_tick: Instant,
}

impl App {
    fn show_modal(&mut self, modal: Modal) {
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

        let grid = Grid::new(&sources);

        if sources.is_empty() {
            modals.push(Modal::new_sources(sources.clone(), text_histories.clone()));
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
                grid,
                media: Default::default(),
                last_tick: Instant::now(),
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

    fn refresh(&mut self) -> Task<Message> {
        self.grid.refresh(&mut self.media, &self.config.playback);
        Task::none()
    }

    fn all_paused(&self) -> bool {
        self.grid.all_paused()
    }

    fn all_muted(&self) -> bool {
        self.grid.all_muted()
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
                self.grid
                    .update_all_players(player::Event::SetPause(true), &mut self.media, &self.config.playback);
                std::process::exit(0)
            }
            Message::Tick(instant) => {
                let elapsed = instant - self.last_tick;
                self.last_tick = instant;
                self.grid.tick(elapsed, &mut self.media, &self.config.playback);
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
            Message::OpenDirSubject(subject) => {
                let path = match subject {
                    BrowseSubject::Source { index } => self.grid.sources()[index].path().cloned(),
                };

                let Some(path) = path else {
                    return Task::none();
                };

                match path.parent_if_file() {
                    Ok(path) => self.update(Message::OpenDir { path }),
                    Err(_) => {
                        self.show_error(Error::UnableToOpenDir(path));
                        Task::none()
                    }
                }
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
            Message::OpenFileSubject(subject) => {
                let path = match subject {
                    BrowseFileSubject::Source { index } => self.grid.sources()[index].path().cloned(),
                };

                let Some(path) = path else {
                    return Task::none();
                };

                self.update(Message::OpenFile { path })
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
            Message::AddPlayer => match self.grid.add_player(&mut self.media, &self.config.playback) {
                Ok(_) => Task::none(),
                Err(e) => match e {
                    grid::Error::NoMediaAvailable => {
                        self.show_modal(Modal::Error {
                            variant: Error::NoMediaFound,
                        });
                        Task::none()
                    }
                },
            },
            Message::SetPause(flag) => {
                self.config.playback.paused = flag;

                self.grid
                    .update_all_players(player::Event::SetPause(flag), &mut self.media, &self.config.playback);

                Task::none()
            }
            Message::SetMute(flag) => {
                self.config.playback.muted = flag;
                self.save_config();

                self.grid
                    .update_all_players(player::Event::SetMute(flag), &mut self.media, &self.config.playback);

                Task::none()
            }
            Message::Player { pane, event } => {
                if let Some(update) = self.grid.update(
                    grid::Event::Player { id: pane, event },
                    &mut self.media,
                    &self.config.playback,
                ) {
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
                            if self.grid.is_idle() {
                                self.show_modal(Modal::new_sources(
                                    self.grid.sources().to_vec(),
                                    self.text_histories.clone(),
                                ));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::AllPlayers { event } => {
                self.grid
                    .update_all_players(event, &mut self.media, &self.config.playback);
                Task::none()
            }
            Message::Modal { event } => {
                if let Some(modal) = self.modals.last_mut() {
                    if let Some(update) = modal.update(event) {
                        match update {
                            modal::Update::SavedSources { sources, histories } => {
                                self.modals.pop();
                                self.text_histories = histories;
                                self.grid.set_sources(sources.clone());
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
            Message::ShowSources => {
                self.show_modal(Modal::new_sources(
                    self.grid.sources().to_vec(),
                    self.text_histories.clone(),
                ));
                Task::none()
            }
            Message::FindMedia => Self::find_media(self.grid.sources().to_vec(), media::RefreshContext::Automatic),
            Message::MediaFound { context, media } => {
                self.media.replace(media);
                self.grid
                    .refresh_on_media_collection_changed(context, &mut self.media, &self.config.playback);
                Task::none()
            }
            Message::FileDragDrop(path) => match self.modals.last_mut() {
                Some(Modal::Sources { sources, histories }) => {
                    histories.sources.push(TextHistory::path(&path));
                    sources.push(media::Source::new_path(path));
                    modal::scroll_down()
                }
                _ => {
                    let mut sources = self.grid.sources().to_vec();
                    let mut histories = self.text_histories.clone();

                    histories.sources.push(TextHistory::path(&path));
                    sources.push(media::Source::new_path(path));

                    self.show_modal(Modal::new_sources(sources, histories));
                    modal::scroll_down()
                }
            },
            Message::WindowFocused => {
                self.grid
                    .update_all_players(player::Event::WindowFocused, &mut self.media, &self.config.playback);
                Task::none()
            }
            Message::WindowUnfocused => {
                self.grid
                    .update_all_players(player::Event::WindowUnfocused, &mut self.media, &self.config.playback);
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
            let content = Container::new(
                Column::new()
                    .spacing(5)
                    .push(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .push(
                                button::icon(Icon::PlaylistAdd)
                                    .on_press(Message::ShowSources)
                                    .obscured(obscured)
                                    .tooltip_below(lang::action::configure_media_sources()),
                            )
                            .push(horizontal_space())
                            .push_maybe((self.grid.is_idle() && !self.grid.sources().is_empty()).then(|| {
                                Container::new(
                                    Row::new().spacing(5).push(
                                        button::icon(Icon::Add)
                                            .on_press(Message::AddPlayer)
                                            .obscured(obscured)
                                            .tooltip_below(lang::action::add_player()),
                                    ),
                                )
                                .class(style::Container::Player)
                            }))
                            .push_maybe((!self.grid.is_idle()).then(|| {
                                Container::new(
                                    Row::new()
                                        .spacing(5)
                                        .push(
                                            button::icon(Icon::Add)
                                                .on_press(Message::AddPlayer)
                                                .obscured(obscured)
                                                .tooltip_below(lang::action::add_player()),
                                        )
                                        .push(
                                            button::icon(if self.config.playback.muted {
                                                Icon::Mute
                                            } else {
                                                Icon::VolumeHigh
                                            })
                                            .on_press(Message::SetMute(!self.config.playback.muted))
                                            .obscured(obscured)
                                            .tooltip_below(
                                                if self.config.playback.muted {
                                                    lang::action::unmute()
                                                } else {
                                                    lang::action::mute()
                                                },
                                            ),
                                        )
                                        .push(
                                            button::icon(if self.config.playback.paused {
                                                Icon::Play
                                            } else {
                                                Icon::Pause
                                            })
                                            .on_press(Message::SetPause(!self.config.playback.paused))
                                            .obscured(obscured)
                                            .tooltip_below(
                                                if self.config.playback.paused {
                                                    lang::action::play()
                                                } else {
                                                    lang::action::pause()
                                                },
                                            ),
                                        )
                                        .push(
                                            button::icon(Icon::Refresh)
                                                .on_press(Message::Refresh)
                                                .obscured(obscured)
                                                .tooltip_below(lang::action::shuffle_media()),
                                        )
                                        .push(
                                            button::icon(Icon::TimerRefresh)
                                                .on_press(Message::AllPlayers {
                                                    event: player::Event::SeekRandom,
                                                })
                                                .obscured(obscured)
                                                .tooltip_below(lang::action::jump_position()),
                                        ),
                                )
                                .class(style::Container::Player)
                            }))
                            .push(horizontal_space())
                            .push(
                                button::icon(Icon::Settings)
                                    .on_press(Message::ShowSettings)
                                    .obscured(obscured)
                                    .tooltip_below(lang::thing::settings()),
                            )
                            .push_maybe(STEAM_DECK.then(|| {
                                button::icon(Icon::LogOut)
                                    .on_press(Message::Exit)
                                    .obscured(obscured)
                                    .tooltip_below(lang::action::exit_app())
                            })),
                    )
                    .push(self.grid.view(obscured)),
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
