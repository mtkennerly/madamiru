use iced::{alignment, Length};

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
    Image,
    LogOut,
    Loop,
    MoreVert,
    #[cfg(feature = "video")]
    Movie,
    #[cfg(feature = "audio")]
    Music,
    Mute,
    OpenInBrowser,
    OpenInNew,
    Pause,
    Play,
    PlaylistAdd,
    PlaylistRemove,
    Refresh,
    Save,
    SaveAs,
    Settings,
    Shuffle,
    SplitHorizontal,
    SplitVertical,
    TimerRefresh,
    VolumeHigh,
}

impl Icon {
    pub const fn as_char(&self) -> char {
        match self {
            Self::Add => '\u{E145}',
            Self::ArrowDownward => '\u{E5DB}',
            Self::ArrowUpward => '\u{E5D8}',
            Self::Close => '\u{e14c}',
            Self::Error => '\u{e000}',
            Self::File => '\u{e24d}',
            Self::FileOpen => '\u{eaf3}',
            Self::FolderOpen => '\u{E2C8}',
            Self::Image => '\u{e3f4}',
            Self::LogOut => '\u{e9ba}',
            Self::Loop => '\u{e040}',
            Self::MoreVert => '\u{E5D4}',
            #[cfg(feature = "video")]
            Self::Movie => '\u{e02c}',
            #[cfg(feature = "audio")]
            Self::Music => '\u{e405}',
            Self::Mute => '\u{e04f}',
            Self::OpenInBrowser => '\u{e89d}',
            Self::OpenInNew => '\u{E89E}',
            Self::Pause => '\u{e034}',
            Self::Play => '\u{e037}',
            Self::Refresh => '\u{E5D5}',
            Self::Save => '\u{e161}',
            Self::SaveAs => '\u{eb60}',
            Self::Settings => '\u{E8B8}',
            Self::Shuffle => '\u{e043}',
            Self::SplitHorizontal => '\u{e8d4}',
            Self::SplitVertical => '\u{e8d5}',
            Self::TimerRefresh => '\u{e889}',
            Self::VolumeHigh => '\u{e050}',
            Self::PlaylistAdd => '\u{e03b}',
            Self::PlaylistRemove => '\u{eb80}',
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

    pub fn mini_control(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(14)
            .width(14)
            .height(14)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }

    pub fn max_control(self) -> Text<'static> {
        text(self.as_char().to_string())
            .font(font::ICONS)
            .size(40)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .line_height(1.0)
    }
}
