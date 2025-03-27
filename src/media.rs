use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::{lang, path::StrictPath};

pub const MAX_INITIAL: usize = 1;

mod placeholder {
    pub const PLAYLIST: &str = "<playlist>";
}

pub fn fill_placeholders_in_path(path: &StrictPath, playlist: Option<&StrictPath>) -> StrictPath {
    let playlist = playlist
        .and_then(|x| x.parent_if_file().ok())
        .unwrap_or_else(StrictPath::cwd);
    path.replace_raw_prefix(placeholder::PLAYLIST, playlist.raw_ref())
}

#[derive(Debug, Clone, Copy)]
pub enum RefreshContext {
    Launch,
    Edit,
    Playlist,
    Automatic,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Path { path: StrictPath },
    Glob { pattern: String },
}

impl Source {
    pub fn new_path(path: StrictPath) -> Self {
        Self::Path { path }
    }

    pub fn new_glob(pattern: String) -> Self {
        Self::Glob { pattern }
    }

    pub fn kind(&self) -> SourceKind {
        match self {
            Self::Path { .. } => SourceKind::Path,
            Self::Glob { .. } => SourceKind::Glob,
        }
    }

    pub fn set_kind(&mut self, kind: SourceKind) {
        let raw = self.raw();

        match kind {
            SourceKind::Path => {
                *self = Self::new_path(StrictPath::new(raw));
            }
            SourceKind::Glob => {
                *self = Self::new_glob(raw.to_string());
            }
        }
    }

    pub fn path(&self) -> Option<&StrictPath> {
        match self {
            Self::Path { path } => Some(path),
            Self::Glob { .. } => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Path { path } => path.raw_ref().trim().is_empty(),
            Self::Glob { pattern } => pattern.trim().is_empty(),
        }
    }

    pub fn raw(&self) -> &str {
        match self {
            Self::Path { path } => path.raw_ref(),
            Self::Glob { pattern } => pattern,
        }
    }

    pub fn reset(&mut self, raw: String) {
        match self {
            Self::Path { path } => {
                path.reset(raw);
            }
            Self::Glob { pattern } => {
                *pattern = raw;
            }
        }
    }

    pub fn fill_placeholders(&self, playlist: &StrictPath) -> Self {
        match self {
            Self::Path { path } => Self::Path {
                path: fill_placeholders_in_path(path, Some(playlist)),
            },
            Self::Glob { pattern } => Self::Glob {
                pattern: match pattern.strip_prefix(placeholder::PLAYLIST) {
                    Some(suffix) => format!("{}{}", playlist.render(), suffix),
                    None => pattern.clone(),
                },
            },
        }
    }

    pub fn has_playlist_placeholder(&self) -> bool {
        self.raw().contains(placeholder::PLAYLIST)
    }
}

impl Default for Source {
    fn default() -> Self {
        Self::Path {
            path: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceKind {
    #[default]
    Path,
    Glob,
}

impl SourceKind {
    pub const ALL: &'static [Self] = &[Self::Path, Self::Glob];
}

impl ToString for SourceKind {
    fn to_string(&self) -> String {
        match self {
            Self::Path => lang::thing::path(),
            Self::Glob => lang::thing::glob(),
        }
    }
}

#[derive(Debug)]
enum Mime {
    /// From the `infer` crate.
    /// Based on magic bytes without system dependencies, but not exhaustive.
    Pure(&'static str),
    /// From the `tree_magic_mini` crate.
    /// Uses the system's shared database on Linux and Mac,
    /// but not viable for Windows without bundling GPL data.
    #[allow(unused)]
    Database(&'static str),
    /// From the `mime_guess` crate.
    /// Guesses based on the file extension.
    Extension(mime_guess::Mime),
}

impl Mime {
    fn essence(&self) -> &str {
        match self {
            Self::Pure(raw) => raw,
            Self::Database(raw) => raw,
            Self::Extension(mime) => mime.essence_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Media {
    Image {
        path: StrictPath,
    },
    Svg {
        path: StrictPath,
    },
    Gif {
        path: StrictPath,
    },
    #[cfg(feature = "audio")]
    Audio {
        path: StrictPath,
    },
    #[cfg(feature = "video")]
    Video {
        path: StrictPath,
    },
}

impl Media {
    pub fn path(&self) -> &StrictPath {
        match self {
            Self::Image { path } => path,
            Self::Svg { path } => path,
            Self::Gif { path } => path,
            #[cfg(feature = "audio")]
            Self::Audio { path } => path,
            #[cfg(feature = "video")]
            Self::Video { path } => path,
        }
    }

    fn identify(path: &StrictPath) -> Option<Self> {
        let inferrable = match path.as_std_path_buf() {
            Ok(pb) => pb,
            Err(e) => {
                log::error!("Unable to parse path: {path:?} | {e:?}");
                return None;
            }
        };

        #[allow(clippy::unnecessary_lazy_evaluations)]
        let mime = infer::get_from_path(&inferrable)
            .map_err(|e| {
                log::error!("Error inferring file type: {path:?} | {e:?}");
                e
            })
            .ok()
            .flatten()
            .map(|x| Mime::Pure(x.mime_type()))
            .or_else(|| {
                #[cfg(target_os = "windows")]
                {
                    None
                }
                #[cfg(not(target_os = "windows"))]
                {
                    tree_magic_mini::from_filepath(&inferrable).map(Mime::Database)
                }
            })
            .or_else(|| mime_guess::from_path(&inferrable).first().map(Mime::Extension));

        log::debug!("Inferred file type '{:?}': {path:?}", mime);

        mime.and_then(|mime| {
            let mime = mime.essence();

            #[cfg(feature = "video")]
            if mime.starts_with("video/") {
                // The exact formats supported will depend on the user's GStreamer plugins,
                // so just go ahead and try it. Some that work by default on Windows:
                // * video/mp4
                // * video/mpeg
                // * video/quicktime
                // * video/webm
                // * video/x-m4v
                // * video/x-matroska
                // * video/x-msvideo
                return Some(Self::Video {
                    path: path.normalized(),
                });
            }

            let extension = path.file_extension().map(|x| x.to_lowercase());

            match mime {
                #[cfg(feature = "audio")]
                "audio/mpeg" | "audio/m4a" | "audio/x-flac" | "audio/x-wav" => Some(Self::Audio {
                    path: path.normalized(),
                }),
                "image/bmp" | "image/jpeg" | "image/png" | "image/tiff" | "image/vnd.microsoft.icon" | "image/webp" => {
                    Some(Self::Image {
                        path: path.normalized(),
                    })
                }
                "image/gif" => Some(Self::Gif {
                    path: path.normalized(),
                }),
                "image/svg+xml" => Some(Self::Svg {
                    path: path.normalized(),
                }),
                "text/xml" if extension.is_some_and(|ext| ext == "svg") => Some(Self::Svg {
                    path: path.normalized(),
                }),
                _ => None,
            }
        })
    }
}

pub type SourceMap = HashMap<Source, HashSet<Media>>;

#[derive(Debug, Default, Clone)]
pub struct Collection {
    media: SourceMap,
    errored: HashSet<Media>,
}

impl Collection {
    pub fn clear(&mut self) {
        self.media.clear();
    }

    pub fn mark_error(&mut self, media: &Media) {
        self.errored.insert(media.clone());
    }

    pub fn is_outdated(&self, media: &Media, sources: &[Source]) -> bool {
        if sources.is_empty() {
            return true;
        }

        sources
            .iter()
            .filter_map(|source| self.media.get(source))
            .all(|known| !known.contains(media))
    }

    pub fn find(sources: &[Source], playlist: Option<StrictPath>) -> SourceMap {
        let mut media = SourceMap::new();
        let playlist = playlist
            .and_then(|x| x.parent_if_file().ok())
            .unwrap_or_else(StrictPath::cwd);

        for source in sources {
            media.insert(source.clone(), Self::find_in_source(source, Some(&playlist)));
        }

        media
    }

    fn find_in_source(source: &Source, playlist: Option<&StrictPath>) -> HashSet<Media> {
        log::debug!("Finding media in source: {source:?}, playlist: {playlist:?}");

        let source = match playlist {
            Some(playlist) => source.fill_placeholders(playlist),
            None => source.clone(),
        };
        log::debug!("Source with placeholders filled: {source:?}");

        match &source {
            Source::Path { path } => {
                if path.is_file() {
                    log::debug!("Source is file");
                    match Media::identify(path) {
                        Some(source) => HashSet::from_iter([source]),
                        None => HashSet::new(),
                    }
                } else if path.is_dir() {
                    log::debug!("Source is directory");
                    path.joined("*")
                        .glob()
                        .into_iter()
                        .filter(|x| x.is_file())
                        .filter_map(|path| Media::identify(&path))
                        .collect()
                } else if path.is_symlink() {
                    log::debug!("Source is symlink");
                    match path.interpreted() {
                        Ok(path) => Self::find_in_source(&Source::new_path(path), None),
                        Err(_) => HashSet::new(),
                    }
                } else {
                    log::debug!("Source is unknown path");
                    HashSet::new()
                }
            }
            Source::Glob { pattern } => {
                log::debug!("Source is glob");
                let mut media = HashSet::new();
                for path in StrictPath::new(pattern).glob() {
                    media.extend(Self::find_in_source(&Source::new_path(path), None));
                }
                media
            }
        }
    }

    pub fn update(&mut self, new: SourceMap, context: RefreshContext) {
        match context {
            RefreshContext::Launch | RefreshContext::Playlist | RefreshContext::Automatic => {
                self.media = new;
            }
            RefreshContext::Edit => {
                self.media.extend(new);
            }
        }
    }

    pub fn new_first(&self, sources: &[Source], take: usize, old: HashSet<&Media>) -> Option<Vec<Media>> {
        use rand::seq::SliceRandom;

        let mut media: Vec<_> = sources
            .iter()
            .filter_map(|source| self.media.get(source))
            .flatten()
            .unique()
            .collect();
        media.shuffle(&mut rand::rng());

        let media: Vec<_> = media
            .iter()
            .filter(|media| !self.errored.contains(media) && !old.contains(*media))
            .chain(
                media
                    .iter()
                    .filter(|media| !self.errored.contains(media) && old.contains(*media)),
            )
            .take(take)
            .cloned()
            .cloned()
            .collect();

        (!media.is_empty()).then_some(media)
    }

    pub fn one_new(&self, sources: &[Source], old: HashSet<&Media>) -> Option<Media> {
        use rand::seq::SliceRandom;

        let mut media: Vec<_> = sources
            .iter()
            .filter_map(|source| self.media.get(source))
            .flatten()
            .unique()
            .collect();
        media.shuffle(&mut rand::rng());

        media
            .into_iter()
            .find(|media| !self.errored.contains(media) && !old.contains(media))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn can_fill_placeholders_in_path_with_match() {
        let source = Source::new_path(StrictPath::new(format!("{}/foo", placeholder::PLAYLIST)));
        let playlist = StrictPath::new("/tmp");
        let filled = Source::new_path(StrictPath::new("/tmp/foo"));
        assert_eq!(filled, source.fill_placeholders(&playlist))
    }

    #[test]
    fn can_fill_placeholders_in_path_without_match() {
        let source = Source::new_path(StrictPath::new(format!("/{}/foo", placeholder::PLAYLIST)));
        let playlist = StrictPath::new("/tmp");
        assert_eq!(source, source.fill_placeholders(&playlist))
    }

    #[test]
    fn can_fill_placeholders_in_glob_with_match() {
        let source = Source::new_glob(format!("{}/foo", placeholder::PLAYLIST));
        let playlist = StrictPath::new("/tmp");
        let filled = Source::new_glob("/tmp/foo".to_string());
        assert_eq!(filled, source.fill_placeholders(&playlist))
    }

    #[test]
    fn can_fill_placeholders_in_glob_without_match() {
        let source = Source::new_glob(format!("/{}/foo", placeholder::PLAYLIST));
        let playlist = StrictPath::new("/tmp");
        assert_eq!(source, source.fill_placeholders(&playlist))
    }
}
