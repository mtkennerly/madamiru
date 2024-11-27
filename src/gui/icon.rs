use iced::alignment;

use crate::gui::{
    font,
    widget::{text, Text},
};

pub enum Icon {
    Add,
    ArrowDownward,
    ArrowUpward,
    Close,
    Error,
    File,
    FileOpen,
    FolderOpen,
    LogOut,
    Loop,
    MoreVert,
    Mute,
    OpenInBrowser,
    OpenInNew,
    Pause,
    Play,
    PlaylistAdd,
    Refresh,
    Settings,
    Shuffle,
    TimerRefresh,
    VolumeHigh,
}

impl Icon {
    pub fn as_char(&self) -> char {
        match self {
            Self::Add => '\u{E145}',
            Self::ArrowDownward => '\u{E5DB}',
            Self::ArrowUpward => '\u{E5D8}',
            Self::Close => '\u{e14c}',
            Self::Error => '\u{e000}',
            Self::File => '\u{e24d}',
            Self::FileOpen => '\u{eaf3}',
            Self::FolderOpen => '\u{E2C8}',
            Self::LogOut => '\u{e9ba}',
            Self::Loop => '\u{e040}',
            Self::MoreVert => '\u{E5D4}',
            Self::Mute => '\u{e04f}',
            Self::OpenInBrowser => '\u{e89d}',
            Self::OpenInNew => '\u{E89E}',
            Self::Pause => '\u{e034}',
            Self::Play => '\u{e037}',
            Self::Refresh => '\u{E5D5}',
            Self::Settings => '\u{E8B8}',
            Self::Shuffle => '\u{e043}',
            Self::TimerRefresh => '\u{e889}',
            Self::VolumeHigh => '\u{e050}',
            Self::PlaylistAdd => '\u{e03b}',
        }
    }

    pub fn big_control(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(40)
            .width(40)
            .height(40)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }

    pub fn small_control(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(20)
            .width(20)
            .height(20)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }
}
