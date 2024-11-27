use std::collections::HashSet;

use crate::path::StrictPath;

fn is_supported(path: &StrictPath) -> bool {
    let inferrable = match path.as_std_path_buf() {
        Ok(pb) => pb,
        Err(e) => {
            log::error!("Unable to parse path: {path:?} | {e:?}");
            return false;
        }
    };

    match infer::get_from_path(inferrable) {
        Ok(Some(info)) => {
            log::info!("Inferred file type '{}': {path:?}", info.mime_type());

            matches!(
                info.mime_type(),
                "video/mp4"
                    | "video/mpeg"
                    | "video/quicktime"
                    | "video/webm"
                    | "video/x-flv"
                    | "video/x-m4v"
                    | "video/x-matroska"
                    | "video/x-ms-wmv"
                    | "video/x-msvideo"
            )
        }
        Ok(None) => {
            log::info!("Did not infer any file type: {path:?}");
            false
        }
        Err(e) => {
            log::error!("Error inferring file type: {path:?} | {e:?}");
            false
        }
    }
}

fn find_videos_in_path(path: &StrictPath) -> Vec<StrictPath> {
    if path.is_file() {
        if is_supported(path) {
            vec![path.clone()]
        } else {
            vec![]
        }
    } else if path.is_dir() {
        path.joined("*")
            .glob()
            .into_iter()
            .filter(|x| x.is_file())
            .filter(is_supported)
            .collect()
    } else {
        path.glob()
            .into_iter()
            .filter(|x| x.is_file())
            .filter(is_supported)
            .collect()
    }
}

pub fn find_videos(sources: &[StrictPath], max: usize) -> Option<Vec<StrictPath>> {
    use rand::seq::SliceRandom;

    let mut videos = vec![];

    for path in sources {
        videos.extend(find_videos_in_path(path));
    }

    videos.shuffle(&mut rand::thread_rng());
    (!videos.is_empty()).then(|| videos.into_iter().take(max).collect())
}

pub fn find_new_videos_first(
    sources: &[StrictPath],
    max: usize,
    take: usize,
    old: HashSet<&StrictPath>,
) -> Option<Vec<StrictPath>> {
    let videos = find_videos(sources, max)?;
    Some(
        videos
            .iter()
            .filter(|video| !old.contains(video))
            .chain(videos.iter().filter(|video| old.contains(video)))
            .take(take)
            .cloned()
            .collect(),
    )
}

pub fn find_new_video(sources: &[StrictPath], max: usize, old: HashSet<&StrictPath>) -> Option<StrictPath> {
    let videos = find_videos(sources, max)?;
    videos.iter().find(|video| !old.contains(video)).cloned()
}
