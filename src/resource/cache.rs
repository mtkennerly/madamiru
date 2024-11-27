use crate::{
    prelude::CANONICAL_VERSION,
    resource::{config::Config, ResourceFile, SaveableResourceFile},
};

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Cache {
    pub version: Option<(u32, u32, u32)>,
    pub release: Release,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Release {
    pub checked: chrono::DateTime<chrono::Utc>,
    pub latest: Option<semver::Version>,
}

impl ResourceFile for Cache {
    const FILE_NAME: &'static str = "cache.yaml";
}

impl SaveableResourceFile for Cache {}

impl Cache {
    pub fn migrate_config(mut self, config: &mut Config) -> Self {
        let mut updated = false;

        if self.version != Some(*CANONICAL_VERSION) {
            self.version = Some(*CANONICAL_VERSION);
            updated = true;
        }

        if updated {
            self.save();
            config.save();
        }

        self
    }

    pub fn should_check_app_update(&self) -> bool {
        let now = chrono::offset::Utc::now();
        now.signed_duration_since(self.release.checked).num_hours() >= 24
    }
}
