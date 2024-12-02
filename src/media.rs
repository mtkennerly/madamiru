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
                        Some(Media::Video { path: path.clone() })
                    }
                    "image/bmp"
                    | "image/jpeg"
                    | "image/png"
                    | "image/tiff"
                    | "image/vnd.microsoft.icon"
                    | "image/webp" => Some(Media::Image { path: path.clone() }),
                    "image/gif" => Some(Media::Gif { path: path.clone() }),
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

fn find_media_in_source(source: &Source) -> Vec<Media> {
    let path = &source.path;

    if path.is_file() {
        match Media::identify(path) {
            Some(source) => vec![source],
            None => vec![],
        }
    } else if path.is_dir() {
        path.joined("*")
            .glob()
            .into_iter()
            .filter(|x| x.is_file())
            .filter_map(|path| Media::identify(&path))
            .collect()
    } else {
        path.glob()
            .into_iter()
            .filter(|x| x.is_file())
            .filter_map(|path| Media::identify(&path))
            .collect()
    }
}

pub fn find_media(sources: &[Source], max: usize) -> Option<Vec<Media>> {
    use rand::seq::SliceRandom;

    let mut media = vec![];

    for path in sources {
        media.extend(find_media_in_source(path));
    }

    media.shuffle(&mut rand::thread_rng());
    (!media.is_empty()).then(|| media.into_iter().take(max).collect())
}

pub fn find_new_media_first(
    sources: &[Source],
    max: usize,
    take: usize,
    old: HashSet<&StrictPath>,
) -> Option<Vec<Media>> {
    let media = find_media(sources, max)?;
    Some(
        media
            .iter()
            .filter(|media| !old.contains(media.path()))
            .chain(media.iter().filter(|source| old.contains(source.path())))
            .take(take)
            .cloned()
            .collect(),
    )
}

pub fn find_new_media(sources: &[Source], max: usize, old: HashSet<&StrictPath>) -> Option<Media> {
    let media = find_media(sources, max)?;
    media.iter().find(|media| !old.contains(media.path())).cloned()
}
