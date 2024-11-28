use std::time::Duration;

use iced::{
    padding,
    widget::{horizontal_space, mouse_area, vertical_space, Image},
    Length,
};
use iced_video_player::{Video, VideoPlayer};

use crate::{
    gui::{
        button,
        common::Message,
        icon::Icon,
        style,
        widget::{text, Column, Container, Element, Row, Stack},
    },
    lang,
    media::Media,
    path::StrictPath,
    resource::config::Playback,
};

#[derive(Debug, Clone, Copy)]
pub struct Id(pub usize);

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Path(crate::path::StrictPathError),
    Url,
    Video(iced_video_player::Error),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<crate::path::StrictPathError> for Error {
    fn from(value: crate::path::StrictPathError) -> Self {
        Self::Path(value)
    }
}

impl From<iced_video_player::Error> for Error {
    fn from(value: iced_video_player::Error) -> Self {
        Self::Video(value)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    SetPause(bool),
    SetLoop(bool),
    SetMute(bool),
    Seek(f64),
    SeekStop,
    SeekRandom,
    EndOfStream,
    NewFrame,
    MouseEnter,
    MouseExit,
    Close,
}

#[derive(Debug, Clone)]
pub enum Update {
    PauseChanged,
    MuteChanged,
    EndOfStream,
    Close,
}

#[derive(Default)]
pub enum Player {
    #[default]
    Idle,
    Error {
        source: StrictPath,
        message: String,
    },
    Image {
        source: StrictPath,
        handle_path: std::path::PathBuf,
        position: f64,
        duration: Duration,
        looping: bool,
        dragging: bool,
        hovered: bool,
    },
    Video {
        source: StrictPath,
        video: Video,
        position: f64,
        dragging: bool,
        hovered: bool,
    },
}

impl Player {
    pub fn new(media: &Media, playback: &Playback) -> Self {
        match media {
            Media::Image { path } => match Self::load_image(path) {
                Ok(handle_path) => Self::Image {
                    source: path.clone(),
                    handle_path,
                    position: 0.0,
                    duration: Duration::from_secs(10),
                    looping: false,
                    dragging: false,
                    hovered: false,
                },
                Err(e) => Self::Error {
                    source: path.clone(),
                    message: match e {
                        Error::Io(error) => error.to_string(),
                        Error::Path(error) => format!("{error:?}"),
                        Error::Url => "URL".to_string(),
                        Error::Video(error) => error.to_string(),
                    },
                },
            },
            Media::Video { path } => match Self::load_video(path) {
                Ok(mut video) => {
                    video.set_paused(playback.paused);
                    video.set_muted(playback.muted);

                    Self::Video {
                        source: path.clone(),
                        video,
                        position: 0.0,
                        dragging: false,
                        hovered: false,
                    }
                }
                Err(e) => Self::Error {
                    source: path.clone(),
                    message: match e {
                        Error::Io(error) => error.to_string(),
                        Error::Path(error) => format!("{error:?}"),
                        Error::Url => "URL".to_string(),
                        Error::Video(error) => error.to_string(),
                    },
                },
            },
        }
    }

    fn load_video(source: &StrictPath) -> Result<Video, Error> {
        Ok(Video::new(
            &url::Url::from_file_path(source.as_std_path_buf()?).map_err(|_| Error::Url)?,
        )?)
    }

    fn load_image(source: &StrictPath) -> Result<std::path::PathBuf, Error> {
        Ok(source.as_std_path_buf()?)
    }

    pub fn swap_media(&mut self, media: &Media, playback: &Playback) {
        *self = Self::new(media, playback)
    }

    pub fn restart(&mut self) {
        match self {
            Player::Idle => {}
            Player::Error { .. } => {}
            Player::Image { position, .. } => {
                *position = 0.0;
            }
            Player::Video { video, position, .. } => {
                *position = 0.0;
                let _ = video.seek(Duration::from_secs_f64(*position), false);
                video.set_paused(false);
            }
        }
    }

    pub fn source(&self) -> Option<&StrictPath> {
        match self {
            Player::Idle => None,
            Player::Error { source, .. } => Some(source),
            Player::Image { source, .. } => Some(source),
            Player::Video { source, .. } => Some(source),
        }
    }

    pub fn is_paused(&self) -> Option<bool> {
        match self {
            Player::Idle => None,
            Player::Error { .. } => None,
            Player::Image { .. } => None,
            Player::Video { video, .. } => Some(video.paused()),
        }
    }

    pub fn is_muted(&self) -> Option<bool> {
        match self {
            Player::Idle => None,
            Player::Error { .. } => None,
            Player::Image { .. } => None,
            Player::Video { video, .. } => Some(video.muted()),
        }
    }

    pub fn tick(&mut self, elapsed: Duration) -> Option<Update> {
        match self {
            Player::Idle => None,
            Player::Error { .. } => None,
            Player::Image {
                position,
                duration,
                looping,
                dragging,
                ..
            } => {
                if !*dragging {
                    *position += elapsed.as_secs_f64();
                }

                if *position >= duration.as_secs_f64() {
                    if *looping {
                        *position = 0.0;
                        None
                    } else {
                        Some(Update::EndOfStream)
                    }
                } else {
                    None
                }
            }
            Player::Video { .. } => None,
        }
    }

    #[must_use]
    pub fn update(&mut self, event: Event) -> Option<Update> {
        match self {
            Player::Idle => None,
            Player::Error { .. } => None,
            Player::Image {
                position,
                duration,
                looping,
                dragging,
                hovered,
                ..
            } => match event {
                Event::SetPause(_) => None,
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(_) => None,
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset.min(duration.as_secs_f64());
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => None,
                Event::EndOfStream => Some(Update::EndOfStream),
                Event::NewFrame => None,
                Event::MouseEnter => {
                    *hovered = true;
                    None
                }
                Event::MouseExit => {
                    *hovered = false;
                    None
                }
                Event::Close => Some(Update::Close),
            },
            Player::Video {
                video,
                position,
                dragging,
                hovered,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    video.set_paused(flag);
                    Some(Update::PauseChanged)
                }
                Event::SetLoop(flag) => {
                    video.set_looping(flag);
                    None
                }
                Event::SetMute(flag) => {
                    video.set_muted(flag);
                    Some(Update::MuteChanged)
                }
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset;
                    // video.seek(Duration::from_secs_f64(*position)).expect("seek");
                    video.seek(Duration::from_secs_f64(*position), false).expect("seek");
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => {
                    use rand::Rng;
                    *position = rand::thread_rng().gen_range(0.0..video.duration().as_secs_f64());
                    video.seek(Duration::from_secs_f64(*position), false).expect("seek");
                    None
                }
                Event::EndOfStream => (!video.looping()).then_some(Update::EndOfStream),
                Event::NewFrame => {
                    *position = video.position().as_secs_f64();
                    None
                }
                Event::MouseEnter => {
                    *hovered = true;
                    None
                }
                Event::MouseExit => {
                    *hovered = false;
                    None
                }
                Event::Close => Some(Update::Close),
            },
        }
    }

    pub fn view(&self, pane: Id, obscured: bool) -> Element {
        let content: Element = match self {
            Player::Idle => Container::new("")
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
            Player::Error { source, message } => Container::new(text(format!("{}\n\n{}", source.render(), message)))
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
            Player::Image {
                source,
                handle_path,
                position,
                duration,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = *hovered || *dragging;

                Stack::new()
                    .push(
                        Container::new(Image::new(handle_path))
                            .align_x(iced::Alignment::Center)
                            .align_y(iced::Alignment::Center)
                            .width(iced::Length::Fill)
                            .height(iced::Length::Fill),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(text(source.render()).size(15))
                                .padding(padding::right(30))
                                .align_top(Length::Fill)
                                .align_left(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Row::new().push(
                                    button::icon(Icon::Close)
                                        .on_press(Message::Player {
                                            pane,
                                            event: Event::Close,
                                        })
                                        .tooltip(lang::action::close()),
                                ),
                            )
                            .align_top(Length::Fill)
                            .align_right(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(iced::alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::big_icon(if *looping { Icon::Loop } else { Icon::Shuffle })
                                            .on_press(Message::Player {
                                                pane,
                                                event: Event::SetLoop(!*looping),
                                            })
                                            .tooltip(if *looping {
                                                lang::tell::player_will_loop()
                                            } else {
                                                lang::tell::player_will_shuffle()
                                            }),
                                    ),
                            )
                            .center(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push(
                                        Row::new()
                                            .push(text(format!(
                                                "{:02}:{:02}",
                                                *position as u64 / 60,
                                                *position as u64 % 60
                                            )))
                                            .push(horizontal_space())
                                            .push(text(format!(
                                                "{:02}:{:02}",
                                                duration.as_secs() / 60,
                                                duration.as_secs() % 60
                                            ))),
                                    )
                                    .push(Container::new(
                                        iced::widget::slider(0.0..=duration.as_secs_f64(), *position, move |x| {
                                            Message::Player {
                                                pane,
                                                event: Event::Seek(x),
                                            }
                                        })
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            pane,
                                            event: Event::SeekStop,
                                        }),
                                    )),
                            )
                            .align_bottom(Length::Fill)
                            .center_x(Length::Fill),
                        ),
                    )
                    .into()
            }
            Player::Video {
                source,
                video,
                position,
                dragging,
                hovered,
                ..
            } => {
                let overlay = *hovered || *dragging;

                Stack::new()
                    .push(
                        Container::new(
                            VideoPlayer::new(video)
                                // .width(iced::Length::Fill)
                                // .height(iced::Length::Fill)
                                // .content_fit(iced::ContentFit::Contain)
                                .on_end_of_stream(Message::Player {
                                    pane,
                                    event: Event::EndOfStream,
                                })
                                .on_new_frame(Message::Player {
                                    pane,
                                    event: Event::NewFrame,
                                }),
                        )
                        .align_x(iced::Alignment::Center)
                        .align_y(iced::Alignment::Center)
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(text(source.render()).size(15))
                                .padding(padding::right(30))
                                .align_top(Length::Fill)
                                .align_left(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Row::new().push(
                                    button::icon(Icon::Close)
                                        .on_press(Message::Player {
                                            pane,
                                            event: Event::Close,
                                        })
                                        .tooltip(lang::action::close()),
                                ),
                            )
                            .align_top(Length::Fill)
                            .align_right(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(iced::alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::icon(if video.muted() { Icon::Mute } else { Icon::VolumeHigh })
                                            .on_press(Message::Player {
                                                pane,
                                                event: Event::SetMute(!video.muted()),
                                            })
                                            .tooltip(if video.muted() {
                                                lang::action::unmute()
                                            } else {
                                                lang::action::mute()
                                            }),
                                    )
                                    .push(
                                        button::big_icon(if video.paused() { Icon::Play } else { Icon::Pause })
                                            .on_press(Message::Player {
                                                pane,
                                                event: Event::SetPause(!video.paused()),
                                            })
                                            .tooltip(if video.paused() {
                                                lang::action::play()
                                            } else {
                                                lang::action::pause()
                                            }),
                                    )
                                    .push(
                                        button::icon(if video.looping() { Icon::Loop } else { Icon::Shuffle })
                                            .on_press(Message::Player {
                                                pane,
                                                event: Event::SetLoop(!video.looping()),
                                            })
                                            .tooltip(if video.looping() {
                                                lang::tell::player_will_loop()
                                            } else {
                                                lang::tell::player_will_shuffle()
                                            }),
                                    ),
                            )
                            .center(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push(
                                        Row::new()
                                            .push(text(format!(
                                                "{:02}:{:02}",
                                                *position as u64 / 60,
                                                *position as u64 % 60
                                            )))
                                            .push(horizontal_space())
                                            .push(text(format!(
                                                "{:02}:{:02}",
                                                video.duration().as_secs() / 60,
                                                video.duration().as_secs() % 60
                                            ))),
                                    )
                                    .push(Container::new(
                                        iced::widget::slider(
                                            0.0..=video.duration().as_secs_f64(),
                                            *position,
                                            move |x| Message::Player {
                                                pane,
                                                event: Event::Seek(x),
                                            },
                                        )
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            pane,
                                            event: Event::SeekStop,
                                        }),
                                    )),
                            )
                            .align_bottom(Length::Fill)
                            .center_x(Length::Fill),
                        ),
                    )
                    .into()
            }
        };

        mouse_area(content)
            .on_enter(if obscured {
                Message::Ignore
            } else {
                Message::Player {
                    pane,
                    event: Event::MouseEnter,
                }
            })
            .on_move(move |_| {
                if obscured {
                    Message::Ignore
                } else {
                    Message::Player {
                        pane,
                        event: Event::MouseEnter,
                    }
                }
            })
            .on_exit(if obscured {
                Message::Ignore
            } else {
                Message::Player {
                    pane,
                    event: Event::MouseExit,
                }
            })
            .into()
    }
}
