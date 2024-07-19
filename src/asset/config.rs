use super::{AssetId, PathExt};
use std::path::{Path, PathBuf};

pub struct AssetConfig {
    root: PathBuf,
    assets: PathBuf,
    preferences: PathBuf,
    cache: PathBuf,
    temp: PathBuf,
}

impl AssetConfig {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            assets: root.join("Assets"),
            preferences: root.join("Preferences"),
            cache: root.join(".cache"),
            temp: root.join(".temp"),
            root,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn preferences(&self) -> &Path {
        &self.preferences
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn temp(&self) -> &Path {
        &self.temp
    }

    pub fn library(&self) -> PathBuf {
        self.cache.join("assets.lib")
    }

    pub fn artifacts(&self) -> PathBuf {
        self.cache.join("artifacts")
    }

    pub fn artifact(&self, id: &AssetId) -> PathBuf {
        self.artifacts().join(id.to_string())
    }

    pub fn metadata(path: impl AsRef<Path>) -> PathBuf {
        path.as_ref().append_extension("meta")
    }
}
