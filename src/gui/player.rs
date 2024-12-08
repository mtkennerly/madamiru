use std::time::Duration;

use iced::{
    alignment, padding,
    widget::{horizontal_space, mouse_area, vertical_space, Image, Responsive, Svg},
    Alignment, Length,
};
use iced_gif::gif;
use iced_video_player::{Video, VideoPlayer};

use crate::{
    gui::{
        button,
        common::Message,
        grid,
        icon::Icon,
        style,
        widget::{text, Column, Container, Element, Row, Stack},
    },
    lang,
    media::Media,
    path::StrictPath,
    prelude::{timestamp_hhmmss, timestamp_mmss},
    resource::config::Playback,
};

fn timestamps<'a>(current: f64, total: Duration) -> Element<'a> {
    let current = current as u64;
    let total = total.as_secs();

    let (current, total) = if total > 60 * 60 {
        (timestamp_hhmmss(current), timestamp_hhmmss(total))
    } else {
        (timestamp_mmss(current), timestamp_mmss(total))
    };

    Row::new()
        .push(text(current))
        .push(horizontal_space())
        .push(text(total))
        .into()
}

#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn build_video(uri: &url::Url) -> Result<Video, iced_video_player::Error> {
    // Based on `iced_video_player::Video::new`,
    // but without a text sink so that the built-in subtitle functionality triggers.

    use gstreamer as gst;
    use gstreamer_app as gst_app;
    use gstreamer_app::prelude::*;

    gst::init()?;

    let pipeline = format!(
        r#"playbin uri="{}" video-sink="videoscale ! videoconvert ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1""#,
        uri.as_str()
    );
    let pipeline = gst::parse::launch(pipeline.as_ref())?
        .downcast::<gst::Pipeline>()
        .map_err(|_| iced_video_player::Error::Cast)?;

    let video_sink: gst::Element = pipeline.property("video-sink");
    let pad = video_sink.pads().first().cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap().downcast::<gst::Bin>().unwrap();
    let video_sink = bin.by_name("iced_video").unwrap();
    let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

    Video::from_gst_pipeline(pipeline, video_sink, None)
}

#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn build_video(uri: &url::Url) -> Result<Video, iced_video_player::Error> {
    Video::new(uri)
}

#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn build_video_player(video: &Video, grid_id: grid::Id, player_id: Id) -> Element {
    VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .on_end_of_stream(Message::Player {
            grid_id,
            player_id,
            event: Event::EndOfStream,
        })
        .on_new_frame(Message::Player {
            grid_id,
            player_id,
            event: Event::NewFrame,
        })
        .into()
}

#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn build_video_player(video: &Video, grid_id: grid::Id, player_id: Id) -> Element {
    VideoPlayer::new(video)
        .on_end_of_stream(Message::Player {
            grid_id,
            player_id,
            event: Event::EndOfStream,
        })
        .on_new_frame(Message::Player {
            grid_id,
            player_id,
            event: Event::NewFrame,
        })
        .into()
}

#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn mute_video(video: &mut Video, muted: bool) {
    video.set_muted(muted);
}

#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn mute_video(_video: &mut Video, _muted: bool) {
    // Panic: `property 'mute' of type 'GstPipeline' not found`
}

#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn seek_video(video: &mut Video, position: f64) {
    let _ = video.seek(Duration::from_secs_f64(position), false);
}

#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn seek_video(video: &mut Video, position: f64) {
    let _ = video.seek(Duration::from_secs_f64(position));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

#[derive(Debug)]
pub enum Error {
    Image(String),
    Io(std::io::Error),
    Path(crate::path::StrictPathError),
    Url,
    Video(iced_video_player::Error),
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            Self::Image(error) => error.to_string(),
            Self::Io(error) => error.to_string(),
            Self::Path(error) => format!("{error:?}"),
            Self::Url => "URL".to_string(),
            Self::Video(error) => error.to_string(),
        }
    }
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

impl From<gif::Error> for Error {
    fn from(value: gif::Error) -> Self {
        match value {
            gif::Error::Image(error) => Self::Image(error.to_string()),
            gif::Error::Io(error) => Self::Io(error),
        }
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
    Refresh,
    Close,
    WindowFocused,
    WindowUnfocused,
}

#[derive(Debug, Clone)]
pub enum Update {
    PauseChanged,
    MuteChanged,
    EndOfStream,
    Refresh,
    Close,
}

#[derive(Default)]
struct Overlay {
    show: bool,
    center_controls: bool,
    top_controls: bool,
    bottom_controls: bool,
    timestamps: bool,
}

#[derive(Default)]
pub enum Player {
    #[default]
    Idle,
    Error {
        media: Media,
        message: String,
        hovered: bool,
    },
    Image {
        media: Media,
        handle_path: std::path::PathBuf,
        position: f64,
        duration: Duration,
        paused: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Svg {
        media: Media,
        handle_path: std::path::PathBuf,
        position: f64,
        duration: Duration,
        paused: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Gif {
        media: Media,
        frames: gif::Frames,
        handle_path: std::path::PathBuf,
        position: f64,
        duration: Duration,
        paused: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Video {
        media: Media,
        video: Video,
        position: f64,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
}

impl Player {
    #[allow(clippy::result_large_err)]
    pub fn new(media: &Media, playback: &Playback) -> Result<Self, Self> {
        match media {
            Media::Image { path } => match Self::load_image(path) {
                Ok(handle_path) => Ok(Self::Image {
                    media: media.clone(),
                    handle_path,
                    position: 0.0,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    looping: false,
                    dragging: false,
                    hovered: false,
                    need_play_on_focus: false,
                }),
                Err(e) => Err(Self::Error {
                    media: media.clone(),
                    message: e.message(),
                    hovered: false,
                }),
            },
            Media::Svg { path } => match Self::load_svg(path) {
                Ok(handle_path) => Ok(Self::Svg {
                    media: media.clone(),
                    handle_path,
                    position: 0.0,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    looping: false,
                    dragging: false,
                    hovered: false,
                    need_play_on_focus: false,
                }),
                Err(e) => Err(Self::Error {
                    media: media.clone(),
                    message: e.message(),
                    hovered: false,
                }),
            },
            Media::Gif { path } => match Self::load_gif(path) {
                Ok((frames, handle_path)) => Ok(Self::Gif {
                    media: media.clone(),
                    frames,
                    handle_path,
                    position: 0.0,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    looping: false,
                    dragging: false,
                    hovered: false,
                    need_play_on_focus: false,
                }),
                Err(e) => Err(Self::Error {
                    media: media.clone(),
                    message: e.message(),
                    hovered: false,
                }),
            },
            Media::Video { path } => match Self::load_video(path) {
                Ok(mut video) => {
                    video.set_paused(playback.paused);
                    mute_video(&mut video, playback.muted);

                    Ok(Self::Video {
                        media: media.clone(),
                        video,
                        position: 0.0,
                        dragging: false,
                        hovered: false,
                        need_play_on_focus: false,
                    })
                }
                Err(e) => Err(Self::Error {
                    media: media.clone(),
                    message: e.message(),
                    hovered: false,
                }),
            },
        }
    }

    fn load_video(source: &StrictPath) -> Result<Video, Error> {
        Ok(build_video(
            &url::Url::from_file_path(source.as_std_path_buf()?).map_err(|_| Error::Url)?,
        )?)
    }

    fn load_image(source: &StrictPath) -> Result<std::path::PathBuf, Error> {
        Ok(source.as_std_path_buf()?)
    }

    fn load_svg(source: &StrictPath) -> Result<std::path::PathBuf, Error> {
        Ok(source.as_std_path_buf()?)
    }

    fn load_gif(source: &StrictPath) -> Result<(gif::Frames, std::path::PathBuf), Error> {
        let bytes = source.try_read_bytes()?;
        let frames = gif::Frames::from_bytes(bytes)?;
        let handle_path = source.as_std_path_buf()?;
        Ok((frames, handle_path))
    }

    pub fn swap_media(&mut self, media: &Media, playback: &Playback) -> Result<(), ()> {
        let playback = playback.with_muted_maybe(self.is_muted());
        let hovered = self.is_hovered();

        let mut error = false;
        *self = match Self::new(media, &playback) {
            Ok(player) => player,
            Err(player) => {
                error = true;
                player
            }
        };

        if let Some(hovered) = hovered {
            self.set_hovered(hovered);
        }

        if error {
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn restart(&mut self) {
        match self {
            Self::Idle => {}
            Self::Error { .. } => {}
            Self::Image { position, .. } => {
                *position = 0.0;
            }
            Self::Svg { position, .. } => {
                *position = 0.0;
            }
            Self::Gif { position, .. } => {
                *position = 0.0;
            }
            Self::Video { video, position, .. } => {
                *position = 0.0;
                seek_video(video, *position);
                video.set_paused(false);
            }
        }
    }

    pub fn media(&self) -> Option<&Media> {
        match self {
            Self::Idle => None,
            Self::Error { media, .. } => Some(media),
            Self::Image { media, .. } => Some(media),
            Self::Svg { media, .. } => Some(media),
            Self::Gif { media, .. } => Some(media),
            Self::Video { media, .. } => Some(media),
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            Self::Idle => false,
            Self::Error { .. } => true,
            Self::Image { .. } => false,
            Self::Svg { .. } => false,
            Self::Gif { .. } => false,
            Self::Video { .. } => false,
        }
    }

    pub fn is_paused(&self) -> Option<bool> {
        match self {
            Self::Idle => None,
            Self::Error { .. } => None,
            Self::Image { paused, .. } => Some(*paused),
            Self::Svg { paused, .. } => Some(*paused),
            Self::Gif { paused, .. } => Some(*paused),
            Self::Video { video, .. } => Some(video.paused()),
        }
    }

    pub fn is_muted(&self) -> Option<bool> {
        match self {
            Self::Idle => None,
            Self::Error { .. } => None,
            Self::Image { .. } => None,
            Self::Svg { .. } => None,
            Self::Gif { .. } => None,
            Self::Video { video, .. } => Some(video.muted()),
        }
    }

    pub fn is_hovered(&self) -> Option<bool> {
        match self {
            Self::Idle => None,
            Self::Error { hovered, .. } => Some(*hovered),
            Self::Image { hovered, .. } => Some(*hovered),
            Self::Svg { hovered, .. } => Some(*hovered),
            Self::Gif { hovered, .. } => Some(*hovered),
            Self::Video { hovered, .. } => Some(*hovered),
        }
    }

    pub fn set_hovered(&mut self, flag: bool) {
        match self {
            Self::Idle => {}
            Self::Error { hovered, .. } => {
                *hovered = flag;
            }
            Self::Image { hovered, .. } => {
                *hovered = flag;
            }
            Self::Svg { hovered, .. } => {
                *hovered = flag;
            }
            Self::Gif { hovered, .. } => {
                *hovered = flag;
            }
            Self::Video { hovered, .. } => {
                *hovered = flag;
            }
        }
    }

    pub fn tick(&mut self, elapsed: Duration) -> Option<Update> {
        match self {
            Self::Idle => None,
            Self::Error { .. } => None,
            Self::Image {
                position,
                duration,
                paused,
                looping,
                dragging,
                ..
            } => {
                if !*paused && !*dragging {
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
            Self::Svg {
                position,
                duration,
                paused,
                looping,
                dragging,
                ..
            } => {
                if !*paused && !*dragging {
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
            Self::Gif {
                position,
                duration,
                paused,
                looping,
                dragging,
                ..
            } => {
                if !*paused && !*dragging {
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
            Self::Video { .. } => None,
        }
    }

    fn overlay(&self, viewport: iced::Size, obscured: bool, hovered: bool) -> Overlay {
        let show = !obscured && hovered;

        match self {
            Self::Idle => Overlay::default(),
            Self::Error { .. } => Overlay {
                show,
                center_controls: show && viewport.height > 40.0 && viewport.width > 80.0,
                top_controls: show && viewport.width > 80.0,
                bottom_controls: false,
                timestamps: false,
            },
            Self::Image { .. } | Self::Svg { .. } | Self::Gif { .. } | Self::Video { .. } => Overlay {
                show,
                center_controls: show && viewport.height > 100.0 && viewport.width > 150.0,
                top_controls: show && viewport.width > 100.0,
                bottom_controls: show && viewport.height > 40.0,
                timestamps: show && viewport.height > 60.0 && viewport.width > 150.0,
            },
        }
    }

    #[must_use]
    pub fn update(&mut self, event: Event, playback: &Playback) -> Option<Update> {
        match self {
            Self::Idle => None,
            Self::Error { hovered, .. } => match event {
                Event::SetPause(_) => None,
                Event::SetLoop(_) => None,
                Event::SetMute(_) => None,
                Event::Seek(_) => None,
                Event::SeekStop => None,
                Event::SeekRandom => None,
                Event::EndOfStream => None,
                Event::NewFrame => None,
                Event::MouseEnter => {
                    *hovered = true;
                    None
                }
                Event::MouseExit => {
                    *hovered = false;
                    None
                }
                Event::Refresh => Some(Update::Refresh),
                Event::Close => Some(Update::Close),
                Event::WindowFocused => None,
                Event::WindowUnfocused => None,
            },
            Self::Image {
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged)
                }
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
                Event::Refresh => Some(Update::Refresh),
                Event::Close => Some(Update::Close),
                Event::WindowFocused => {
                    if *need_play_on_focus {
                        *paused = false;
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        *paused = true;
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
            Self::Svg {
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged)
                }
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
                Event::Refresh => Some(Update::Refresh),
                Event::Close => Some(Update::Close),
                Event::WindowFocused => {
                    if *need_play_on_focus {
                        *paused = false;
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        *paused = true;
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
            Self::Gif {
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged)
                }
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
                Event::Refresh => Some(Update::Refresh),
                Event::Close => Some(Update::Close),
                Event::WindowFocused => {
                    if *need_play_on_focus {
                        *paused = false;
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        *paused = true;
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
            Self::Video {
                video,
                position,
                dragging,
                hovered,
                need_play_on_focus,
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
                    mute_video(video, flag);
                    Some(Update::MuteChanged)
                }
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset;
                    seek_video(video, *position);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => {
                    use rand::Rng;
                    *position = rand::thread_rng().gen_range(0.0..video.duration().as_secs_f64());
                    seek_video(video, *position);
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
                Event::Refresh => Some(Update::Refresh),
                Event::Close => Some(Update::Close),
                Event::WindowFocused => {
                    if *need_play_on_focus {
                        video.set_paused(false);
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        video.set_paused(true);
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
        }
    }

    pub fn view(&self, grid_id: grid::Id, player_id: Id, obscured: bool) -> Element {
        Responsive::new(move |viewport| {
            mouse_area(self.view_inner(grid_id, player_id, obscured, viewport))
                .on_enter(if obscured {
                    Message::Ignore
                } else {
                    Message::Player {
                        grid_id,
                        player_id,
                        event: Event::MouseEnter,
                    }
                })
                .on_move(move |_| {
                    if obscured {
                        Message::Ignore
                    } else {
                        Message::Player {
                            grid_id,
                            player_id,
                            event: Event::MouseEnter,
                        }
                    }
                })
                .on_exit(if obscured {
                    Message::Ignore
                } else {
                    Message::Player {
                        grid_id,
                        player_id,
                        event: Event::MouseExit,
                    }
                })
                .into()
        })
        .into()
    }

    fn view_inner(&self, grid_id: grid::Id, player_id: Id, obscured: bool, viewport: iced::Size) -> Element {
        match self {
            Self::Idle => Container::new("")
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            Self::Error {
                media,
                message,
                hovered,
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered);

                Stack::new()
                    .push(
                        Container::new(text(message))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .push_maybe(
                        overlay.show.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.top_controls.then_some(
                            Container::new(
                                Row::new()
                                    .push(
                                        button::icon(Icon::OpenInNew)
                                            .on_press(Message::OpenFile {
                                                path: media.path().clone(),
                                            })
                                            .tooltip(media.path().render()),
                                    )
                                    .push(horizontal_space())
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Close,
                                            })
                                            .tooltip(lang::action::close()),
                                    ),
                            )
                            .align_top(Length::Fill)
                            .width(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.center_controls.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::big_icon(Icon::Refresh)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Refresh,
                                            })
                                            .tooltip(lang::action::shuffle_media()),
                                    ),
                            )
                            .center(Length::Fill),
                        ),
                    )
                    .into()
            }
            Self::Image {
                media,
                handle_path,
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || *dragging);

                Stack::new()
                    .push(
                        Container::new(Image::new(handle_path).width(Length::Fill).height(Length::Fill))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .push_maybe(
                        overlay.show.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.top_controls.then_some(
                            Container::new(
                                Row::new()
                                    .push(
                                        button::icon(Icon::Image)
                                            .on_press(Message::OpenFile {
                                                path: media.path().clone(),
                                            })
                                            .tooltip(media.path().render()),
                                    )
                                    .push(horizontal_space())
                                    .push(
                                        button::icon(Icon::Refresh)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Refresh,
                                            })
                                            .tooltip(lang::action::shuffle_media()),
                                    )
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Close,
                                            })
                                            .tooltip(lang::action::close()),
                                    ),
                            )
                            .align_top(Length::Fill)
                            .width(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.center_controls.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::big_icon(if *paused { Icon::Play } else { Icon::Pause })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::SetPause(!*paused),
                                            })
                                            .tooltip(if *paused {
                                                lang::action::play()
                                            } else {
                                                lang::action::pause()
                                            }),
                                    )
                                    .push(
                                        button::icon(if *looping { Icon::Loop } else { Icon::Shuffle })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
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
                        overlay.bottom_controls.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push_maybe(overlay.timestamps.then_some(timestamps(*position, *duration)))
                                    .push(Container::new(
                                        iced::widget::slider(0.0..=duration.as_secs_f64(), *position, move |x| {
                                            Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Seek(x),
                                            }
                                        })
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            grid_id,
                                            player_id,
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
            Self::Svg {
                media,
                handle_path,
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || *dragging);

                Stack::new()
                    .push(
                        Container::new(Svg::new(handle_path).width(Length::Fill).height(Length::Fill))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .push_maybe(
                        overlay.show.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.top_controls.then_some(
                            Container::new(
                                Row::new()
                                    .push(
                                        button::icon(Icon::Image)
                                            .on_press(Message::OpenFile {
                                                path: media.path().clone(),
                                            })
                                            .tooltip(media.path().render()),
                                    )
                                    .push(horizontal_space())
                                    .push(
                                        button::icon(Icon::Refresh)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Refresh,
                                            })
                                            .tooltip(lang::action::shuffle_media()),
                                    )
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Close,
                                            })
                                            .tooltip(lang::action::close()),
                                    ),
                            )
                            .align_top(Length::Fill)
                            .width(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.center_controls.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::big_icon(if *paused { Icon::Play } else { Icon::Pause })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::SetPause(!*paused),
                                            })
                                            .tooltip(if *paused {
                                                lang::action::play()
                                            } else {
                                                lang::action::pause()
                                            }),
                                    )
                                    .push(
                                        button::icon(if *looping { Icon::Loop } else { Icon::Shuffle })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
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
                        overlay.bottom_controls.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push_maybe(overlay.timestamps.then_some(timestamps(*position, *duration)))
                                    .push(Container::new(
                                        iced::widget::slider(0.0..=duration.as_secs_f64(), *position, move |x| {
                                            Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Seek(x),
                                            }
                                        })
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            grid_id,
                                            player_id,
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
            Self::Gif {
                media,
                frames,
                handle_path,
                position,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || *dragging);

                Stack::new()
                    .push({
                        let media = if *paused {
                            Container::new(Image::new(handle_path).width(Length::Fill).height(Length::Fill))
                        } else {
                            Container::new(gif(frames).width(Length::Fill).height(Length::Fill))
                        };

                        media
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill)
                    })
                    .push_maybe(
                        overlay.show.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.top_controls.then_some(
                            Container::new(
                                Row::new()
                                    .push(
                                        button::icon(Icon::Image)
                                            .on_press(Message::OpenFile {
                                                path: media.path().clone(),
                                            })
                                            .tooltip(media.path().render()),
                                    )
                                    .push(horizontal_space())
                                    .push(
                                        button::icon(Icon::Refresh)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Refresh,
                                            })
                                            .tooltip(lang::action::shuffle_media()),
                                    )
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Close,
                                            })
                                            .tooltip(lang::action::close()),
                                    ),
                            )
                            .align_top(Length::Fill)
                            .width(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.center_controls.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::big_icon(if *paused { Icon::Play } else { Icon::Pause })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::SetPause(!*paused),
                                            })
                                            .tooltip(if *paused {
                                                lang::action::play()
                                            } else {
                                                lang::action::pause()
                                            }),
                                    )
                                    .push(
                                        button::icon(if *looping { Icon::Loop } else { Icon::Shuffle })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
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
                        overlay.bottom_controls.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push_maybe(overlay.timestamps.then_some(timestamps(*position, *duration)))
                                    .push(Container::new(
                                        iced::widget::slider(0.0..=duration.as_secs_f64(), *position, move |x| {
                                            Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Seek(x),
                                            }
                                        })
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            grid_id,
                                            player_id,
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
            Self::Video {
                media,
                video,
                position,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || *dragging);

                Stack::new()
                    .push(
                        Container::new(build_video_player(video, grid_id, player_id))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                    .push_maybe(
                        overlay.show.then_some(
                            Container::new("")
                                .center(Length::Fill)
                                .class(style::Container::ModalBackground),
                        ),
                    )
                    .push_maybe(
                        overlay.top_controls.then_some(
                            Container::new(
                                Row::new()
                                    .push(
                                        button::icon(Icon::Movie)
                                            .on_press(Message::OpenFile {
                                                path: media.path().clone(),
                                            })
                                            .tooltip(media.path().render()),
                                    )
                                    .push(horizontal_space())
                                    .push(
                                        button::icon(Icon::Refresh)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Refresh,
                                            })
                                            .tooltip(lang::action::shuffle_media()),
                                    )
                                    .push(
                                        button::icon(Icon::Close)
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Close,
                                            })
                                            .tooltip(lang::action::close()),
                                    ),
                            )
                            .align_top(Length::Fill)
                            .width(Length::Fill),
                        ),
                    )
                    .push_maybe(
                        overlay.center_controls.then_some(
                            Container::new(
                                Row::new()
                                    .spacing(5)
                                    .align_y(alignment::Vertical::Center)
                                    .padding(padding::all(10.0))
                                    .push(
                                        button::icon(if video.muted() { Icon::Mute } else { Icon::VolumeHigh })
                                            .on_press(Message::Player {
                                                grid_id,
                                                player_id,
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
                                                grid_id,
                                                player_id,
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
                                                grid_id,
                                                player_id,
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
                        overlay.bottom_controls.then_some(
                            Container::new(
                                Column::new()
                                    .padding(padding::left(10).right(10).bottom(5))
                                    .push(vertical_space())
                                    .push_maybe(overlay.timestamps.then_some(timestamps(*position, video.duration())))
                                    .push(Container::new(
                                        iced::widget::slider(
                                            0.0..=video.duration().as_secs_f64(),
                                            *position,
                                            move |x| Message::Player {
                                                grid_id,
                                                player_id,
                                                event: Event::Seek(x),
                                            },
                                        )
                                        .step(0.1)
                                        .on_release(Message::Player {
                                            grid_id,
                                            player_id,
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
        }
    }
}
