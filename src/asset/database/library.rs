use crate::asset::AssetId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct AssetLibrary {
    ids: HashMap<PathBuf, AssetId>,
    paths: HashMap<AssetId, PathBuf>,
}

impl AssetLibrary {
    pub fn new() -> Self {
        AssetLibrary {
            ids: HashMap::new(),
            paths: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, path: PathBuf) -> (Option<AssetId>, Option<PathBuf>) {
        let ret_id = self.ids.insert(path.clone(), id);
        let ret_path = self.paths.insert(id, path);

        (ret_id, ret_path)
    }

    pub fn id_path(&self, id: &AssetId) -> Option<&PathBuf> {
        self.paths.get(id)
    }

    pub fn path_id(&self, path: &Path) -> Option<&AssetId> {
        self.ids.get(path)
    }
}
