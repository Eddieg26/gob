use super::{
    artifact::{Artifact, ArtifactMeta},
    config::AssetConfig,
    AssetId, AssetMetadata, PathExt, Settings,
};
use crate::asset::bytes::IntoBytes;
use std::{
    error::Error,
    hash::Hash,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

#[derive(Debug, Clone)]
pub enum AssetIoError {
    NotFound(PathBuf),
    Io(Arc<std::io::Error>),
    Http(u16),
}

impl PartialEq for AssetIoError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AssetIoError::NotFound(a), AssetIoError::NotFound(b)) => a == b,
            (AssetIoError::Io(a), AssetIoError::Io(b)) => a.kind() == b.kind(),
            (AssetIoError::Http(a), AssetIoError::Http(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for AssetIoError {}

impl From<std::io::Error> for AssetIoError {
    fn from(error: std::io::Error) -> Self {
        AssetIoError::Io(Arc::new(error))
    }
}

impl From<std::io::ErrorKind> for AssetIoError {
    fn from(kind: std::io::ErrorKind) -> Self {
        AssetIoError::Io(Arc::new(std::io::Error::from(kind)))
    }
}

impl std::fmt::Display for AssetIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AssetIoError::NotFound(path) => write!(f, "Asset not found: {:?}", path),
            AssetIoError::Io(error) => write!(f, "IO error: {}", error),
            AssetIoError::Http(status) => write!(f, "HTTP error: {}", status),
        }
    }
}

impl Error for AssetIoError {}

pub struct FileReader {
    reader: Box<dyn Read>,
}

impl FileReader {
    pub fn new<R: Read + 'static>(reader: R) -> Self {
        Self {
            reader: Box::new(reader),
        }
    }

    pub fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), AssetIoError> {
        self.reader
            .read_exact(buffer)
            .map_err(|e| AssetIoError::Io(Arc::new(e)))
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, AssetIoError> {
        self.reader
            .read(buffer)
            .map_err(|e| AssetIoError::Io(Arc::new(e)))
    }

    pub fn read_to_end(&mut self) -> Result<Vec<u8>, AssetIoError> {
        let mut buffer = Vec::new();
        self.reader
            .read_to_end(&mut buffer)
            .map_err(|e| AssetIoError::Io(Arc::new(e)))?;
        Ok(buffer)
    }
}

pub trait FileSystem: Send + Sync + 'static {
    fn read(&self, path: &Path) -> Result<Vec<u8>, AssetIoError>;
    fn read_to_string(&self, path: &Path) -> Result<String, AssetIoError>;
    fn read_exact(&self, path: &Path, buffer: &mut [u8]) -> Result<(), AssetIoError>;
    fn reader(&self, path: &Path) -> Result<FileReader, AssetIoError>;
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), AssetIoError>;
    fn remove(&self, path: &Path) -> Result<Vec<PathBuf>, AssetIoError>;
    fn rename(&self, old: &Path, new: &Path) -> Result<(), AssetIoError>;
    fn read_directory(&self, path: &Path, recursive: bool) -> Result<Vec<PathBuf>, AssetIoError>;
    fn create_dir(&self, path: &Path) -> Result<(), AssetIoError>;
}

pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, AssetIoError> {
        std::fs::read(path).map_err(|e| e.into())
    }

    fn read_exact(&self, path: &Path, buffer: &mut [u8]) -> Result<(), AssetIoError> {
        let mut file = std::fs::File::open(path)?;
        file.read_exact(buffer).map_err(|e| e.into())
    }

    fn read_to_string(&self, path: &Path) -> Result<String, AssetIoError> {
        std::fs::read_to_string(path).map_err(|e| e.into())
    }

    fn reader(&self, path: &Path) -> Result<FileReader, AssetIoError> {
        let file = std::fs::File::open(path)?;
        Ok(FileReader::new(file))
    }

    fn write(&self, path: &Path, data: &[u8]) -> Result<(), AssetIoError> {
        std::fs::write(path, data).map_err(|e| e.into())
    }

    fn remove(&self, path: &Path) -> Result<Vec<PathBuf>, AssetIoError> {
        if path.is_dir() {
            let entries = self.read_directory(path, true).unwrap_or_default();
            if entries.is_empty() {
                std::fs::remove_dir(path)
                    .map(|_| vec![])
                    .map_err(|e| e.into())
            } else {
                std::fs::remove_dir_all(path)
                    .map(|_| entries)
                    .map_err(|e| e.into())
            }
        } else if path.is_file() {
            std::fs::remove_file(path)
                .map(|_| vec![])
                .map_err(|e| e.into())
        } else {
            Err(std::io::ErrorKind::NotFound.into())
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), AssetIoError> {
        std::fs::rename(from, to).map_err(|e| e.into())
    }

    fn read_directory(&self, path: &Path, recursive: bool) -> Result<Vec<PathBuf>, AssetIoError> {
        let mut paths = vec![];
        let dir = std::fs::read_dir(path)?;

        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                paths.extend(self.read_directory(&path, recursive)?)
            }
        }

        Ok(paths)
    }

    fn create_dir(&self, path: &Path) -> Result<(), AssetIoError> {
        std::fs::create_dir_all(path).map_err(|e| e.into())
    }
}

pub struct AssetFileSystem {
    config: AssetConfig,
    system: Box<dyn FileSystem>,
}

impl AssetFileSystem {
    pub fn new(config: AssetConfig, system: impl FileSystem) -> Self {
        Self {
            config,
            system: Box::new(system),
        }
    }

    pub fn config(&self) -> &AssetConfig {
        &self.config
    }

    pub fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetIoError> {
        self.system.read(path.as_ref())
    }

    pub fn read_exact(
        &self,
        path: impl AsRef<Path>,
        buffer: &mut [u8],
    ) -> Result<(), AssetIoError> {
        self.system.read_exact(path.as_ref(), buffer)
    }

    pub fn read_to_string(&self, path: impl AsRef<Path>) -> Result<String, AssetIoError> {
        self.system.read_to_string(path.as_ref())
    }

    pub fn reader(&self, path: impl AsRef<Path>) -> Result<FileReader, AssetIoError> {
        self.system.reader(path.as_ref())
    }

    pub fn write(
        &self,
        path: impl AsRef<Path>,
        data: impl AsRef<[u8]>,
    ) -> Result<(), AssetIoError> {
        self.system.write(path.as_ref(), data.as_ref())
    }

    pub fn remove(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, AssetIoError> {
        self.system.remove(path.as_ref())
    }

    pub fn rename(&self, old: impl AsRef<Path>, new: impl AsRef<Path>) -> Result<(), AssetIoError> {
        self.system.rename(old.as_ref(), new.as_ref())
    }

    pub fn read_directory(
        &self,
        path: impl AsRef<Path>,
        recursive: bool,
    ) -> Result<Vec<PathBuf>, AssetIoError> {
        self.system.read_directory(path.as_ref(), recursive)
    }

    pub fn load_metadata<S: Settings>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AssetMetadata<S>, AssetIoError> {
        let path = path.as_ref().append_extension("meta");
        let content = self.read_to_string(path)?;
        toml::from_str::<AssetMetadata<S>>(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e).into())
    }

    pub fn save_metadata<S: Settings>(
        &self,
        path: impl AsRef<Path>,
        metadata: &AssetMetadata<S>,
    ) -> Result<Vec<u8>, AssetIoError> {
        let path = path.as_ref().append_extension("meta");
        let content = toml::to_string(metadata)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.write(path, &content)?;
        Ok(content.into())
    }

    pub fn load_artifact_meta(&self, id: &AssetId) -> Result<ArtifactMeta, AssetIoError> {
        let path = self.config.artifact(id);
        let mut reader = self.reader(&path)?;
        let mut len_buffer = [0u8; 8];
        reader.read_exact(&mut len_buffer)?;
        let len = usize::from_bytes(&len_buffer).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Could not read length.",
        ))?;
        let mut buffer = vec![0u8; len];
        reader.read_exact(&mut buffer)?;
        let meta = ArtifactMeta::from_bytes(&buffer).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Could not read artifact meta.",
        ))?;

        Ok(meta)
    }

    pub fn load_artifact(&self, id: &AssetId) -> Result<Artifact, AssetIoError> {
        let path = self.config.artifact(id);
        let bytes = self.read(&path)?;
        Artifact::from_bytes(&bytes).ok_or(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Could not read artifact.").into(),
        )
    }

    pub fn modified_secs(path: impl AsRef<Path>) -> Result<u64, AssetIoError> {
        let metadata = path.as_ref().metadata()?;
        let modified = metadata.modified().unwrap_or(SystemTime::now());
        let elapsed = modified
            .elapsed()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(elapsed.as_secs())
    }

    pub fn calculate_checksum(asset: &[u8], metadata: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        asset.hash(&mut hasher);
        metadata.hash(&mut hasher);
        hasher.finalize()
    }
}
