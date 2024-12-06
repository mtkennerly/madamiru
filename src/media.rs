use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::{lang, path::StrictPath};

#[derive(Debug, Clone, Copy)]
pub enum RefreshContext {
    Launch,
    Edit,
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Media {
    Image { path: StrictPath },
    Svg { path: StrictPath },
    Gif { path: StrictPath },
    Video { path: StrictPath },
}

impl Media {
    pub fn path(&self) -> &StrictPath {
        match self {
            Self::Image { path } => path,
            Self::Svg { path } => path,
            Self::Gif { path } => path,
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

        match infer::get_from_path(inferrable) {
            Ok(Some(info)) => {
                log::info!("Inferred file type '{}': {path:?}", info.mime_type());

                let extension = path.file_extension().map(|x| x.to_lowercase());

                match info.mime_type() {
                    "video/mp4" | "video/quicktime" | "video/webm" | "video/x-m4v" | "video/x-matroska"
                    | "video/x-msvideo" => Some(Self::Video { path: path.clone() }),
                    "image/bmp"
                    | "image/jpeg"
                    | "image/png"
                    | "image/tiff"
                    | "image/vnd.microsoft.icon"
                    | "image/webp" => Some(Self::Image { path: path.clone() }),
                    "image/gif" => Some(Self::Gif { path: path.clone() }),
                    _ => match extension.as_deref() {
                        Some("svg") => Some(Self::Svg { path: path.clone() }),
                        _ => None,
                    },
                }
            }
            Ok(None) => {
                log::info!("Did not infer any file type: {path:?}");
                None
            }
            Err(e) => {
                log::error!("Error inferring file type: {path:?} | {e:?}");
                None
            }
        }
    }
}

pub type SourceMap = HashMap<Source, HashSet<Media>>;

#[derive(Debug, Default, Clone)]
pub struct Collection {
    media: SourceMap,
    errored: HashSet<Media>,
}

impl Collection {
    pub fn mark_error(&mut self, media: &Media) {
        self.errored.insert(media.clone());
    }

    pub fn is_outdated(&self, media: &Media, sources: &[Source]) -> bool {
        sources
            .iter()
            .filter_map(|source| self.media.get(source))
            .all(|known| !known.contains(media))
    }

    pub fn find(sources: &[Source]) -> SourceMap {
        let mut media = SourceMap::new();

        for source in sources {
            media.insert(source.clone(), Self::find_in_source(source));
        }

        media
    }

    fn find_in_source(source: &Source) -> HashSet<Media> {
        match source {
            Source::Path { path } => {
                if path.is_file() {
                    match Media::identify(path) {
                        Some(source) => HashSet::from_iter([source]),
                        None => HashSet::new(),
                    }
                } else if path.is_dir() {
                    path.joined("*")
                        .glob()
                        .into_iter()
                        .filter(|x| x.is_file())
                        .filter_map(|path| Media::identify(&path))
                        .collect()
                } else {
                    HashSet::new()
                }
            }
            Source::Glob { pattern } => {
                let mut media = HashSet::new();
                for path in StrictPath::new(pattern).glob() {
                    media.extend(Self::find_in_source(&Source::new_path(path)));
                }
                media
            }
        }
    }

    pub fn replace(&mut self, new: SourceMap) {
        self.media = new;
    }

    pub fn new_first(&self, sources: &[Source], take: usize, old: HashSet<&Media>) -> Option<Vec<Media>> {
        use rand::seq::SliceRandom;

        let mut media: Vec<_> = sources
            .iter()
            .filter_map(|source| self.media.get(source))
            .flatten()
            .unique()
            .collect();
        media.shuffle(&mut rand::thread_rng());

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
        media.shuffle(&mut rand::thread_rng());

        media
            .into_iter()
            .find(|media| !self.errored.contains(media) && !old.contains(media))
            .cloned()
    }
}
