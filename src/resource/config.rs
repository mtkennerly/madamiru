use std::num::NonZeroUsize;

use crate::{
    lang::{self, Language},
    prelude::{app_dir, Error, StrictPath},
    resource::{ResourceFile, SaveableResourceFile},
};

#[derive(Debug, Clone)]
pub enum Event {
    Theme(Theme),
    Language(Language),
    CheckRelease(bool),
    MaxInitialMediaRaw(String),
    ImageDurationRaw(String),
    PauseWhenWindowLosesFocus(bool),
}

/// Settings for `config.yaml`
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Config {
    pub release: Release,
    pub language: Language,
    pub theme: Theme,
    pub playback: Playback,
}

impl ResourceFile for Config {
    const FILE_NAME: &'static str = "config.yaml";
}

impl SaveableResourceFile for Config {}

impl Config {
    fn file_archived_invalid() -> StrictPath {
        app_dir().joined("config.invalid.yaml")
    }

    pub fn load() -> Result<Self, Error> {
        ResourceFile::load().map_err(|e| Error::ConfigInvalid { why: format!("{}", e) })
    }

    pub fn archive_invalid() -> Result<(), Box<dyn std::error::Error>> {
        Self::path().move_to(&Self::file_archived_invalid())?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Release {
    /// Whether to check for new releases.
    /// If enabled, the application will check at most once every 24 hours.
    pub check: bool,
}

impl Default for Release {
    fn default() -> Self {
        Self { check: true }
    }
}

/// Visual theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub const ALL: &'static [Self] = &[Self::Light, Self::Dark];
}

impl ToString for Theme {
    fn to_string(&self) -> String {
        lang::theme_name(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Playback {
    #[serde(skip)]
    pub paused: bool,
    /// Whether all players are muted.
    pub muted: bool,
    /// How many players to show at most by default.
    pub max_initial_media: NonZeroUsize,
    /// How long to show images, in seconds.
    pub image_duration: NonZeroUsize,
    /// Whether to pause when window loses focus.
    pub pause_on_unfocus: bool,
}

impl Playback {
    pub fn with_muted_maybe(&self, muted: Option<bool>) -> Self {
        Self {
            muted: muted.unwrap_or(self.muted),
            ..self.clone()
        }
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            paused: false,
            muted: false,
            max_initial_media: NonZeroUsize::new(4).unwrap(),
            image_duration: NonZeroUsize::new(10).unwrap(),
            pause_on_unfocus: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub const ALL: &[Self] = &[Self::Horizontal, Self::Vertical];
}

impl ToString for Orientation {
    fn to_string(&self) -> String {
        match self {
            Self::Horizontal => lang::state::horizontal(),
            Self::Vertical => lang::state::vertical(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum OrientationLimit {
    #[default]
    Automatic,
    Fixed(NonZeroUsize),
}

impl OrientationLimit {
    pub const DEFAULT_FIXED: NonZeroUsize = NonZeroUsize::new(4).unwrap();

    pub fn is_fixed(&self) -> bool {
        match self {
            Self::Automatic => false,
            Self::Fixed(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn can_parse_minimal_config() {
        let config = Config::load_from_string("{}").unwrap();

        assert_eq!(Config::default(), config);
    }

    #[test]
    fn can_parse_optional_fields_when_present_in_config() {
        let config = Config::load_from_string(
            r#"
                release:
                  check: false
                theme: Light
                playback:
                  muted: true
                  max_initial_media: 1
                  image_duration: 2
                  pause_on_unfocus: true
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                release: Release { check: false },
                language: Language::English,
                theme: Theme::Light,
                playback: Playback {
                    paused: false,
                    muted: true,
                    max_initial_media: NonZeroUsize::new(1).unwrap(),
                    image_duration: NonZeroUsize::new(2).unwrap(),
                    pause_on_unfocus: true,
                },
            },
            config,
        );
    }

    #[test]
    fn can_be_serialized() {
        assert_eq!(
            r#"
---
release:
  check: true
language: en-US
theme: Dark
playback:
  muted: false
  max_initial_media: 4
  image_duration: 10
  pause_on_unfocus: false
"#
            .trim(),
            serde_yaml::to_string(&Config::default()).unwrap().trim(),
        );
    }
}
