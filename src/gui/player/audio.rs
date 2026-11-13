use super::Error;
use rodio::{ChannelCount, SampleRate, Source, source::SeekError};
use std::{
    fs::File,
    num::NonZero,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use symphonia::{
    core::{
        audio::SampleBuffer,
        codecs::{CODEC_TYPE_NULL, CodecRegistry, DecoderOptions},
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
        units::Time,
    },
    default::{get_probe, register_enabled_codecs},
};
use symphonia_adapter_libopus::OpusDecoder;

pub struct CustomSymphoniaSource {
    path: PathBuf,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_buf: Option<SampleBuffer<f32>>,
    buf_pos: usize,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Option<Duration>,
}

impl CustomSymphoniaSource {
    fn init_pipeline(
        path: &Path,
    ) -> Result<
        (
            Box<dyn FormatReader>,
            Box<dyn symphonia::core::codecs::Decoder>,
            u32,
            ChannelCount,
            SampleRate,
            Option<Duration>,
        ),
        Error,
    > {
        let file = File::open(path).map_err(|e| Error::Audio(e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| Error::Audio(e.to_string()))?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| Error::Audio("No supported audio track".into()))?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let mut registry = CodecRegistry::new();
        register_enabled_codecs(&mut registry);
        registry.register_all::<OpusDecoder>();

        let decoder = registry
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| Error::Audio(e.to_string()))?;

        let sample_rate = NonZero::new(codec_params.sample_rate.unwrap_or(48_000))
            .ok_or_else(|| Error::Audio("Invalid sample rate".into()))?;
        let channels = NonZero::new(codec_params.channels.map(|c| c.count() as u16).unwrap_or(2))
            .ok_or_else(|| Error::Audio("Invalid channel count".into()))?;

        let duration = codec_params.n_frames.and_then(|frames| {
            codec_params.time_base.map(|tb| {
                let t = tb.calc_time(frames);
                Duration::from_secs(t.seconds) + Duration::from_secs_f64(t.frac)
            })
        });

        Ok((format, decoder, track_id, channels, sample_rate, duration))
    }

    pub fn open(path: &Path) -> Result<Self, Error> {
        let (format, decoder, track_id, channels, sample_rate, duration) = Self::init_pipeline(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            format,
            decoder,
            track_id,
            sample_buf: None,
            buf_pos: 0,
            channels,
            sample_rate,
            duration,
        })
    }
}

impl Iterator for CustomSymphoniaSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(buf) = &self.sample_buf
                && self.buf_pos < buf.samples().len()
            {
                let sample = buf.samples()[self.buf_pos];
                self.buf_pos += 1;
                return Some(sample);
            }

            let packet = loop {
                match self.format.next_packet() {
                    Ok(pkt) if pkt.track_id() == self.track_id => break pkt,
                    Ok(_) => continue,
                    Err(_) => return None,
                }
            };

            if let Ok(decoded) = self.decoder.decode(&packet) {
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;
                let mut buf = SampleBuffer::<f32>::new(duration, spec);
                buf.copy_interleaved_ref(decoded);
                self.buf_pos = 0;
                self.sample_buf = Some(buf);
            }
        }
    }
}

impl Source for CustomSymphoniaSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        let seek_to = |track_id: u32| SeekTo::Time {
            time: Time::new(pos.as_secs(), pos.subsec_nanos() as f64 / 1_000_000_000.0),
            track_id: Some(track_id),
        };

        let mode = if pos.is_zero() {
            SeekMode::Coarse
        } else {
            SeekMode::Accurate
        };

        let seek_ok = self.format.seek(mode, seek_to(self.track_id)).is_ok()
            || self.format.seek(SeekMode::Coarse, seek_to(self.track_id)).is_ok();

        if seek_ok {
            self.decoder.reset();
            self.sample_buf = None;
            self.buf_pos = 0;
            return Ok(());
        }

        if let Ok((format, decoder, track_id, _, _, _)) = Self::init_pipeline(&self.path) {
            self.format = format;
            self.decoder = decoder;
            self.track_id = track_id;
            self.sample_buf = None;
            self.buf_pos = 0;

            if !pos.is_zero() {
                let _ = self.format.seek(SeekMode::Coarse, seek_to(self.track_id));
            }
            return Ok(());
        }

        Err(SeekError::NotSupported {
            underlying_source: "Symphonia",
        })
    }
}

pub struct KeepAlive<S> {
    inner: S,
    finished: Arc<AtomicBool>,
}

impl<S> KeepAlive<S> {
    pub fn new(inner: S) -> (Self, Arc<AtomicBool>) {
        let finished = Arc::new(AtomicBool::new(false));
        (
            Self {
                inner,
                finished: finished.clone(),
            },
            finished,
        )
    }
}

impl<S: Source> Iterator for KeepAlive<S> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished.load(Ordering::Acquire) {
            return Some(0.0);
        }

        match self.inner.next() {
            Some(sample) => Some(sample),
            None => {
                self.finished.store(true, Ordering::Release);
                Some(0.0)
            }
        }
    }
}

impl<S: Source> Source for KeepAlive<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> NonZero<u16> {
        self.inner.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        self.finished.store(false, Ordering::Release);
        Ok(())
    }
}
