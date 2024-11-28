use std::{
    collections::HashMap,
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
        shortcuts::{Shortcut, TextHistories},
        style,
        widget::{Column, Container, Element, Responsive, Row, Stack},
    },
    lang,
    prelude::{Error, STEAM_DECK},
    resource::{cache::Cache, config::Config, ResourceFile, SaveableResourceFile},
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
    last_tick: Instant,
}

impl App {
    fn show_modal(&mut self, modal: Modal) -> Task<Message> {
        self.modals.push(modal);
        Task::none()
    }

    fn close_modal(&mut self) -> Task<Message> {
        self.modals.pop();
        Task::none()
    }

    fn show_error(&mut self, error: Error) -> Task<Message> {
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

        let mut modal = vec![];
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

        if !errors.is_empty() {
            modal.push(Modal::Errors { errors });
        }

        let sources = flags.sources.clone();
        if let Some(max) = flags.max {
            config.playback.max = max;
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

        let grid = Grid::new(&sources, &config.playback);

        if grid.is_idle() && modal.is_empty() {
            modal.push(Modal::Sources {
                sources: sources.clone(),
                histories: text_histories.clone(),
            });
        }

        (
            Self {
                config,
                cache,
                modals: modal,
                text_histories,
                pending_save: Default::default(),
                modifiers: Default::default(),
                grid,
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
        self.grid.refresh(&self.config.playback);
        Task::none()
    }

    fn all_paused(&self) -> bool {
        self.grid.all_paused()
    }

    fn all_muted(&self) -> bool {
        self.grid.all_muted()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ignore => Task::none(),
            Message::Exit => {
                // If we don't pause first, you may still hear the videos for a moment after the app closes.
                self.grid
                    .update_all_players(player::Event::SetPause(true), &self.config.playback);
                std::process::exit(0)
            }
            Message::Tick(instant) => {
                let elapsed = instant - self.last_tick;
                self.last_tick = instant;
                self.grid.tick(elapsed, &self.config.playback);
                Task::none()
            }
            Message::Save => {
                self.save();
                Task::none()
            }
            Message::CloseModal => self.close_modal(),
            Message::AppReleaseToggle(enabled) => {
                self.config.release.check = enabled;
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
                                return self.show_modal(Modal::AppUpdate { release });
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
            Message::SelectedFile(subject, path) => {
                match subject {
                    BrowseFileSubject::Source { index } => {
                        self.text_histories.sources[index].push(&path.raw());
                    }
                }
                self.save_config();
                Task::none()
            }
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
                    BrowseSubject::Source { index } => self.grid.sources()[index].clone(),
                };

                match path.parent_if_file() {
                    Ok(path) => self.update(Message::OpenDir { path }),
                    Err(_) => self.show_error(Error::UnableToOpenDir(path)),
                }
            }
            Message::OpenFile { path } => {
                let path = match path.parent_if_file() {
                    Ok(path) => path,
                    Err(_) => return self.show_error(Error::UnableToOpenDir(path)),
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
                    BrowseFileSubject::Source { index } => self.grid.sources()[index].clone(),
                };

                self.update(Message::OpenFile { path })
            }
            Message::OpenDirFailure { path } => self.show_modal(Modal::Error {
                variant: Error::UnableToOpenDir(path),
            }),
            Message::OpenUrlFailure { url } => self.show_modal(Modal::Error {
                variant: Error::UnableToOpenUrl(url),
            }),
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
                match self.modals.last_mut() {
                    Some(modal) => modal.apply_shortcut(subject, shortcut),
                    None => {
                        match subject {
                            UndoSubject::Source { .. } => {}
                        }
                        self.save_config();
                    }
                }
                Task::none()
            }
            Message::SelectedLanguage(language) => {
                lang::set(language);
                self.config.language = language;
                self.save_config();
                Task::none()
            }
            Message::SelectedTheme(theme) => {
                self.config.theme = theme;
                self.save_config();
                Task::none()
            }
            Message::OpenUrl(url) => Self::open_url(url),
            Message::OpenUrlAndCloseModal(url) => Task::batch([Self::open_url(url), self.close_modal()]),
            Message::Refresh => self.refresh(),
            Message::AddPlayer => match self.grid.add_player(&self.config.playback) {
                Ok(_) => Task::none(),
                Err(e) => match e {
                    grid::Error::NoMediaAvailable => self.show_modal(Modal::Error {
                        variant: Error::NoMediaFound,
                    }),
                },
            },
            Message::SetPause(flag) => {
                self.config.playback.paused = flag;

                self.grid
                    .update_all_players(player::Event::SetPause(flag), &self.config.playback);

                Task::none()
            }
            Message::SetMute(flag) => {
                self.config.playback.muted = flag;
                self.save_config();

                self.grid
                    .update_all_players(player::Event::SetMute(flag), &self.config.playback);

                Task::none()
            }
            Message::Player { pane, event } => {
                if let Some(update) = self
                    .grid
                    .update(grid::Event::Player { id: pane, event }, &self.config.playback)
                {
                    match update {
                        grid::Update::PauseChanged { .. } => {
                            self.config.playback.paused = self.all_paused();
                        }
                        grid::Update::MuteChanged { .. } => {
                            self.config.playback.muted = self.all_muted();
                            self.save_config();
                        }
                        grid::Update::PlayerClosed => {
                            if self.grid.is_idle() {
                                return self.show_modal(Modal::Sources {
                                    sources: self.grid.sources().to_vec(),
                                    histories: self.text_histories.clone(),
                                });
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::AllPlayers { event } => {
                self.grid.update_all_players(event, &self.config.playback);
                Task::none()
            }
            Message::Modal { event } => {
                if let Some(modal) = self.modals.last_mut() {
                    if let Some(update) = modal.update(event) {
                        match update {
                            modal::Update::SavedSources { sources, histories } => {
                                self.modals.pop();
                                self.text_histories = histories;
                                self.grid.set_sources(sources, &self.config.playback);
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ShowSettings => self.show_modal(Modal::Settings),
            Message::ShowSources => self.show_modal(Modal::Sources {
                sources: self.grid.sources().to_vec(),
                histories: self.text_histories.clone(),
            }),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            iced::event::listen_with(|event, _status, _window| match event {
                iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
                iced::Event::Window(iced::window::Event::CloseRequested) => Some(Message::Exit),
                _ => None,
            }),
            iced::time::every(Duration::from_millis(100)).map(Message::Tick),
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
        Responsive::new(|viewport| {
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
                                    .tooltip_below(lang::action::configure_media_sources()),
                            )
                            .push(horizontal_space())
                            .push_maybe((self.grid.is_idle() && !self.grid.sources().is_empty()).then(|| {
                                Container::new(
                                    Row::new().spacing(5).push(
                                        button::icon(Icon::Add)
                                            .on_press(Message::AddPlayer)
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
                                                .tooltip_below(lang::action::add_player()),
                                        )
                                        .push(
                                            button::icon(if self.config.playback.muted {
                                                Icon::Mute
                                            } else {
                                                Icon::VolumeHigh
                                            })
                                            .on_press(Message::SetMute(!self.config.playback.muted))
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
                                                .tooltip_below(lang::action::shuffle_media()),
                                        )
                                        .push(
                                            button::icon(Icon::TimerRefresh)
                                                .on_press(Message::AllPlayers {
                                                    event: player::Event::SeekRandom,
                                                })
                                                .tooltip_below(lang::action::jump_position()),
                                        ),
                                )
                                .class(style::Container::Player)
                            }))
                            .push(horizontal_space())
                            .push(
                                button::icon(Icon::Settings)
                                    .on_press(Message::ShowSettings)
                                    .tooltip_below(lang::thing::settings()),
                            )
                            .push_maybe(STEAM_DECK.then(|| {
                                button::icon(Icon::LogOut)
                                    .on_press(Message::Exit)
                                    .tooltip_below(lang::action::exit_app())
                            })),
                    )
                    .push(self.grid.view(!self.modals.is_empty())),
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
