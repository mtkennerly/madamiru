use std::{sync::Arc, time::Duration};

use iced::{
    alignment, padding,
    widget::{mouse_area, space, Image, Responsive, Svg},
    Alignment, Length,
};
use iced_moving_picture::{apng, gif};

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
type VideoPipeline = gstreamer::Pipeline;

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
type VideoPipeline = ();

use crate::{
    gui::{
        button,
        common::{Message, Step},
        grid,
        icon::Icon,
        style,
        widget::{text, Column, Container, Element, Row, Stack},
    },
    lang,
    media::Media,
    path::StrictPath,
    prelude::{timestamp_hhmmss, timestamp_mmss},
    resource::{config::Playback, playlist::ContentFit},
};

const IMAGE_STEP: Duration = Duration::from_secs(2);
#[cfg(feature = "audio")]
const AUDIO_STEP: Duration = Duration::from_secs(10);
#[cfg(feature = "video")]
const VIDEO_STEP: Duration = Duration::from_secs(10);

fn timestamps<'a>(current: Duration, total: Duration) -> Element<'a> {
    let current = current.as_secs();
    let total = total.as_secs();

    let (current, total) = if total > 60 * 60 {
        (timestamp_hhmmss(current), timestamp_hhmmss(total))
    } else {
        (timestamp_mmss(current), timestamp_mmss(total))
    };

    Row::new()
        .push(text(current))
        .push(space::horizontal())
        .push(text(total))
        .into()
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn build_video(uri: &url::Url) -> Result<iced_video_player::Video, iced_video_player::Error> {
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
        .downcast::<VideoPipeline>()
        .map_err(|_| iced_video_player::Error::Cast)?;

    let video_sink: gst::Element = pipeline.property("video-sink");
    let pad = video_sink.pads().first().cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap().downcast::<gst::Bin>().unwrap();
    let video_sink = bin.by_name("iced_video").unwrap();
    let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

    iced_video_player::Video::from_gst_pipeline(pipeline, video_sink, None)
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn build_video(uri: &url::Url) -> Result<iced_video_player::Video, iced_video_player::Error> {
    iced_video_player::Video::new(uri)
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn get_video_pipeline(video: &iced_video_player::Video) -> VideoPipeline {
    video.pipeline()
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn get_video_pipeline(_video: &iced_video_player::Video) -> VideoPipeline {
    ()
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn get_video_duration(pipeline: &VideoPipeline) -> Option<gstreamer::ClockTime> {
    use gstreamer::prelude::ElementExtManual;

    pipeline.query_duration::<gstreamer::ClockTime>()
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn get_video_duration(_pipeline: &VideoPipeline) -> Option<gstreamer::ClockTime> {
    None
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn get_video_position(pipeline: &VideoPipeline, _video: &iced_video_player::Video) -> Option<Duration> {
    use gstreamer::prelude::ElementExtManual;

    let clock_time = pipeline.query_position::<gstreamer::ClockTime>()?;
    Some(Duration::from_nanos(clock_time.nseconds()))
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn get_video_position(_pipeline: &VideoPipeline, video: &iced_video_player::Video) -> Option<Duration> {
    Some(video.position())
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn build_video_player(
    video: &iced_video_player::Video,
    grid_id: grid::Id,
    player_id: Id,
    content_fit: ContentFit,
) -> Element {
    iced_video_player::VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(content_fit.into())
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

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn build_video_player(
    video: &iced_video_player::Video,
    grid_id: grid::Id,
    player_id: Id,
    _content_fit: ContentFit,
) -> Element {
    iced_video_player::VideoPlayer::new(video)
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

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn mute_video(video: &mut iced_video_player::Video, muted: bool) {
    video.set_muted(muted);
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn mute_video(_video: &mut iced_video_player::Video, _muted: bool) {
    // Panic: `property 'mute' of type 'GstPipeline' not found`
}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn set_video_volume(video: &mut iced_video_player::Video, volume: f32) {
    video.set_volume(volume as f64);
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn set_video_volume(_video: &mut iced_video_player::Video, _volume: f32) {}

#[cfg(feature = "video")]
#[realia::dep_since("madamiru", "iced_video_player", "0.6.0")]
fn seek_video(video: &mut iced_video_player::Video, position: Duration) {
    let _ = video.seek(position, false);
}

#[cfg(feature = "video")]
#[realia::dep_before("madamiru", "iced_video_player", "0.6.0")]
fn seek_video(video: &mut iced_video_player::Video, position: Duration) {
    let _ = video.seek(position);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "audio")]
    Audio(String),
    Image(String),
    Io(Arc<std::io::Error>),
    Path(crate::path::StrictPathError),
    #[cfg(feature = "video")]
    Url,
    #[cfg(feature = "video")]
    Video(iced_video_player::Error),
}

impl Error {
    pub fn message(&self) -> String {
        match self {
            #[cfg(feature = "audio")]
            Self::Audio(error) => error.to_string(),
            Self::Image(error) => error.to_string(),
            Self::Io(error) => error.to_string(),
            Self::Path(error) => format!("{error:?}"),
            #[cfg(feature = "video")]
            Self::Url => "URL".to_string(),
            #[cfg(feature = "video")]
            Self::Video(error) => error.to_string(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(Arc::new(value))
    }
}

impl From<Arc<std::io::Error>> for Error {
    fn from(value: Arc<std::io::Error>) -> Self {
        Self::Io(value)
    }
}

impl From<crate::path::StrictPathError> for Error {
    fn from(value: crate::path::StrictPathError) -> Self {
        Self::Path(value)
    }
}

#[cfg(feature = "video")]
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

impl From<apng::Error> for Error {
    fn from(value: apng::Error) -> Self {
        match value {
            apng::Error::Image(error) => Self::Image(error.to_string()),
            apng::Error::Io(error) => Self::Io(error),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    SetPause(bool),
    SetLoop(bool),
    SetMute(bool),
    SetVolume(f32),
    Seek(Duration),
    SeekRelative(f64),
    SeekStop,
    SeekRandom,
    SeekRandomRelative(f64),
    Step(Step),
    EndOfStream,
    NewFrame,
    MouseEnter,
    MouseExit,
    Refresh,
    Close,
    WindowFocused,
    WindowUnfocused,
}

impl Event {
    pub fn seek_random_relative() -> Self {
        use rand::Rng;
        let position = rand::rng().random_range(0.0..0.95);
        Self::SeekRandomRelative(position)
    }
}

#[derive(Debug, Clone)]
pub enum Update {
    PauseChanged(bool),
    #[cfg_attr(not(any(feature = "audio", feature = "video")), allow(unused))]
    MuteChanged,
    RelativePositionChanged(f64),
    Step(Step),
    EndOfStream,
    Refresh,
    Close,
}

impl Update {
    fn relative_position_changed(absolute_position: Duration, duration: Duration) -> Option<Self> {
        let relative = absolute_position.as_secs_f64() / duration.as_secs_f64();
        relative.is_finite().then_some(Self::RelativePositionChanged(relative))
    }
}

#[derive(Default)]
struct Overlay {
    show: bool,
    center_controls: bool,
    top_controls: bool,
    bottom_controls: bool,
    timestamps: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Other,
    Image,
    #[cfg(feature = "audio")]
    Audio,
    #[cfg(feature = "video")]
    Video,
}

pub enum Player {
    Idle {
        hovered: bool,
    },
    Error {
        media: Media,
        message: String,
        hovered: bool,
    },
    Image {
        media: Media,
        handle: iced::widget::image::Handle,
        position: Duration,
        duration: Duration,
        paused: bool,
        muted: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Svg {
        media: Media,
        handle: iced::widget::svg::Handle,
        position: Duration,
        duration: Duration,
        paused: bool,
        muted: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Gif {
        media: Media,
        frames: gif::Frames,
        handle: iced::widget::image::Handle,
        position: Duration,
        duration: Duration,
        paused: bool,
        muted: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    Apng {
        media: Media,
        frames: apng::Frames,
        handle: iced::widget::image::Handle,
        position: Duration,
        duration: Duration,
        paused: bool,
        muted: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    #[cfg(feature = "audio")]
    Audio {
        media: Media,
        // We must hold the stream for as long as the sink.
        #[allow(unused)]
        stream: rodio::OutputStream,
        sink: rodio::Sink,
        duration: Duration,
        paused: bool,
        looping: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
    #[cfg(feature = "video")]
    Video {
        media: Media,
        video: iced_video_player::Video,
        pipeline: VideoPipeline,
        position: Duration,
        duration: Duration,
        paused: bool,
        dragging: bool,
        hovered: bool,
        need_play_on_focus: bool,
    },
}

impl Default for Player {
    fn default() -> Self {
        Self::Idle { hovered: false }
    }
}

impl Player {
    #[allow(clippy::result_large_err)]
    pub fn new(media: &Media, playback: &Playback) -> Result<Self, Self> {
        match media {
            Media::Image { path } => match Self::load_image(path) {
                Ok(handle) => Ok(Self::Image {
                    media: media.clone(),
                    handle,
                    position: Duration::ZERO,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    muted: playback.muted,
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
                Ok(handle) => Ok(Self::Svg {
                    media: media.clone(),
                    handle,
                    position: Duration::ZERO,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    muted: playback.muted,
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
                Ok((frames, handle)) => Ok(Self::Gif {
                    media: media.clone(),
                    frames,
                    handle,
                    position: Duration::ZERO,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    muted: playback.muted,
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
            Media::Apng { path } => match Self::load_apng(path) {
                Ok((frames, handle)) => Ok(Self::Apng {
                    media: media.clone(),
                    frames,
                    handle,
                    position: Duration::ZERO,
                    duration: Duration::from_secs(playback.image_duration.get() as u64),
                    paused: playback.paused,
                    muted: playback.muted,
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
            #[cfg(feature = "audio")]
            Media::Audio { path } => match Self::load_audio(path, playback, Duration::from_millis(0)) {
                Ok((stream, sink, duration)) => Ok(Self::Audio {
                    media: media.clone(),
                    stream,
                    sink,
                    duration,
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
            #[cfg(feature = "video")]
            Media::Video { path } => match Self::load_video(path, playback) {
                Ok(video) => Ok(Self::Video {
                    media: media.clone(),
                    duration: video.duration(),
                    pipeline: get_video_pipeline(&video),
                    video,
                    position: Duration::ZERO,
                    paused: playback.paused,
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
        }
    }

    #[cfg(feature = "video")]
    fn load_video(source: &StrictPath, playback: &Playback) -> Result<iced_video_player::Video, Error> {
        let mut video = build_video(&url::Url::from_file_path(source.as_std_path_buf()?).map_err(|_| Error::Url)?)?;

        video.set_paused(playback.paused);
        mute_video(&mut video, playback.muted);
        if !playback.muted {
            set_video_volume(&mut video, playback.volume);
        }

        Ok(video)
    }

    fn load_image(source: &StrictPath) -> Result<iced::widget::image::Handle, Error> {
        let bytes = source.try_read_bytes()?;
        Ok(iced::widget::image::Handle::from_bytes(bytes))
    }

    fn load_svg(source: &StrictPath) -> Result<iced::widget::svg::Handle, Error> {
        let bytes = source.try_read_bytes()?;
        Ok(iced::widget::svg::Handle::from_memory(bytes))
    }

    fn load_gif(source: &StrictPath) -> Result<(gif::Frames, iced::widget::image::Handle), Error> {
        let bytes = source.try_read_bytes()?;
        let frames = gif::Frames::from_bytes(bytes.clone())?;
        let handle = iced::widget::image::Handle::from_bytes(bytes);
        Ok((frames, handle))
    }

    fn load_apng(source: &StrictPath) -> Result<(apng::Frames, iced::widget::image::Handle), Error> {
        let bytes = source.try_read_bytes()?;
        let frames = apng::Frames::from_bytes(bytes.clone())?;
        let handle = iced::widget::image::Handle::from_bytes(bytes);
        Ok((frames, handle))
    }

    #[cfg(feature = "audio")]
    fn load_audio(
        source: &StrictPath,
        playback: &Playback,
        position: Duration,
    ) -> Result<(rodio::OutputStream, rodio::Sink, Duration), Error> {
        use rodio::Source;

        let (stream, stream_handle) = rodio::OutputStream::try_default().map_err(|e| Error::Audio(e.to_string()))?;
        let sink = rodio::Sink::try_new(&stream_handle).map_err(|e| Error::Audio(e.to_string()))?;

        if playback.paused {
            sink.pause();
        } else {
            sink.play();
        }

        if playback.muted {
            sink.set_volume(0.0);
        } else {
            sink.set_volume(playback.volume);
        }

        let _ = sink.try_seek(position);

        let file = source.open_buffered()?;
        let source = rodio::Decoder::new(file)
            .map_err(|e| Error::Audio(e.to_string()))?
            .track_position();
        let Some(duration) = source.total_duration() else {
            return Err(Error::Audio(lang::tell::unable_to_determine_media_duration()));
        };
        sink.append(source);

        Ok((stream, sink, duration))
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

        self.set_hovered(hovered);

        if error {
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn go_idle(&mut self) {
        *self = Self::Idle {
            hovered: self.is_hovered(),
        };
    }

    pub fn restart(&mut self) {
        match self {
            Self::Idle { .. } => {}
            Self::Error { .. } => {}
            Self::Image { position, .. } => {
                *position = Duration::ZERO;
            }
            Self::Svg { position, .. } => {
                *position = Duration::ZERO;
            }
            Self::Gif { position, .. } => {
                *position = Duration::ZERO;
            }
            Self::Apng { position, .. } => {
                *position = Duration::ZERO;
            }
            #[cfg(feature = "audio")]
            Self::Audio { sink, paused, .. } => {
                let _ = sink.try_seek(Duration::ZERO);
                *paused = false;
                sink.play();
            }
            #[cfg(feature = "video")]
            Self::Video {
                video,
                position,
                paused,
                ..
            } => {
                *position = Duration::ZERO;
                seek_video(video, *position);
                *paused = false;
                video.set_paused(false);
            }
        }
    }

    pub fn media(&self) -> Option<&Media> {
        match self {
            Self::Idle { .. } => None,
            Self::Error { media, .. } => Some(media),
            Self::Image { media, .. } => Some(media),
            Self::Svg { media, .. } => Some(media),
            Self::Gif { media, .. } => Some(media),
            Self::Apng { media, .. } => Some(media),
            #[cfg(feature = "audio")]
            Self::Audio { media, .. } => Some(media),
            #[cfg(feature = "video")]
            Self::Video { media, .. } => Some(media),
        }
    }

    pub fn category(&self) -> Category {
        match self {
            Self::Idle { .. } => Category::Other,
            Self::Error { .. } => Category::Other,
            Self::Image { .. } => Category::Image,
            Self::Svg { .. } => Category::Image,
            Self::Gif { .. } => Category::Image,
            Self::Apng { .. } => Category::Image,
            #[cfg(feature = "audio")]
            Self::Audio { .. } => Category::Audio,
            #[cfg(feature = "video")]
            Self::Video { .. } => Category::Video,
        }
    }

    pub fn is_error(&self) -> bool {
        match self {
            Self::Idle { .. } => false,
            Self::Error { .. } => true,
            Self::Image { .. } => false,
            Self::Svg { .. } => false,
            Self::Gif { .. } => false,
            Self::Apng { .. } => false,
            #[cfg(feature = "audio")]
            Self::Audio { .. } => false,
            #[cfg(feature = "video")]
            Self::Video { .. } => false,
        }
    }

    pub fn is_paused(&self) -> Option<bool> {
        match self {
            Self::Idle { .. } => None,
            Self::Error { .. } => None,
            Self::Image { paused, .. } => Some(*paused),
            Self::Svg { paused, .. } => Some(*paused),
            Self::Gif { paused, .. } => Some(*paused),
            Self::Apng { paused, .. } => Some(*paused),
            #[cfg(feature = "audio")]
            Self::Audio { paused, .. } => Some(*paused),
            #[cfg(feature = "video")]
            Self::Video { paused, .. } => Some(*paused),
        }
    }

    pub fn is_muted(&self) -> Option<bool> {
        match self {
            Self::Idle { .. } => None,
            Self::Error { .. } => None,
            Self::Image { muted, .. } => Some(*muted),
            Self::Svg { muted, .. } => Some(*muted),
            Self::Gif { muted, .. } => Some(*muted),
            Self::Apng { muted, .. } => Some(*muted),
            #[cfg(feature = "audio")]
            Self::Audio { sink, .. } => Some(sink.volume() == 0.0),
            #[cfg(feature = "video")]
            Self::Video { video, .. } => Some(video.muted()),
        }
    }

    pub fn can_jump(&self) -> bool {
        match self {
            Self::Idle { .. } => false,
            Self::Error { .. } => false,
            Self::Image { .. } => false,
            Self::Svg { .. } => false,
            Self::Gif { .. } => false,
            Self::Apng { .. } => false,
            #[cfg(feature = "audio")]
            Self::Audio { .. } => true,
            #[cfg(feature = "video")]
            Self::Video { .. } => true,
        }
    }

    pub fn is_hovered(&self) -> bool {
        match self {
            Self::Idle { hovered } => *hovered,
            Self::Error { hovered, .. } => *hovered,
            Self::Image { hovered, .. } => *hovered,
            Self::Svg { hovered, .. } => *hovered,
            Self::Gif { hovered, .. } => *hovered,
            Self::Apng { hovered, .. } => *hovered,
            #[cfg(feature = "audio")]
            Self::Audio { hovered, .. } => *hovered,
            #[cfg(feature = "video")]
            Self::Video { hovered, .. } => *hovered,
        }
    }

    pub fn set_hovered(&mut self, flag: bool) {
        match self {
            Self::Idle { hovered } => {
                *hovered = flag;
            }
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
            Self::Apng { hovered, .. } => {
                *hovered = flag;
            }
            #[cfg(feature = "audio")]
            Self::Audio { hovered, .. } => {
                *hovered = flag;
            }
            #[cfg(feature = "video")]
            Self::Video { hovered, .. } => {
                *hovered = flag;
            }
        }
    }

    pub fn tick(&mut self, elapsed: Duration) -> Option<Update> {
        match self {
            Self::Idle { .. } => None,
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
                    *position += elapsed;
                }

                if *position >= *duration {
                    if *looping {
                        *position = Duration::ZERO;
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
                    *position += elapsed;
                }

                if *position >= *duration {
                    if *looping {
                        *position = Duration::ZERO;
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
                    *position += elapsed;
                }

                if *position >= *duration {
                    if *looping {
                        *position = Duration::ZERO;
                        None
                    } else {
                        Some(Update::EndOfStream)
                    }
                } else {
                    None
                }
            }
            Self::Apng {
                position,
                duration,
                paused,
                looping,
                dragging,
                ..
            } => {
                if !*paused && !*dragging {
                    *position += elapsed;
                }

                if *position >= *duration {
                    if *looping {
                        *position = Duration::ZERO;
                        None
                    } else {
                        Some(Update::EndOfStream)
                    }
                } else {
                    None
                }
            }
            #[cfg(feature = "audio")]
            Self::Audio {
                sink,
                duration,
                looping,
                ..
            } => {
                if sink.get_pos() >= *duration {
                    if *looping {
                        let _ = sink.try_seek(Duration::from_millis(0));
                        sink.play();
                    } else {
                        return Some(Update::EndOfStream);
                    }
                }
                None
            }
            #[cfg(feature = "video")]
            Self::Video { pipeline, duration, .. } => {
                // If the video is still being downloaded/written,
                // then we want to get the latest total duration.
                if let Some(clock_time) = get_video_duration(pipeline) {
                    *duration = Duration::from_nanos(clock_time.nseconds());
                }

                None
            }
        }
    }

    #[cfg(feature = "audio")]
    pub fn reload_audio(&mut self, playback: &Playback) {
        if let Self::Audio {
            media,
            stream: _,
            sink,
            duration: _,
            paused,
            looping,
            dragging,
            hovered,
            need_play_on_focus,
        } = self
        {
            let playback = playback.with_paused(*paused).with_muted(sink.volume() == 0.0);
            let position = sink.get_pos();

            *self = match Self::load_audio(media.path(), &playback, position) {
                Ok((stream, sink, duration)) => Self::Audio {
                    media: media.clone(),
                    stream,
                    sink,
                    duration,
                    paused: *paused,
                    looping: *looping,
                    dragging: *dragging,
                    hovered: *hovered,
                    need_play_on_focus: *need_play_on_focus,
                },
                Err(e) => Self::Error {
                    media: media.clone(),
                    message: e.message(),
                    hovered: false,
                },
            };
        }
    }

    fn overlay(&self, viewport: iced::Size, obscured: bool, hovered: bool) -> Overlay {
        let show = !obscured && hovered;

        match self {
            Self::Idle { .. } => Overlay {
                show,
                center_controls: false,
                top_controls: show && viewport.width > 80.0,
                bottom_controls: false,
                timestamps: false,
            },
            Self::Error { .. } => Overlay {
                show,
                center_controls: show && viewport.height > 40.0 && viewport.width > 80.0,
                top_controls: show && viewport.width > 80.0,
                bottom_controls: false,
                timestamps: false,
            },
            Self::Image { .. } | Self::Svg { .. } | Self::Gif { .. } | Self::Apng { .. } => Overlay {
                show,
                center_controls: show && viewport.height > 100.0 && viewport.width > 150.0,
                top_controls: show && viewport.width > 100.0,
                bottom_controls: show && viewport.height > 40.0,
                timestamps: show && viewport.height > 60.0 && viewport.width > 150.0,
            },
            #[cfg(feature = "audio")]
            Self::Audio { .. } => Overlay {
                show,
                center_controls: show && viewport.height > 100.0 && viewport.width > 150.0,
                top_controls: show && viewport.width > 100.0,
                bottom_controls: show && viewport.height > 40.0,
                timestamps: show && viewport.height > 60.0 && viewport.width > 150.0,
            },
            #[cfg(feature = "video")]
            Self::Video { .. } => Overlay {
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
            Self::Idle { hovered } => match event {
                Event::SetPause(_) => None,
                Event::SetLoop(_) => None,
                Event::SetMute(_) => None,
                Event::SetVolume(_) => None,
                Event::Seek(_) => None,
                Event::SeekRelative(_) => None,
                Event::SeekStop => None,
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step { .. } => None,
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
                Event::Refresh => None,
                Event::Close => Some(Update::Close),
                Event::WindowFocused => None,
                Event::WindowUnfocused => None,
            },
            Self::Error { hovered, .. } => match event {
                Event::SetPause(_) => None,
                Event::SetLoop(_) => None,
                Event::SetMute(_) => None,
                Event::SetVolume(_) => None,
                Event::Seek(_) => None,
                Event::SeekRelative(_) => None,
                Event::SeekStop => None,
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step { .. } => None,
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
                muted,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(flag) => {
                    *muted = flag;
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(_) => None,
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset.min(*duration);
                    Update::relative_position_changed(*position, *duration)
                }
                Event::SeekRelative(offset) => {
                    *position = Duration::from_secs_f64(duration.as_secs_f64() * offset);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step(step) => {
                    *position = step.compute(*position, *duration, IMAGE_STEP);
                    Some(Update::Step(step))
                }
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
                muted,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(flag) => {
                    *muted = flag;
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(_) => None,
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset.min(*duration);
                    Update::relative_position_changed(*position, *duration)
                }
                Event::SeekRelative(offset) => {
                    *position = Duration::from_secs_f64(duration.as_secs_f64() * offset);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step(step) => {
                    *position = step.compute(*position, *duration, IMAGE_STEP);
                    Some(Update::Step(step))
                }
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
                muted,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(flag) => {
                    *muted = flag;
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(_) => None,
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset.min(*duration);
                    Update::relative_position_changed(*position, *duration)
                }
                Event::SeekRelative(offset) => {
                    *position = Duration::from_secs_f64(duration.as_secs_f64() * offset);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step(step) => {
                    *position = step.compute(*position, *duration, IMAGE_STEP);
                    Some(Update::Step(step))
                }
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
            Self::Apng {
                position,
                duration,
                paused,
                muted,
                looping,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(flag) => {
                    *muted = flag;
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(_) => None,
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset.min(*duration);
                    Update::relative_position_changed(*position, *duration)
                }
                Event::SeekRelative(offset) => {
                    *position = Duration::from_secs_f64(duration.as_secs_f64() * offset);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => None,
                Event::SeekRandomRelative(_) => None,
                Event::Step(step) => {
                    *position = step.compute(*position, *duration, IMAGE_STEP);
                    Some(Update::Step(step))
                }
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

            #[cfg(feature = "audio")]
            Self::Audio {
                sink,
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
                    if flag {
                        sink.pause();
                    } else {
                        sink.play();
                    }
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    *looping = flag;
                    None
                }
                Event::SetMute(flag) => {
                    if flag {
                        sink.set_volume(0.0);
                    } else {
                        sink.set_volume(playback.volume);
                    }
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(volume) => {
                    if !playback.muted {
                        sink.set_volume(volume);
                    }
                    None
                }
                Event::Seek(offset) => {
                    *dragging = true;
                    let _ = sink.try_seek(offset);
                    Update::relative_position_changed(offset, *duration)
                }
                Event::SeekRelative(offset) | Event::SeekRandomRelative(offset) => {
                    let _ = sink.try_seek(Duration::from_secs_f64(duration.as_secs_f64() * offset));
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => {
                    use rand::Rng;
                    let position = Duration::from_secs_f64(rand::rng().random_range(0.0..duration.as_secs_f64()));
                    let _ = sink.try_seek(position);
                    Update::relative_position_changed(position, *duration)
                }
                Event::Step(step) => {
                    let position = step.compute(sink.get_pos(), *duration, AUDIO_STEP);
                    let _ = sink.try_seek(position);
                    Some(Update::Step(step))
                }
                Event::EndOfStream => (!*looping).then_some(Update::EndOfStream),
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
                        sink.play();
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        *paused = true;
                        sink.pause();
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
            #[cfg(feature = "video")]
            Self::Video {
                video,
                pipeline,
                position,
                duration,
                paused,
                dragging,
                hovered,
                need_play_on_focus,
                ..
            } => match event {
                Event::SetPause(flag) => {
                    *paused = flag;
                    video.set_paused(flag);
                    Some(Update::PauseChanged(flag))
                }
                Event::SetLoop(flag) => {
                    video.set_looping(flag);
                    None
                }
                Event::SetMute(flag) => {
                    mute_video(video, flag);
                    if !flag {
                        set_video_volume(video, playback.volume);
                    }
                    Some(Update::MuteChanged)
                }
                Event::SetVolume(volume) => {
                    if !playback.muted {
                        set_video_volume(video, volume);
                    }
                    None
                }
                Event::Seek(offset) => {
                    *dragging = true;
                    *position = offset;
                    seek_video(video, *position);
                    Update::relative_position_changed(offset, *duration)
                }
                Event::SeekRelative(offset) | Event::SeekRandomRelative(offset) => {
                    *position = Duration::from_secs_f64(duration.as_secs_f64() * offset);
                    seek_video(video, *position);
                    None
                }
                Event::SeekStop => {
                    *dragging = false;
                    None
                }
                Event::SeekRandom => {
                    use rand::Rng;
                    *position = Duration::from_secs_f64(rand::rng().random_range(0.0..duration.as_secs_f64()));
                    seek_video(video, *position);
                    Update::relative_position_changed(*position, *duration)
                }
                Event::Step(step) => {
                    *position = step.compute(*position, *duration, VIDEO_STEP);
                    seek_video(video, *position);
                    Some(Update::Step(step))
                }
                Event::EndOfStream => (!video.looping()).then_some(Update::EndOfStream),
                Event::NewFrame => {
                    if let Some(new_position) = get_video_position(pipeline, video) {
                        *position = new_position;
                    }
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
                        *paused = false;
                        video.set_paused(false);
                        *need_play_on_focus = false;
                    }
                    None
                }
                Event::WindowUnfocused => {
                    if playback.pause_on_unfocus {
                        *paused = true;
                        video.set_paused(true);
                        *need_play_on_focus = true;
                    }
                    None
                }
            },
        }
    }

    pub fn view(
        &self,
        grid_id: grid::Id,
        player_id: Id,
        selected: bool,
        obscured: bool,
        content_fit: ContentFit,
    ) -> Element {
        Responsive::new(move |viewport| {
            mouse_area(self.view_inner(grid_id, player_id, selected, obscured, content_fit, viewport))
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

    fn view_inner(
        &self,
        grid_id: grid::Id,
        player_id: Id,
        selected: bool,
        obscured: bool,
        content_fit: ContentFit,
        viewport: iced::Size,
    ) -> Element {
        match self {
            Self::Idle { hovered } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected);

                let body = Container::new("")
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new().push(space::horizontal()).push(
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .into()
            }
            Self::Error {
                media,
                message,
                hovered,
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected);

                let body = Container::new(text(message))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::OpenInNew)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
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
                );

                let center_controls = overlay.center_controls.then_some(
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
                                    .tooltip(lang::action::shuffle()),
                            ),
                    )
                    .center(Length::Fill),
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .into()
            }
            Self::Image {
                media,
                handle,
                position,
                duration,
                paused,
                muted,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = Container::new(
                    Image::new(handle)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(content_fit.into()),
                )
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill);

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let oveerlay_top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Image)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
                    Container::new(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .padding(padding::all(10.0))
                            .push(
                                button::icon(if *muted { Icon::Mute } else { Icon::VolumeHigh })
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::SetMute(!*muted),
                                    })
                                    .tooltip(if *muted {
                                        lang::action::unmute()
                                    } else {
                                        lang::action::mute()
                                    }),
                            )
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(*position, *duration)))
                            .push(Container::new(
                                iced::widget::slider(0.0..=duration.as_secs_f64(), position.as_secs_f64(), move |x| {
                                    Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(oveerlay_top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
            Self::Svg {
                media,
                handle,
                position,
                duration,
                paused,
                muted,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = Container::new(
                    Svg::new(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(content_fit.into()),
                )
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill);

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Image)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
                    Container::new(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .padding(padding::all(10.0))
                            .push(
                                button::icon(if *muted { Icon::Mute } else { Icon::VolumeHigh })
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::SetMute(!*muted),
                                    })
                                    .tooltip(if *muted {
                                        lang::action::unmute()
                                    } else {
                                        lang::action::mute()
                                    }),
                            )
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(*position, *duration)))
                            .push(Container::new(
                                iced::widget::slider(0.0..=duration.as_secs_f64(), position.as_secs_f64(), move |x| {
                                    Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
            Self::Gif {
                media,
                frames,
                handle,
                position,
                duration,
                paused,
                muted,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = {
                    let media = if *paused {
                        Container::new(
                            Image::new(handle)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .content_fit(content_fit.into()),
                        )
                    } else {
                        Container::new(
                            gif(frames)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .content_fit(content_fit.into()),
                        )
                    };

                    media
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .height(Length::Fill)
                };

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Image)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
                    Container::new(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .padding(padding::all(10.0))
                            .push(
                                button::icon(if *muted { Icon::Mute } else { Icon::VolumeHigh })
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::SetMute(!*muted),
                                    })
                                    .tooltip(if *muted {
                                        lang::action::unmute()
                                    } else {
                                        lang::action::mute()
                                    }),
                            )
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(*position, *duration)))
                            .push(Container::new(
                                iced::widget::slider(0.0..=duration.as_secs_f64(), position.as_secs_f64(), move |x| {
                                    Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
            Self::Apng {
                media,
                frames,
                handle,
                position,
                duration,
                paused,
                muted,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = {
                    let media = if *paused {
                        Container::new(
                            Image::new(handle)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .content_fit(content_fit.into()),
                        )
                    } else {
                        Container::new(
                            apng(frames)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .content_fit(content_fit.into()),
                        )
                    };

                    media
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .height(Length::Fill)
                };

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Image)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
                    Container::new(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .padding(padding::all(10.0))
                            .push(
                                button::icon(if *muted { Icon::Mute } else { Icon::VolumeHigh })
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::SetMute(!*muted),
                                    })
                                    .tooltip(if *muted {
                                        lang::action::unmute()
                                    } else {
                                        lang::action::mute()
                                    }),
                            )
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(*position, *duration)))
                            .push(Container::new(
                                iced::widget::slider(0.0..=duration.as_secs_f64(), position.as_secs_f64(), move |x| {
                                    Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
            #[cfg(feature = "audio")]
            Self::Audio {
                media,
                sink,
                duration,
                paused,
                looping,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = (!overlay.show).then_some(
                    Container::new(Icon::Music.max_control())
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .height(Length::Fill),
                );

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Music)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
                    Container::new(
                        Row::new()
                            .spacing(5)
                            .align_y(alignment::Vertical::Center)
                            .padding(padding::all(10.0))
                            .push({
                                let muted = sink.volume() == 0.0;

                                button::icon(if muted { Icon::Mute } else { Icon::VolumeHigh })
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::SetMute(!muted),
                                    })
                                    .tooltip(if muted {
                                        lang::action::unmute()
                                    } else {
                                        lang::action::mute()
                                    })
                            })
                            .push({
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
                                    })
                            })
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(sink.get_pos(), *duration)))
                            .push(Container::new(
                                iced::widget::slider(
                                    0.0..=duration.as_secs_f64(),
                                    sink.get_pos().as_secs_f64(),
                                    move |x| Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
            #[cfg(feature = "video")]
            Self::Video {
                media,
                video,
                position,
                duration,
                paused,
                dragging,
                hovered,
                ..
            } => {
                let overlay = self.overlay(viewport, obscured, *hovered || selected || *dragging);

                let body = Container::new(build_video_player(video, grid_id, player_id, content_fit))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let controls_background = overlay.show.then_some(
                    Container::new("")
                        .center(Length::Fill)
                        .class(style::Container::ModalBackground),
                );

                let top_controls = overlay.top_controls.then_some(
                    Container::new(
                        Row::new()
                            .push(
                                button::icon(Icon::Movie)
                                    .on_press(Message::OpenDir {
                                        path: media.path().clone(),
                                    })
                                    .tooltip(media.path().render()),
                            )
                            .push(space::horizontal())
                            .push(
                                button::icon(Icon::Refresh)
                                    .on_press(Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Refresh,
                                    })
                                    .tooltip(lang::action::shuffle()),
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
                );

                let center_controls = overlay.center_controls.then_some(
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
                );

                let bottom_controls = overlay.bottom_controls.then_some(
                    Container::new(
                        Column::new()
                            .padding(padding::left(10).right(10).bottom(5))
                            .push(space::vertical())
                            .push(overlay.timestamps.then_some(timestamps(*position, *duration)))
                            .push(Container::new(
                                iced::widget::slider(0.0..=duration.as_secs_f64(), position.as_secs_f64(), move |x| {
                                    Message::Player {
                                        grid_id,
                                        player_id,
                                        event: Event::Seek(Duration::from_secs_f64(x)),
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
                );

                Stack::new()
                    .push(body)
                    .push(controls_background)
                    .push(top_controls)
                    .push(center_controls)
                    .push(bottom_controls)
                    .into()
            }
        }
    }
}
