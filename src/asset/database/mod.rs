use importer::AssetImporters;
use library::AssetLibrary;
use std::sync::{Arc, RwLock, RwLockReadGuard};

pub mod importer;
pub mod library;

pub struct AssetDatabase {
    library: Arc<RwLock<AssetLibrary>>,
    importers: Arc<RwLock<AssetImporters>>,
}

impl AssetDatabase {
    pub fn new() -> Self {
        Self {
            library: Arc::new(RwLock::new(AssetLibrary::new())),
            importers: Arc::new(RwLock::new(AssetImporters::new())),
        }
    }

    pub fn library(&self) -> RwLockReadGuard<AssetLibrary> {
        self.library.read().unwrap()
    }

    pub fn importers(&self) -> RwLockReadGuard<AssetImporters> {
        self.importers.read().unwrap()
    }
}
