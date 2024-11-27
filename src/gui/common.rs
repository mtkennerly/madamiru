use crate::{
    gui::{modal, player},
    lang::Language,
    prelude::StrictPath,
    resource::config::Theme,
};

#[derive(Clone, Debug, Default)]
pub struct Flags {
    pub sources: Vec<StrictPath>,
    pub max: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ignore,
    Exit,
    Save,
    CloseModal,
    AppReleaseToggle(bool),
    CheckAppRelease,
    AppReleaseChecked(Result<crate::metadata::Release, String>),
    BrowseDir(BrowseSubject),
    BrowseFile(BrowseFileSubject),
    SelectedFile(BrowseFileSubject, StrictPath),
    OpenDir { path: StrictPath },
    OpenDirSubject(BrowseSubject),
    OpenFile { path: StrictPath },
    OpenFileSubject(BrowseFileSubject),
    OpenDirFailure { path: StrictPath },
    OpenUrlFailure { url: String },
    KeyboardEvent(iced::keyboard::Event),
    SelectedLanguage(Language),
    SelectedTheme(Theme),
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
}

impl Message {
    pub fn browsed_dir(subject: BrowseSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => match subject {
                BrowseSubject::Source { index } => Message::Modal {
                    event: modal::Event::EditedSource {
                        action: EditAction::Change(
                            index,
                            globetter::Pattern::escape(&crate::path::render_pathbuf(&path)),
                        ),
                    },
                },
            },
            None => Message::Ignore,
        }
    }

    pub fn browsed_file(subject: BrowseFileSubject, choice: Option<std::path::PathBuf>) -> Self {
        match choice {
            Some(path) => Message::SelectedFile(subject, StrictPath::from(path)),
            None => Message::Ignore,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UndoSubject {
    Source { index: usize },
}
