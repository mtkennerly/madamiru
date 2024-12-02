use std::collections::HashSet;

use crate::path::StrictPath;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Source {
    pub path: StrictPath,
}

impl Source {
    pub fn new(path: StrictPath) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Media {
    Image { path: StrictPath },
    Gif { path: StrictPath },
    Video { path: StrictPath },
}

impl Media {
    pub fn path(&self) -> &StrictPath {
        match self {
            Self::Gif { path } => path,
            Self::Image { path } => path,
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

                match info.mime_type() {
                    "video/mp4" | "video/mpeg" | "video/quicktime" | "video/webm" | "video/x-flv" | "video/x-m4v"
                    | "video/x-matroska" | "video/x-ms-wmv" | "video/x-msvideo" => {
                        Some(Self::Video { path: path.clone() })
                    }
                    "image/bmp"
                    | "image/jpeg"
                    | "image/png"
                    | "image/tiff"
                    | "image/vnd.microsoft.icon"
                    | "image/webp" => Some(Self::Image { path: path.clone() }),
                    "image/gif" => Some(Self::Gif { path: path.clone() }),
                    _ => None,
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

#[derive(Debug, Default, Clone)]
pub struct Collection {
    media: HashSet<Media>,
}

impl Collection {
    pub fn new(media: HashSet<Media>) -> Self {
        Self { media }
    }

    pub fn find(sources: &[Source]) -> Self {
        let mut media = HashSet::new();

        for source in sources {
            media.extend(Self::find_in_source(&source.path));
        }

        Self::new(media)
    }

    fn find_in_source(path: &StrictPath) -> HashSet<Media> {
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
            let mut media = HashSet::new();
            for path in path.glob() {
                media.extend(Self::find_in_source(&path));
            }
            media
        }
    }

    pub fn new_first(
        &self,
        take: usize,
        old: HashSet<&StrictPath>,
        ignored: &HashSet<StrictPath>,
    ) -> Option<Vec<Media>> {
        use rand::seq::SliceRandom;

        let mut media: Vec<_> = self.media.iter().collect();
        media.shuffle(&mut rand::thread_rng());

        let media: Vec<_> = media
            .iter()
            .filter(|media| !ignored.contains(media.path()) && !old.contains(media.path()))
            .chain(
                media
                    .iter()
                    .filter(|media| !ignored.contains(media.path()) && old.contains(media.path())),
            )
            .take(take)
            .cloned()
            .cloned()
            .collect();

        (!media.is_empty()).then_some(media)
    }

    pub fn one_new(&self, old: HashSet<&StrictPath>, ignored: &HashSet<StrictPath>) -> Option<Media> {
        use rand::seq::SliceRandom;

        let mut media: Vec<_> = self.media.iter().collect();
        media.shuffle(&mut rand::thread_rng());

        media
            .into_iter()
            .find(|media| !ignored.contains(media.path()) && !old.contains(media.path()))
            .cloned()
    }
}
