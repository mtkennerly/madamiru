use std::{num::NonZeroUsize, time::Instant};

use iced::{
    widget::{pane_grid, text_input},
    Length,
};

use crate::{
    gui::{
        grid, modal, player,
        shortcuts::TextHistories,
        style,
        widget::{Element, TextInput, Undoable},
    },
    media,
    prelude::StrictPath,
    resource::config,
};

const ERROR_ICON: text_input::Icon<iced::Font> = text_input::Icon {
    font: crate::gui::font::ICONS,
    code_point: crate::gui::icon::Icon::Error.as_char(),
    size: None,
    spacing: 5.0,
    side: text_input::Side::Right,
};

#[derive(Clone, Debug, Default)]
pub struct Flags {
    pub sources: Vec<media::Source>,
    pub max_initial_media: Option<NonZeroUsize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ignore,
    Exit,
    Tick(Instant),
    Save,
    CloseModal,
    Config {
        event: config::Event,
    },
    CheckAppRelease,
    AppReleaseChecked(Result<crate::metadata::Release, String>),
    BrowseDir(BrowseSubject),
    BrowseFile(BrowseFileSubject),
    OpenDir {
        path: StrictPath,
    },
    OpenFile {
        path: StrictPath,
    },
    OpenDirFailure {
        path: StrictPath,
    },
    OpenUrlFailure {
        url: String,
    },
    KeyboardEvent(iced::keyboard::Event),
    UndoRedo(crate::gui::undoable::Action, UndoSubject),
    OpenUrl(String),
    OpenUrlAndCloseModal(String),
    Refresh,
    SetPause(bool),
    SetMute(bool),
    Player {
        grid_id: grid::Id,
        player_id: player::Id,
        event: player::Event,
    },
    AllPlayers {
        event: player::Event,
    },
    Modal {
        event: modal::Event,
    },
    ShowSettings,
    FindMedia,
    MediaFound {
        context: media::RefreshContext,
        media: media::SourceMap,
    },
    FileDragDrop(StrictPath),
    FileDragDropGridSelected(grid::Id),
    WindowFocused,
    WindowUnfocused,
    Pane {
        event: PaneEvent,
    },
}

impl Message {
    pub fn browsed_dir(subject: BrowseSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => match subject {
                BrowseSubject::Source { index } => Self::Modal {
                    event: modal::Event::EditedSource {
                        action: EditAction::Change(index, crate::path::render_pathbuf(&path)),
                    },
                },
            },
            None => Self::Ignore,
        }
    }

    pub fn browsed_file(subject: BrowseFileSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => match subject {
                BrowseFileSubject::Source { index } => Self::Modal {
                    event: modal::Event::EditedSource {
                        action: EditAction::Change(index, crate::path::render_pathbuf(&path)),
                    },
                },
            },
            None => Self::Ignore,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAction {
    Add,
    Change(usize, String),
    Remove(usize),
    Move(usize, EditDirection),
}

impl EditAction {
    pub fn move_up(index: usize) -> Self {
        Self::Move(index, EditDirection::Up)
    }

    pub fn move_down(index: usize) -> Self {
        Self::Move(index, EditDirection::Down)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditDirection {
    Up,
    Down,
}

impl EditDirection {
    pub fn shift(&self, index: usize) -> usize {
        match self {
            Self::Up => index - 1,
            Self::Down => index + 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseSubject {
    Source { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseFileSubject {
    Source { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UndoSubject {
    MaxInitialMedia,
    ImageDuration,
    Source { index: usize },
    OrientationLimit,
}

impl UndoSubject {
    pub fn view_with<'a>(self, histories: &TextHistories) -> Element<'a> {
        match self {
            Self::MaxInitialMedia => self.view(&histories.max_initial_media.current()),
            Self::ImageDuration => self.view(&histories.image_duration.current()),
            Self::Source { .. } => self.view(""),
            Self::OrientationLimit { .. } => self.view(""),
        }
    }

    pub fn view<'a>(self, current: &str) -> Element<'a> {
        let event: Box<dyn Fn(String) -> Message> = match self {
            UndoSubject::MaxInitialMedia => Box::new(move |value| Message::Config {
                event: config::Event::MaxInitialMediaRaw(value),
            }),
            UndoSubject::ImageDuration => Box::new(move |value| Message::Config {
                event: config::Event::ImageDurationRaw(value),
            }),
            UndoSubject::Source { index } => Box::new(move |value| Message::Modal {
                event: modal::Event::EditedSource {
                    action: EditAction::Change(index, value),
                },
            }),
            UndoSubject::OrientationLimit => Box::new(move |value| Message::Modal {
                event: modal::Event::EditedGridOrientationLimit { raw_limit: value },
            }),
        };

        let placeholder = "";

        let icon = match self {
            UndoSubject::MaxInitialMedia => (current.parse::<NonZeroUsize>().is_err()).then_some(ERROR_ICON),
            UndoSubject::ImageDuration => (current.parse::<NonZeroUsize>().is_err()).then_some(ERROR_ICON),
            UndoSubject::Source { .. } => (!path_appears_valid(current)).then_some(ERROR_ICON),
            UndoSubject::OrientationLimit => (current.parse::<NonZeroUsize>().is_err()).then_some(ERROR_ICON),
        };

        let width = match self {
            UndoSubject::MaxInitialMedia => Length::Fixed(80.0),
            UndoSubject::ImageDuration => Length::Fixed(80.0),
            UndoSubject::Source { .. } => Length::Fill,
            UndoSubject::OrientationLimit => Length::Fixed(80.0),
        };

        Undoable::new(
            {
                let mut input = TextInput::new(placeholder, current)
                    .on_input(event)
                    .class(style::TextInput)
                    .padding(5)
                    .width(width);

                if let Some(icon) = icon {
                    input = input.icon(icon);
                }

                input
            },
            move |action| Message::UndoRedo(action, self),
        )
        .into()
    }
}

fn path_appears_valid(path: &str) -> bool {
    !path.contains("://")
}

#[derive(Debug, Clone)]
pub enum PaneEvent {
    Drag(pane_grid::DragEvent),
    Resize(pane_grid::ResizeEvent),
    Split { grid_id: grid::Id, axis: pane_grid::Axis },
    Close { grid_id: grid::Id },
    AddPlayer { grid_id: grid::Id },
    ShowSettings { grid_id: grid::Id },
    ShowControls { grid_id: grid::Id },
    CloseControls,
    SetMute { grid_id: grid::Id, muted: bool },
    SetPause { grid_id: grid::Id, paused: bool },
    SeekRandom { grid_id: grid::Id },
    Refresh { grid_id: grid::Id },
}
