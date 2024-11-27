use crate::{
    lang::{self, Language},
    prelude::{app_dir, Error, StrictPath},
    resource::{ResourceFile, SaveableResourceFile},
};

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
    pub muted: bool,
    pub max: usize,
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
            max: 4,
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
                  max: 1
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
                    max: 1
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
  max: 4
"#
            .trim(),
            serde_yaml::to_string(&Config::default()).unwrap().trim(),
        );
    }
}
