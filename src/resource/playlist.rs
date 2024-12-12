use std::num::NonZeroUsize;

use itertools::Itertools;

use crate::{
    lang, media,
    prelude::{Error, StrictPath},
    resource::ResourceFile,
};

const HINT: &str = "# madamiru-playlist";

/// Settings for a playlist
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Playlist {
    pub layout: Layout,
}

impl ResourceFile for Playlist {
    const FILE_NAME: &'static str = "playlist.madamiru";
}

impl Playlist {
    pub const EXTENSION: &'static str = "madamiru";

    pub fn new(layout: Layout) -> Self {
        Self { layout }
    }

    pub fn load_from(path: &StrictPath) -> Result<Self, Error> {
        let content = Self::load_raw(path).map_err(|e| Error::PlaylistInvalid { why: e.to_string() })?;
        let parsed = Self::load_from_string(&content).map_err(|e| Error::PlaylistInvalid { why: e.to_string() })?;
        Ok(parsed)
    }

    pub fn save_to(&self, path: &StrictPath) -> Result<(), Error> {
        let new_content = self.serialize();

        if let Ok(old_content) = Self::load_raw(path) {
            if old_content == new_content {
                return Ok(());
            }
        }

        path.create_parent_dir()
            .map_err(|e| Error::UnableToSavePlaylist { why: e.to_string() })?;
        path.write_with_content(&self.serialize())
            .map_err(|e| Error::UnableToSavePlaylist { why: e.to_string() })?;

        Ok(())
    }

    pub fn serialize(&self) -> String {
        serde_yaml::to_string(&self)
            .unwrap()
            .replacen("---", &format!("---\n{HINT}"), 1)
    }

    pub fn sources(&self) -> Vec<media::Source> {
        self.layout.sources()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Split(Split),
    Group(Group),
}

impl Layout {
    pub fn sources(&self) -> Vec<media::Source> {
        match self {
            Layout::Split(split) => split
                .first
                .sources()
                .into_iter()
                .chain(split.second.sources())
                .unique()
                .collect(),
            Layout::Group(group) => group.sources.iter().unique().cloned().collect(),
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self::Group(Group::default())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Split {
    pub axis: SplitAxis,
    pub ratio: f32,
    pub first: Box<Layout>,
    pub second: Box<Layout>,
}

impl Default for Split {
    fn default() -> Self {
        Self {
            axis: Default::default(),
            ratio: 0.5,
            first: Default::default(),
            second: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Group {
    pub sources: Vec<media::Source>,
    pub max_media: usize,
    pub content_fit: ContentFit,
    pub orientation: Orientation,
    pub orientation_limit: OrientationLimit,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            sources: Default::default(),
            max_media: 1,
            content_fit: Default::default(),
            orientation: Default::default(),
            orientation_limit: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub const ALL: &'static [Self] = &[Self::Horizontal, Self::Vertical];
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
#[serde(rename_all = "snake_case")]
pub enum OrientationLimit {
    #[default]
    Automatic,
    Fixed(NonZeroUsize),
}

impl OrientationLimit {
    pub const DEFAULT_FIXED: usize = 4;

    pub fn default_fixed() -> NonZeroUsize {
        NonZeroUsize::new(Self::DEFAULT_FIXED).unwrap()
    }

    pub fn is_fixed(&self) -> bool {
        match self {
            Self::Automatic => false,
            Self::Fixed(_) => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentFit {
    /// Scale the media up or down to fill as much of the available space as possible
    /// while maintaining the media's aspect ratio.
    #[default]
    Scale,

    /// Scale the media down to fill as much of the available space as possible
    /// while maintaining the media's aspect ratio.
    /// Don't scale up if it's smaller than the available space.
    ScaleDown,

    /// Crop the media to fill all of the available space.
    /// Maintain the aspect ratio, cutting off parts of the media as needed to fit.
    Crop,

    /// Stretch the media to fill all of the available space.
    /// Preserve the whole media, disregarding the aspect ratio.
    Stretch,
}

impl ContentFit {
    pub const ALL: &'static [Self] = &[Self::Scale, Self::ScaleDown, Self::Crop, Self::Stretch];
}

impl ToString for ContentFit {
    fn to_string(&self) -> String {
        match self {
            ContentFit::Scale => lang::action::scale(),
            ContentFit::ScaleDown => lang::action::scale_down(),
            ContentFit::Crop => lang::action::crop(),
            ContentFit::Stretch => lang::action::stretch(),
        }
    }
}

impl From<ContentFit> for iced::ContentFit {
    fn from(value: ContentFit) -> Self {
        match value {
            ContentFit::Scale => iced::ContentFit::Contain,
            ContentFit::ScaleDown => iced::ContentFit::ScaleDown,
            ContentFit::Crop => iced::ContentFit::Cover,
            ContentFit::Stretch => iced::ContentFit::Fill,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn can_parse_minimal_config() {
        let playlist = Playlist::load_from_string("{}").unwrap();

        assert_eq!(Playlist::default(), playlist);
    }

    #[test]
    fn can_parse_optional_fields_when_present_in_config() {
        let playlist = Playlist::load_from_string(
            r#"
                layout:
                  group:
                    sources:
                      - path:
                          path: tmp
                    max_media: 4
                    content_fit: crop
                    orientation: vertical
                    orientation_limit:
                      fixed: 2
            "#,
        )
        .unwrap();

        assert_eq!(
            Playlist {
                layout: Layout::Group(Group {
                    sources: vec![media::Source::new_path(StrictPath::new("tmp"))],
                    max_media: 4,
                    content_fit: ContentFit::Crop,
                    orientation: Orientation::Vertical,
                    orientation_limit: OrientationLimit::Fixed(NonZeroUsize::new(2).unwrap())
                })
            },
            playlist,
        );
    }

    #[test]
    fn can_be_serialized() {
        assert_eq!(
            r#"
---
# madamiru-playlist
layout:
  group:
    sources: []
    max_media: 1
    content_fit: scale
    orientation: horizontal
    orientation_limit: automatic
"#
            .trim(),
            Playlist::default().serialize().trim(),
        );
    }
}
