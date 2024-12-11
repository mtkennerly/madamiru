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
    ImageDurationRaw(String),
    PauseWhenWindowLosesFocus(bool),
    ConfirmWhenDiscardingUnsavedPlaylist(bool),
}

/// Settings for `config.yaml`
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Config {
    pub release: Release,
    pub view: View,
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct View {
    pub language: Language,
    pub theme: Theme,
    pub confirm_discard_playlist: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            language: Default::default(),
            theme: Default::default(),
            confirm_discard_playlist: true,
        }
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
        match self {
            Self::Light => lang::state::light(),
            Self::Dark => lang::state::dark(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Playback {
    #[serde(skip)]
    pub paused: bool,
    /// Whether all players are muted.
    pub muted: bool,
    /// Volume level when not muted. 1.0 is 100%, 0.01 is 1%.
    pub volume: f32,
    /// How long to show images, in seconds.
    pub image_duration: NonZeroUsize,
    /// Whether to pause when window loses focus.
    pub pause_on_unfocus: bool,
}

impl Playback {
    pub fn with_paused(&self, paused: bool) -> Self {
        Self { paused, ..self.clone() }
    }

    pub fn with_paused_maybe(&self, paused: Option<bool>) -> Self {
        Self {
            paused: paused.unwrap_or(self.paused),
            ..self.clone()
        }
    }

    pub fn with_muted(&self, muted: bool) -> Self {
        Self { muted, ..self.clone() }
    }

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
            volume: 1.0,
            image_duration: NonZeroUsize::new(10).unwrap(),
            pause_on_unfocus: false,
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
                view:
                  theme: Light
                  confirm_discard_playlist: false
                playback:
                  muted: true
                  volume: 0.5
                  image_duration: 2
                  pause_on_unfocus: true
            "#,
        )
        .unwrap();

        assert_eq!(
            Config {
                release: Release { check: false },
                view: View {
                    language: Language::English,
                    theme: Theme::Light,
                    confirm_discard_playlist: false
                },
                playback: Playback {
                    paused: false,
                    muted: true,
                    volume: 0.5,
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
view:
  language: en-US
  theme: Dark
  confirm_discard_playlist: true
playback:
  muted: false
  volume: 1.0
  image_duration: 10
  pause_on_unfocus: false
"#
            .trim(),
            serde_yaml::to_string(&Config::default()).unwrap().trim(),
        );
    }
}
