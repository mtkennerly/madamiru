use std::sync::LazyLock;

use iced::{
    alignment,
    keyboard::Modifiers,
    padding,
    widget::{mouse_area, opaque, scrollable},
    Alignment, Length, Task,
};
use itertools::Itertools;

use crate::{
    gui::{
        button,
        common::{BrowseFileSubject, BrowseSubject, EditAction, Message, UndoSubject},
        icon::Icon,
        shortcuts::{Shortcut, TextHistories, TextHistory},
        style,
        widget::{checkbox, pick_list, text, Column, Container, Element, Row, Scrollable, Space, Stack},
    },
    lang::{self, Language},
    media::Source,
    path::StrictPath,
    prelude::Error,
    resource::config::{Config, Theme},
};

const RELEASE_URL: &str = "https://github.com/mtkennerly/madamiru/releases";
static SCROLLABLE: LazyLock<scrollable::Id> = LazyLock::new(scrollable::Id::unique);

#[derive(Debug, Clone)]
pub enum Event {
    EditedSource { action: EditAction },
    Save,
}

pub enum Update {
    SavedSources {
        sources: Vec<Source>,
        histories: TextHistories,
    },
    Task(Task<Message>),
}

pub enum ModalVariant {
    Info,
    Confirm,
    Editor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    Settings,
    Sources {
        sources: Vec<Source>,
        histories: TextHistories,
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
}

impl Modal {
    pub fn new_sources(mut sources: Vec<Source>, mut histories: TextHistories) -> Self {
        if sources.is_empty() {
            sources.push(Source::default());
            histories.sources.push(TextHistory::default())
        }

        Self::Sources { sources, histories }
    }

    pub fn variant(&self) -> ModalVariant {
        match self {
            Self::Error { .. } | Self::Errors { .. } => ModalVariant::Info,
            Self::Sources { .. } | Self::AppUpdate { .. } => ModalVariant::Confirm,
            Self::Settings => ModalVariant::Editor,
        }
    }

    pub fn title(&self, _config: &Config) -> Option<String> {
        match self {
            Self::Settings => None,
            Self::Sources { .. } => Some(lang::action::configure_media_sources()),
            Self::Error { .. } => None,
            Self::Errors { .. } => None,
            Self::AppUpdate { .. } => None,
        }
    }

    pub fn message(&self, _histories: &TextHistories) -> Option<Message> {
        match self {
            Self::Settings => Some(Message::CloseModal),
            Self::Sources { .. } => Some(Message::Modal { event: Event::Save }),
            Self::Error { .. } => Some(Message::CloseModal),
            Self::Errors { .. } => Some(Message::CloseModal),
            Self::AppUpdate { release } => Some(Message::OpenUrlAndCloseModal(release.url.clone())),
        }
    }

    pub fn body(&self, config: &Config, histories: &TextHistories, modifiers: &Modifiers) -> Option<Column> {
        let mut col = Column::new().spacing(15).padding(padding::right(10));

        match self {
            Self::Settings => {
                col = col
                    // TODO: Enable language selector once we have translations.
                    .push_maybe(false.then(|| {
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(text(lang::field(&lang::thing::language())))
                            .push(pick_list(
                                Language::ALL,
                                Some(config.language),
                                Message::SelectedLanguage,
                            ))
                    }))
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(text(lang::field(&lang::thing::theme())))
                            .push(pick_list(Theme::ALL, Some(config.theme), Message::SelectedTheme)),
                    )
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .spacing(20)
                            .push(checkbox(
                                lang::action::check_for_updates(),
                                config.release.check,
                                Message::AppReleaseToggle,
                            ))
                            .push(
                                button::icon(Icon::OpenInBrowser)
                                    .on_press(Message::OpenUrl(RELEASE_URL.to_string()))
                                    .tooltip(lang::action::view_releases()),
                            ),
                    );
            }
            Self::Sources { sources, .. } => {
                for (index, source) in sources.iter().enumerate() {
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
                                        sources.len(),
                                    )),
                            )
                            .push(histories.input(UndoSubject::Source { index }))
                            .push(
                                Row::new()
                                    .spacing(10)
                                    .align_y(alignment::Vertical::Center)
                                    .push(button::choose_folder(
                                        BrowseSubject::Source { index },
                                        source.path.clone(),
                                        modifiers,
                                    ))
                                    .push(button::choose_file(
                                        BrowseFileSubject::Source { index },
                                        source.path.clone(),
                                        modifiers,
                                    ))
                                    .push(button::icon(Icon::Close).on_press_maybe((sources.len() > 1).then_some(
                                        Message::Modal {
                                            event: Event::EditedSource {
                                                action: EditAction::Remove(index),
                                            },
                                        },
                                    ))),
                            ),
                    );
                }

                col = col.push(button::icon(Icon::Add).on_press(Message::Modal {
                    event: Event::EditedSource {
                        action: EditAction::Add,
                    },
                }));
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
        }

        Some(col)
    }

    pub fn controls(&self, histories: &TextHistories) -> Element {
        let positive_button = button::primary(match self.variant() {
            ModalVariant::Info => lang::action::close(),
            ModalVariant::Confirm => lang::action::confirm(),
            ModalVariant::Editor => lang::action::close(),
        })
        .on_press_maybe(self.message(histories));

        let negative_button = button::negative(lang::action::cancel()).on_press(Message::CloseModal);

        let row = match self.variant() {
            ModalVariant::Info | ModalVariant::Editor => Row::new().push(positive_button),
            ModalVariant::Confirm => Row::new().push(positive_button).push(negative_button),
        };

        row.spacing(20).into()
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
                .push_maybe(self.title(config).map(text))
                .push_maybe(self.body(config, histories, modifiers).map(|body| {
                    Container::new(Scrollable::new(body.padding([0, 30])).id((*SCROLLABLE).clone()))
                        .padding(padding::right(5))
                        .max_height(viewport.height - 300.0)
                }))
                .push(Container::new(self.controls(histories))),
        )
        .class(style::Container::ModalForeground)
    }

    pub fn apply_shortcut(&mut self, subject: UndoSubject, shortcut: Shortcut) {
        match self {
            Modal::Settings | Modal::Error { .. } | Modal::Errors { .. } | Modal::AppUpdate { .. } => {}
            Modal::Sources { sources, histories } => match subject {
                UndoSubject::Source { index } => sources[index].path.reset(histories.sources[index].apply(shortcut)),
            },
        }
    }

    #[must_use]
    pub fn update(&mut self, event: Event) -> Option<Update> {
        match self {
            Modal::Settings | Modal::Error { .. } | Modal::Errors { .. } | Modal::AppUpdate { .. } => None,
            Modal::Sources { sources, histories } => match event {
                Event::EditedSource { action } => {
                    match action {
                        EditAction::Add => {
                            let value = StrictPath::default();
                            histories.sources.push(TextHistory::path(&value));
                            sources.push(Source::new(value));
                            return Some(Update::Task(scrollable::scroll_by(
                                (*SCROLLABLE).clone(),
                                scrollable::AbsoluteOffset { x: 0.0, y: f32::MAX },
                            )));
                        }
                        EditAction::Change(index, value) => {
                            histories.sources[index].push(&value);
                            sources[index] = Source::new(StrictPath::new(value));
                        }
                        EditAction::Remove(index) => {
                            histories.sources.remove(index);
                            sources.remove(index);
                        }
                        EditAction::Move(index, direction) => {
                            let offset = direction.shift(index);
                            histories.sources.swap(index, offset);
                            sources.swap(index, offset);
                        }
                    }
                    None
                }
                Event::Save => {
                    for index in (0..sources.len()).rev() {
                        if sources[index].path.raw_ref().trim().is_empty() {
                            sources.remove(index);
                            histories.sources.remove(index);
                        }
                    }

                    Some(Update::SavedSources {
                        sources: sources.clone(),
                        histories: histories.clone(),
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
        let histories = match self {
            Modal::Settings | Modal::Error { .. } | Modal::Errors { .. } | Modal::AppUpdate { .. } => histories,
            Modal::Sources { histories, .. } => histories,
        };

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
            .push(Container::new(opaque(self.content(viewport, config, histories, modifiers))).center(Length::Fill))
            .into()
    }
}
