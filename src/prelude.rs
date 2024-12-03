use std::{path::PathBuf, sync::Mutex};

use std::sync::LazyLock;

use crate::path::CommonPath;
pub use crate::path::StrictPath;

pub static VERSION: LazyLock<&'static str> =
    LazyLock::new(|| option_env!("MADAMIRU_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")));
pub static USER_AGENT: LazyLock<String> = LazyLock::new(|| format!("madamiru/{}", *VERSION));
pub static CANONICAL_VERSION: LazyLock<(u32, u32, u32)> = LazyLock::new(|| {
    let version_parts: Vec<u32> = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|x| x.parse().unwrap_or(0))
        .collect();
    if version_parts.len() != 3 {
        (0, 0, 0)
    } else {
        (version_parts[0], version_parts[1], version_parts[2])
    }
});

pub type AnyError = Box<dyn std::error::Error>;

pub const APP_DIR_NAME: &str = "com.mtkennerly.madamiru";
#[allow(unused)]
pub const LINUX_APP_ID: &str = "com.mtkennerly.madamiru";
const PORTABLE_FLAG_FILE_NAME: &str = "madamiru.portable";

pub static STEAM_DECK: LazyLock<bool> =
    LazyLock::new(|| cfg!(target_os = "linux") && StrictPath::new("/home/deck".to_string()).exists());

pub static CONFIG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

#[allow(unused)]
pub const ENV_DEBUG: &str = "MADAMIRU_DEBUG";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    ConfigInvalid { why: String },
    NoMediaFound,
    UnableToOpenDir(StrictPath),
    UnableToOpenUrl(String),
}

pub fn app_dir() -> StrictPath {
    if let Some(dir) = CONFIG_DIR.lock().unwrap().as_ref() {
        return StrictPath::from(dir.clone());
    }

    if let Ok(mut flag) = std::env::current_exe() {
        flag.pop();
        flag.push(PORTABLE_FLAG_FILE_NAME);
        if flag.exists() {
            flag.pop();
            return StrictPath::from(flag);
        }
    }

    StrictPath::new(format!("{}/{}", CommonPath::Config.get().unwrap(), APP_DIR_NAME))
}

pub fn timestamp_mmss(seconds: u64) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;

    format!("{minutes:02}:{seconds:02}")
}

pub fn timestamp_hhmmss(mut seconds: u64) -> String {
    let hours = seconds / (60 * 60);
    seconds %= 60 * 60;

    let minutes = seconds / 60;
    seconds %= 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    #[test_case(0, "00:00")]
    #[test_case(9, "00:09")]
    #[test_case(10, "00:10")]
    #[test_case(60, "01:00")]
    #[test_case(60 * 60 + 1, "60:01")]
    pub fn can_format_timestamp_mmss(seconds: u64, formatted: &str) {
        assert_eq!(formatted, timestamp_mmss(seconds));
    }

    #[test_case(0, "00:00:00")]
    #[test_case(9, "00:00:09")]
    #[test_case(10, "00:00:10")]
    #[test_case(60, "00:01:00")]
    #[test_case(60 * 60, "01:00:00")]
    #[test_case(60 * 60 + 1, "01:00:01")]
    #[test_case(60 * 60 * 2 - 1, "01:59:59")]
    pub fn can_format_timestamp_hhmmss(seconds: u64, formatted: &str) {
        assert_eq!(formatted, timestamp_hhmmss(seconds));
    }
}
