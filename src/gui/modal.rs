use std::{num::NonZeroUsize, sync::LazyLock};

use iced::{
    alignment,
    keyboard::Modifiers,
    padding,
    widget::{horizontal_rule, mouse_area, opaque, scrollable},
    Alignment, Length, Task,
};
use itertools::Itertools;

use crate::{
    gui::{
        button,
        common::{BrowseFileSubject, BrowseSubject, EditAction, Message, UndoSubject},
        grid,
        icon::Icon,
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{checkbox, pick_list, text, Column, Container, Element, Row, Scrollable, Space, Stack},
    },
    lang::{self, Language},
    media,
    path::StrictPath,
    prelude::Error,
    resource::{
        config::{self, Config, Theme},
        playlist,
    },
};

const RELEASE_URL: &str = "https://github.com/mtkennerly/madamiru/releases";
static SCROLLABLE: LazyLock<scrollable::Id> = LazyLock::new(scrollable::Id::unique);

pub fn scroll_down() -> Task<Message> {
    scrollable::scroll_by(
        (*SCROLLABLE).clone(),
        scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
    )
}

#[derive(Debug, Clone)]
pub enum Event {
    EditedSource { action: EditAction },
    EditedSourceKind { index: usize, kind: media::SourceKind },
    SelectedGridTab { tab: GridTab },
    EditedGridContentFit { content_fit: playlist::ContentFit },
    EditedGridOrientation { orientation: playlist::Orientation },
    EditedGridOrientationLimitKind { fixed: bool },
    EditedGridOrientationLimit { raw_limit: String },
    Save,
}

pub enum Update {
    SavedGridSettings {
        grid_id: grid::Id,
        settings: grid::Settings,
    },
    Task(Task<Message>),
}

pub enum ModalVariant {
    Info,
    Confirm,
    Editor,
}

#[derive(Debug, Clone)]
pub enum Modal {
    Settings,
    GridSettings {
        grid_id: grid::Id,
        tab: GridTab,
        settings: grid::Settings,
        histories: GridHistories,
    },
    Error {
        variant: Error,
    },
    Errors {
        errors: Vec<Error>,
    },
    AppUpdate {
        release: crate::metadata::Release,
    },
    ConfirmLoadPlaylist {
        path: Option<StrictPath>,
    },
    ConfirmDiscardPlaylist {
        exit: bool,
    },
}

impl Modal {
    pub fn new_grid_settings(grid_id: grid::Id, mut settings: grid::Settings) -> Self {
        let mut histories = GridHistories::default();

        if settings.sources.is_empty() {
            settings.sources.push(media::Source::default());
            histories.sources.push(TextHistory::default())
        } else {
            for source in &settings.sources {
                histories.sources.push(TextHistory::raw(source.raw()));
            }
        }

        let raw_limit = match settings.orientation_limit {
            playlist::OrientationLimit::Automatic => playlist::OrientationLimit::DEFAULT_FIXED.to_string(),
            playlist::OrientationLimit::Fixed(limit) => limit.to_string(),
        };
        histories.orientation_limit.push(&raw_limit);

        Self::GridSettings {
            grid_id,
            tab: GridTab::default(),
            settings,
            histories,
        }
    }

    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::Error { .. } | Self::Errors { .. } => ModalVariant::Info,
            Self::GridSettings { .. }
            | Self::AppUpdate { .. }
            | Self::ConfirmLoadPlaylist { .. }
            | Self::ConfirmDiscardPlaylist { .. } => ModalVariant::Confirm,
            Self::Settings => ModalVariant::Editor,
        }
    }

    pub fn title(&self, _config: &Config) -> Option<Element> {
        match self {
            Self::Settings => None,
            Self::GridSettings { tab, .. } => Some(
                Row::new()
                    .spacing(20)
                    .push(GridTab::Sources.view(*tab))
                    .push(GridTab::Layout.view(*tab))
                    .into(),
            ),
            Self::Error { .. } => None,
            Self::Errors { .. } => None,
            Self::AppUpdate { .. } => None,
            Self::ConfirmLoadPlaylist { .. } => None,
            Self::ConfirmDiscardPlaylist { .. } => None,
        }
    }

    pub fn message(&self) -> Option<Message> {
        match self {
            Self::Settings => Some(Message::CloseModal),
            Self::GridSettings { .. } => Some(Message::Modal { event: Event::Save }),
            Self::Error { .. } => Some(Message::CloseModal),
            Self::Errors { .. } => Some(Message::CloseModal),
            Self::AppUpdate { release } => Some(Message::OpenUrlAndCloseModal(release.url.clone())),
            Self::ConfirmLoadPlaylist { path } => match path {
                Some(path) => Some(Message::PlaylistLoad { path: path.clone() }),
                None => Some(Message::PlaylistSelect { force: true }),
            },
            Self::ConfirmDiscardPlaylist { exit } => {
                if *exit {
                    Some(Message::Exit { force: true })
                } else {
                    Some(Message::PlaylistReset { force: true })
                }
            }
        }
    }

    pub fn body(&self, config: &Config, histories: &TextHistories, modifiers: &Modifiers) -> Option<Column> {
        let mut col = Column::new().spacing(15).padding(padding::right(10));

        match self {
            Self::Settings => {
                col = col
                    .push(text(lang::field(&lang::thing::application())))
                    .push(
                        Container::new(
                            Column::new()
                                .spacing(10)
                                .padding(10)
                                // TODO: Enable language selector once we have translations.
                                .push_maybe(false.then(|| {
                                    Row::new()
                                        .align_y(Alignment::Center)
                                        .spacing(20)
                                        .push(text(lang::field(&lang::thing::language())))
                                        .push(pick_list(Language::ALL, Some(config.view.language), |value| {
                                            Message::Config {
                                                event: config::Event::Language(value),
                                            }
                                        }))
                                }))
                                .push(
                                    Row::new()
                                        .align_y(Alignment::Center)
                                        .spacing(20)
                                        .push(text(lang::field(&lang::thing::theme())))
                                        .push(pick_list(Theme::ALL, Some(config.view.theme), |value| {
                                            Message::Config {
                                                event: config::Event::Theme(value),
                                            }
                                        })),
                                )
                                .push(
                                    Row::new()
                                        .align_y(Alignment::Center)
                                        .spacing(20)
                                        .push(checkbox(
                                            lang::action::check_for_updates(),
                                            config.release.check,
                                            |value| Message::Config {
                                                event: config::Event::CheckRelease(value),
                                            },
                                        ))
                                        .push(
                                            button::icon(Icon::OpenInBrowser)
                                                .on_press(Message::OpenUrl(RELEASE_URL.to_string()))
                                                .tooltip(lang::action::view_releases())
                                                .padding([0, 10]),
                                        ),
                                )
                                .push(checkbox(
                                    lang::action::pause_when_window_loses_focus(),
                                    config.playback.pause_on_unfocus,
                                    |value| Message::Config {
                                        event: config::Event::PauseWhenWindowLosesFocus(value),
                                    },
                                ))
                                .push(checkbox(
                                    lang::action::confirm_when_discarding_unsaved_playlist(),
                                    config.view.confirm_discard_playlist,
                                    |value| Message::Config {
                                        event: config::Event::ConfirmWhenDiscardingUnsavedPlaylist(value),
                                    },
                                )),
                        )
                        .class(style::Container::Player),
                    )
                    .push(text(lang::field(&lang::thing::audio())))
                    .push(
                        Container::new(
                            Column::new().spacing(10).padding(10).push(
                                Row::new()
                                    .spacing(10)
                                    .align_y(alignment::Vertical::Center)
                                    .push(
                                        button::icon(if config.playback.muted {
                                            Icon::Mute
                                        } else {
                                            Icon::VolumeHigh
                                        })
                                        .on_press(Message::SetMute(!config.playback.muted))
                                        .tooltip(if config.playback.muted {
                                            lang::action::unmute()
                                        } else {
                                            lang::action::mute()
                                        }),
                                    )
                                    .push(
                                        iced::widget::slider(0.01..=1.0, config.playback.volume, |volume| {
                                            Message::SetVolume { volume }
                                        })
                                        .step(0.01)
                                        .width(150),
                                    )
                                    .push(
                                        text(format!("{:.0}%", config.playback.volume * 100.0))
                                            .width(50)
                                            .align_x(alignment::Horizontal::Center),
                                    ),
                            ),
                        )
                        .class(style::Container::Player),
                    )
                    .push(text(lang::field(&lang::thing::image())))
                    .push(
                        Container::new(
                            Column::new().spacing(10).padding(10).push(
                                Row::new()
                                    .align_y(Alignment::Center)
                                    .spacing(20)
                                    .push(text(lang::field(&lang::action::play_for_this_many_seconds())))
                                    .push(UndoSubject::ImageDuration.view_with(histories)),
                            ),
                        )
                        .class(style::Container::Player),
                    );
            }
            Self::GridSettings {
                tab: GridTab::Sources,
                settings,
                histories,
                ..
            } => {
                for (index, source) in settings.sources.iter().enumerate() {
                    col = col.push(
                        Row::new()
                            .spacing(20)
                            .align_y(alignment::Vertical::Center)
                            .push(
                                Row::new()
                                    .spacing(10)
                                    .align_y(alignment::Vertical::Center)
                                    .push(button::move_up(
                                        |action| Message::Modal {
                                            event: Event::EditedSource { action },
                                        },
                                        index,
                                    ))
                                    .push(button::move_down(
                                        |action| Message::Modal {
                                            event: Event::EditedSource { action },
                                        },
                                        index,
                                        settings.sources.len(),
                                    ))
                                    .push(pick_list(media::SourceKind::ALL, Some(source.kind()), move |kind| {
                                        Message::Modal {
                                            event: Event::EditedSourceKind { index, kind },
                                        }
                                    })),
                            )
                            .push(UndoSubject::Source { index }.view(&histories.sources[index].current()))
                            .push(match source {
                                media::Source::Path { path } => Row::new()
                                    .spacing(10)
                                    .align_y(alignment::Vertical::Center)
                                    .push(button::choose_folder(
                                        BrowseSubject::Source { index },
                                        path.clone(),
                                        modifiers,
                                    ))
                                    .push(button::choose_file(
                                        BrowseFileSubject::Source { index },
                                        path.clone(),
                                        modifiers,
                                    ))
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Modal {
                                                event: Event::EditedSource {
                                                    action: EditAction::Remove(index),
                                                },
                                            })
                                            .enabled(settings.sources.len() > 1),
                                    ),
                                media::Source::Glob { .. } => {
                                    Row::new().spacing(10).align_y(alignment::Vertical::Center).push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Modal {
                                                event: Event::EditedSource {
                                                    action: EditAction::Remove(index),
                                                },
                                            })
                                            .enabled(settings.sources.len() > 1),
                                    )
                                }
                            }),
                    );
                }

                col = col.push(button::icon(Icon::Add).on_press(Message::Modal {
                    event: Event::EditedSource {
                        action: EditAction::Add,
                    },
                }));
            }
            Self::GridSettings {
                tab: GridTab::Layout,
                settings,
                histories,
                ..
            } => {
                col = col
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(text(lang::field(&lang::thing::orientation())))
                            .push(pick_list(
                                playlist::Orientation::ALL,
                                Some(settings.orientation),
                                |orientation| Message::Modal {
                                    event: Event::EditedGridOrientation { orientation },
                                },
                            )),
                    )
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(checkbox(
                                lang::field(&lang::thing::items_per_line()),
                                settings.orientation_limit.is_fixed(),
                                |fixed| Message::Modal {
                                    event: Event::EditedGridOrientationLimitKind { fixed },
                                },
                            ))
                            .push(UndoSubject::OrientationLimit.view(&histories.orientation_limit.current())),
                    )
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(text(lang::field(&lang::thing::content_fit())))
                            .push(pick_list(
                                playlist::ContentFit::ALL,
                                Some(settings.content_fit),
                                |content_fit| Message::Modal {
                                    event: Event::EditedGridContentFit { content_fit },
                                },
                            )),
                    );
            }
            Self::Error { variant } => {
                col = col.push(text(lang::handle_error(variant)));
            }
            Self::Errors { errors } => {
                col = col.push(text(errors.iter().map(lang::handle_error).join("\n\n")));
            }
            Self::AppUpdate { release } => {
                col = col
                    .push(text(lang::tell::new_version_available(
                        release.version.to_string().as_str(),
                    )))
                    .push(text(lang::ask::view_release_notes()));
            }
            Self::ConfirmLoadPlaylist { .. } => {
                col = col.push(text(lang::join!(
                    lang::tell::playlist_has_unsaved_changes(),
                    lang::ask::load_new_playlist_anyway()
                )));
            }
            Self::ConfirmDiscardPlaylist { .. } => {
                col = col.push(text(lang::join!(
                    lang::tell::playlist_has_unsaved_changes(),
                    lang::ask::discard_changes()
                )));
            }
        }

        Some(col)
    }

    pub fn controls(&self) -> Element {
        let positive_button = button::primary(match self.variant() {
            ModalVariant::Info => lang::action::close(),
            ModalVariant::Confirm => lang::action::confirm(),
            ModalVariant::Editor => lang::action::close(),
        })
        .on_press_maybe(self.message());

        let negative_button = button::negative(lang::action::cancel()).on_press(Message::CloseModal);

        let row = match self.variant() {
            ModalVariant::Info | ModalVariant::Editor => Row::new().push(positive_button),
            ModalVariant::Confirm => Row::new().push(positive_button).push(negative_button),
        };

        row.spacing(20).padding([0, 30]).into()
    }

    fn content(
        &self,
        viewport: iced::Size,
        config: &Config,
        histories: &TextHistories,
        modifiers: &Modifiers,
    ) -> Container {
        Container::new(
            Column::new()
                .spacing(30)
                .padding(padding::top(30).bottom(30))
                .align_x(Alignment::Center)
                .push_maybe(self.title(config))
                .push_maybe(self.body(config, histories, modifiers).map(|body| {
                    Container::new(Scrollable::new(body.padding([0, 30])).id((*SCROLLABLE).clone()))
                        .padding(padding::right(5))
                        .max_height(viewport.height - 300.0)
                }))
                .push(Container::new(self.controls())),
        )
        .class(style::Container::ModalForeground)
    }

    pub fn apply_shortcut(&mut self, subject: UndoSubject, shortcut: Shortcut) -> bool {
        match self {
            Self::Settings
            | Self::Error { .. }
            | Self::Errors { .. }
            | Self::AppUpdate { .. }
            | Self::ConfirmLoadPlaylist { .. }
            | Self::ConfirmDiscardPlaylist { .. } => false,
            Self::GridSettings {
                settings, histories, ..
            } => match subject {
                UndoSubject::ImageDuration => false,
                UndoSubject::Source { index } => {
                    settings.sources[index].reset(histories.sources[index].apply(shortcut));
                    true
                }
                UndoSubject::OrientationLimit => {
                    if let Ok(value) = histories.orientation_limit.apply(shortcut).parse::<NonZeroUsize>() {
                        settings.orientation_limit = playlist::OrientationLimit::Fixed(value);
                    }
                    true
                }
            },
        }
    }

    #[must_use]
    pub fn update(&mut self, event: Event) -> Option<Update> {
        match self {
            Self::Settings
            | Self::Error { .. }
            | Self::Errors { .. }
            | Self::AppUpdate { .. }
            | Self::ConfirmLoadPlaylist { .. }
            | Self::ConfirmDiscardPlaylist { .. } => None,
            Self::GridSettings {
                grid_id,
                tab,
                settings,
                histories,
            } => match event {
                Event::EditedSource { action } => {
                    match action {
                        EditAction::Add => {
                            let value = StrictPath::default();
                            histories.sources.push(TextHistory::path(&value));
                            settings.sources.push(media::Source::new_path(value));
                            return Some(Update::Task(scroll_down()));
                        }
                        EditAction::Change(index, value) => {
                            histories.sources[index].push(&value);
                            settings.sources[index].reset(value);
                        }
                        EditAction::Remove(index) => {
                            histories.sources.remove(index);
                            settings.sources.remove(index);
                        }
                        EditAction::Move(index, direction) => {
                            let offset = direction.shift(index);
                            histories.sources.swap(index, offset);
                            settings.sources.swap(index, offset);
                        }
                    }
                    None
                }
                Event::EditedSourceKind { index, kind } => {
                    settings.sources[index].set_kind(kind);
                    None
                }
                Event::SelectedGridTab { tab: new_tab } => {
                    *tab = new_tab;
                    None
                }
                Event::EditedGridContentFit { content_fit } => {
                    settings.content_fit = content_fit;
                    None
                }
                Event::EditedGridOrientation { orientation } => {
                    settings.orientation = orientation;
                    None
                }
                Event::EditedGridOrientationLimitKind { fixed } => {
                    if fixed {
                        let limit = histories
                            .orientation_limit
                            .current()
                            .parse::<NonZeroUsize>()
                            .unwrap_or(playlist::OrientationLimit::default_fixed());
                        settings.orientation_limit = playlist::OrientationLimit::Fixed(limit);
                    } else {
                        settings.orientation_limit = playlist::OrientationLimit::Automatic;
                    }
                    None
                }
                Event::EditedGridOrientationLimit { raw_limit } => {
                    histories.orientation_limit.push(&raw_limit);
                    if settings.orientation_limit.is_fixed() {
                        if let Ok(limit) = raw_limit.parse::<NonZeroUsize>() {
                            settings.orientation_limit = playlist::OrientationLimit::Fixed(limit);
                        }
                    }
                    None
                }
                Event::Save => {
                    for index in (0..settings.sources.len()).rev() {
                        if settings.sources[index].is_empty() {
                            settings.sources.remove(index);
                        }
                    }

                    Some(Update::SavedGridSettings {
                        grid_id: *grid_id,
                        settings: settings.clone(),
                    })
                }
            },
        }
    }

    pub fn view(
        &self,
        viewport: iced::Size,
        config: &Config,
        histories: &TextHistories,
        modifiers: &Modifiers,
    ) -> Element {
        Stack::new()
            .push({
                let mut area = mouse_area(
                    Container::new(Space::new(Length::Fill, Length::Fill)).class(style::Container::ModalBackground),
                );

                match self.variant() {
                    ModalVariant::Info | ModalVariant::Confirm | ModalVariant::Editor => {
                        area = area.on_press(Message::CloseModal);
                    }
                }

                area
            })
            .push(
                Container::new(opaque(self.content(viewport, config, histories, modifiers)))
                    .center(Length::Fill)
                    .padding([0.0, (100.0 + viewport.width - 640.0).clamp(0.0, 100.0)]),
            )
            .into()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GridTab {
    #[default]
    Sources,
    Layout,
}

impl GridTab {
    fn view(&self, selected: Self) -> Element {
        let label = match self {
            GridTab::Sources => lang::thing::sources(),
            GridTab::Layout => lang::thing::layout(),
        };

        Column::new()
            .width(80)
            .spacing(2)
            .align_x(alignment::Horizontal::Center)
            .push(button::bare(label).on_press(Message::Modal {
                event: Event::SelectedGridTab { tab: *self },
            }))
            .push_maybe((*self == selected).then_some(horizontal_rule(2)))
            .into()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GridHistories {
    pub sources: Vec<TextHistory>,
    pub orientation_limit: TextHistory,
}
