use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::ser::SerializeStruct;

use crate::blob::BlobCell;

pub trait Asset: Send + Sync + 'static {}

pub trait Settings:
    Default + Send + Sync + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static
{
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AssetId(u64);

impl AssetId {
    pub fn gen() -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let id = ulid::Ulid::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssetType(u64);

impl AssetType {
    pub fn from<A: Asset>() -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<A>().hash(&mut hasher);
        AssetType(hasher.finish())
    }

    pub fn dynamic(ty: u64) -> Self {
        AssetType(ty)
    }
}

pub struct AssetMetadata<S: Settings> {
    id: AssetId,
    settings: S,
}

impl<S: Settings> AssetMetadata<S> {
    pub fn new(id: AssetId, settings: S) -> Self {
        AssetMetadata { id, settings }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn settings(&self) -> &S {
        &self.settings
    }

    pub fn take(self) -> (AssetId, S) {
        (self.id, self.settings)
    }
}

impl<S: Settings> Default for AssetMetadata<S> {
    fn default() -> Self {
        AssetMetadata {
            id: AssetId::gen(),
            settings: S::default(),
        }
    }
}

impl<S: Settings> serde::Serialize for AssetMetadata<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("AssetMetadata", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("settings", &self.settings)?;
        state.end()
    }
}

impl<'de, S: Settings> serde::Deserialize<'de> for AssetMetadata<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AssetMetadataVisitor<S: Settings>(std::marker::PhantomData<S>);

        impl<'de, S: Settings> serde::de::Visitor<'de> for AssetMetadataVisitor<S> {
            type Value = AssetMetadata<S>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct AssetMetadata")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut id = None;
                let mut settings = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "id" => {
                            if id.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        "settings" => {
                            if settings.is_some() {
                                return Err(serde::de::Error::duplicate_field("settings"));
                            }
                            settings = Some(map.next_value()?);
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                let id = id.ok_or_else(|| serde::de::Error::missing_field("id"))?;
                let settings =
                    settings.ok_or_else(|| serde::de::Error::missing_field("settings"))?;
                Ok(AssetMetadata { id, settings })
            }
        }

        deserializer.deserialize_struct(
            "AssetMetadata",
            &["id", "settings"],
            AssetMetadataVisitor(Default::default()),
        )
    }
}

pub struct SourceInfo {
    id: AssetId,
    checksum: u32,
    modified: u64,
}

impl SourceInfo {
    pub fn new(id: AssetId, checksum: u32, modified: u64) -> Self {
        SourceInfo {
            id,
            checksum,
            modified,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
    }
}

pub struct ArtifactInfo {
    id: AssetId,
    ty: AssetType,
    filepath: PathBuf,
    dependencies: Vec<AssetId>,
}

impl ArtifactInfo {
    pub fn new(id: AssetId, ty: AssetType, filepath: PathBuf, dependencies: Vec<AssetId>) -> Self {
        ArtifactInfo {
            id,
            ty,
            filepath,
            dependencies,
        }
    }

    pub fn from<A: Asset>(id: AssetId, filepath: PathBuf, dependencies: Vec<AssetId>) -> Self {
        ArtifactInfo {
            id,
            ty: AssetType::from::<A>(),
            filepath,
            dependencies,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn filepath(&self) -> &Path {
        &self.filepath
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn set_dependencies(&mut self, dependencies: Vec<AssetId>) {
        self.dependencies = dependencies;
    }
}

pub struct ArtifactHeader {
    info_size: u64,
    data_size: u64,
}

impl ArtifactHeader {
    pub fn new(info_size: u64, data_size: u64) -> Self {
        ArtifactHeader {
            info_size,
            data_size,
        }
    }

    pub fn info_size(&self) -> u64 {
        self.info_size
    }

    pub fn data_size(&self) -> u64 {
        self.data_size
    }
}

pub struct Artifact {
    header: ArtifactHeader,
    info: ArtifactInfo,
    data: Vec<u8>,
}

pub struct AssetLibrary {
    sources: HashMap<PathBuf, SourceInfo>,
    artifacts: HashMap<AssetId, ArtifactInfo>,
}

pub enum AssetMode {
    Unprocessed,
    Processed,
}

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: &'a AssetMetadata<S>,
    dependencies: Vec<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, bytes: &'a [u8], metadata: &'a AssetMetadata<S>) -> Self {
        LoadContext {
            path,
            bytes,
            metadata,
            dependencies: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        &self.metadata
    }

    pub fn dependencies(&self) -> &[AssetId] {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.push(id);
    }

    pub fn finish(self) -> Vec<AssetId> {
        self.dependencies
    }
}

pub trait AssetImporter: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings;
    type Saver: AssetSaver<Asset = Self::Asset, Settings = Self::Settings>;

    fn import(ctx: &mut LoadContext<Self::Settings>) -> Self::Asset;
    fn extensions() -> &'static [&'static str] {
        &[]
    }
}

pub trait AssetProcessor: Send + Sync + 'static {
    type Importer: AssetImporter;

    fn process(
        asset: &mut <Self::Importer as AssetImporter>::Asset,
        metadata: &AssetMetadata<<Self::Importer as AssetImporter>::Settings>,
    );
}

pub trait AssetSaver: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn save(asset: &Self::Asset, metadata: &AssetMetadata<Self::Settings>) -> Vec<u8>;
    fn load(bytes: &[u8]) -> Self::Asset;
}

pub struct ErasedAsset {
    asset: BlobCell,
    ty: AssetType,
}

impl ErasedAsset {
    pub fn new<A: Asset>(asset: A) -> Self {
        ErasedAsset {
            asset: BlobCell::new(asset),
            ty: AssetType::from::<A>(),
        }
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn cast<A: Asset>(&self) -> &A {
        self.asset.value()
    }

    pub fn cast_mut<A: Asset>(&mut self) -> &mut A {
        self.asset.value_mut()
    }

    pub fn into<A: Asset>(self) -> A {
        self.asset.take()
    }
}

pub struct AssetStore {
    assets: HashMap<AssetId, ErasedAsset>,
}

impl AssetStore {
    pub fn new() -> Self {
        AssetStore {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, asset: ErasedAsset) {
        self.assets.insert(id, asset);
    }

    pub fn get<A: Asset>(&self, id: AssetId) -> Option<&A> {
        self.assets.get(&id).map(|asset| asset.cast())
    }

    pub fn get_mut<A: Asset>(&mut self, id: AssetId) -> Option<&mut A> {
        self.assets.get_mut(&id).map(|asset| asset.cast_mut())
    }

    pub fn remove(&mut self, id: AssetId) -> Option<ErasedAsset> {
        self.assets.remove(&id)
    }
}

pub trait PathExt {
    fn append_extension(&self, ext: &str) -> PathBuf;
    fn ext(&self) -> Option<&str>;
}

impl<T: AsRef<Path>> PathExt for T {
    fn append_extension(&self, ext: &str) -> PathBuf {
        PathBuf::from(format!("{}.{}", self.as_ref().display(), ext))
    }

    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }
}

pub struct ErasedAssetLoader {
    import: fn(&PathBuf) -> ErasedAsset,
    process: Option<fn(&AssetId)>,
}

impl ErasedAssetLoader {
    pub fn new<I: AssetImporter>() -> Self {
        Self {
            import: |path| {
                let metapath = path.append_extension("meta");
                let metadata = match std::fs::read_to_string(&metapath) {
                    Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
                    Err(_) => AssetMetadata::<I::Settings>::default(),
                };

                let metabytes = match toml::to_string(&metadata) {
                    Ok(contents) => contents,
                    Err(_) => todo!("Error handling"),
                };

                match std::fs::write(metapath, &metabytes) {
                    Ok(_) => {}
                    Err(_) => todo!("Error handling"),
                }

                let bytes = match std::fs::read(path) {
                    Ok(bytes) => bytes,
                    Err(_) => todo!("Error handling"),
                };

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&path, &bytes, &metadata);
                    let asset = I::import(&mut ctx);
                    (asset, ctx.finish())
                };

                let modified = match std::fs::metadata(path) {
                    Ok(metadata) => metadata
                        .modified()
                        .unwrap_or(SystemTime::now())
                        .elapsed()
                        .unwrap_or_default()
                        .as_secs(),
                    Err(_) => 0,
                };

                let mut hasher = crc32fast::Hasher::new();
                bytes.hash(&mut hasher);
                metabytes.hash(&mut hasher);
                let checksum = hasher.finalize() as u32;

                let source = SourceInfo::new(metadata.id(), checksum, modified);

                let artifact =
                    ArtifactInfo::from::<I::Asset>(metadata.id(), path.clone(), dependencies);

                todo!()
            },
            process: None,
        }
    }
}

pub struct AssetLoaders {}
