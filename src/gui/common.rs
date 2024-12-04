use std::{num::NonZeroUsize, time::Instant};

use crate::{
    gui::{modal, player},
    media,
    prelude::StrictPath,
    resource::config,
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
    Config { event: config::Event },
    CheckAppRelease,
    AppReleaseChecked(Result<crate::metadata::Release, String>),
    BrowseDir(BrowseSubject),
    BrowseFile(BrowseFileSubject),
    OpenDir { path: StrictPath },
    OpenDirSubject(BrowseSubject),
    OpenFile { path: StrictPath },
    OpenFileSubject(BrowseFileSubject),
    OpenDirFailure { path: StrictPath },
    OpenUrlFailure { url: String },
    KeyboardEvent(iced::keyboard::Event),
    UndoRedo(crate::gui::undoable::Action, UndoSubject),
    OpenUrl(String),
    OpenUrlAndCloseModal(String),
    Refresh,
    AddPlayer,
    SetPause(bool),
    SetMute(bool),
    Player { pane: player::Id, event: player::Event },
    AllPlayers { event: player::Event },
    Modal { event: modal::Event },
    ShowSettings,
    ShowSources,
    FindMedia,
    MediaFound { refresh: bool, media: media::Collection },
    FileDragDrop(StrictPath),
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
}
