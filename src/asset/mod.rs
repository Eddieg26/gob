use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
};

use bytes::IntoBytes;
use serde::ser::SerializeStruct;

use crate::{blob::BlobCell, dense::DenseMap};

pub mod bytes;

pub trait Asset: Send + Sync + 'static {}

pub trait Settings:
    Default + Send + Sync + serde::Serialize + for<'a> serde::Deserialize<'a> + 'static
{
}

#[derive(
    Default, Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize,
)]
pub struct AssetId(u64);

impl AssetId {
    pub fn gen() -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let id = ulid::Ulid::new();
        id.hash(&mut hasher);
        AssetId(hasher.finish())
    }
}

impl IntoBytes for AssetId {
    fn into_bytes(&self) -> Vec<u8> {
        self.0.into_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        u64::from_bytes(bytes).map(AssetId)
    }
}

impl ToString for AssetId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Default, Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SettingsType(u64);

impl SettingsType {
    pub fn from<S: Settings>() -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::any::TypeId::of::<S>().hash(&mut hasher);
        SettingsType(hasher.finish())
    }

    pub fn dynamic(ty: u64) -> Self {
        SettingsType(ty)
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

#[derive(Clone, Debug, Default)]
pub struct ArtifactMeta {
    id: AssetId,
    ty: AssetType,
    checksum: u32,
    modified: u64,
    dependencies: HashSet<AssetId>,
}

impl ArtifactMeta {
    pub fn new(
        id: AssetId,
        ty: AssetType,
        checksum: u32,
        modified: u64,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        ArtifactMeta {
            id,
            ty,
            checksum,
            modified,
            dependencies,
        }
    }

    pub fn from<A: Asset>(
        id: AssetId,
        checksum: u32,
        modified: u64,
        dependencies: HashSet<AssetId>,
    ) -> Self {
        ArtifactMeta {
            id,
            ty: AssetType::from::<A>(),
            checksum,
            modified,
            dependencies,
        }
    }

    pub fn id(&self) -> AssetId {
        self.id
    }

    pub fn ty(&self) -> AssetType {
        self.ty
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    pub fn modified(&self) -> u64 {
        self.modified
    }

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn set_dependencies(&mut self, dependencies: HashSet<AssetId>) {
        self.dependencies = dependencies;
    }
}

impl IntoBytes for ArtifactMeta {
    fn into_bytes(&self) -> Vec<u8> {
        todo!()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        todo!()
    }
}

pub struct Artifact {
    meta: ArtifactMeta,
    asset: Vec<u8>,
}

impl Artifact {
    pub fn new(meta: ArtifactMeta, asset: Vec<u8>) -> Self {
        Artifact { meta, asset }
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn asset(&self) -> &[u8] {
        &self.asset
    }

    pub fn meta_mut(&mut self) -> &mut ArtifactMeta {
        &mut self.meta
    }

    pub fn read_meta(path: &Path) -> std::io::Result<ArtifactMeta> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = [0u8; 8];
        file.read(&mut buffer)?;
        let len = usize::from_bytes(&mut buffer)
            .ok_or::<std::io::Error>(std::io::ErrorKind::InvalidData.into())?;
        let mut bytes = vec![0u8; len];
        file.read_exact(&mut bytes)?;
        ArtifactMeta::from_bytes(&bytes).ok_or(std::io::ErrorKind::InvalidData.into())
    }
}

impl IntoBytes for Artifact {
    fn into_bytes(&self) -> Vec<u8> {
        todo!()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        todo!()
    }
}

pub enum AssetMode {
    Unprocessed,
    Processed,
}

pub struct LoadContext<'a, S: Settings> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: &'a AssetMetadata<S>,
    dependencies: HashSet<AssetId>,
}

impl<'a, S: Settings> LoadContext<'a, S> {
    pub fn new(path: &'a Path, bytes: &'a [u8], metadata: &'a AssetMetadata<S>) -> Self {
        LoadContext {
            path,
            bytes,
            metadata,
            dependencies: HashSet::new(),
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

    pub fn dependencies(&self) -> &HashSet<AssetId> {
        &self.dependencies
    }

    pub fn add_dependency(&mut self, id: AssetId) {
        self.dependencies.insert(id);
    }

    pub fn finish(self) -> HashSet<AssetId> {
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

pub struct ProcessContext<'a, S: Settings> {
    assets: &'a mut AssetStore,
    metadata: &'a AssetMetadata<S>,
    dependencies: &'a HashSet<AssetId>,
}

impl<'a, S: Settings> ProcessContext<'a, S> {
    pub fn new(
        assets: &'a mut AssetStore,
        metadata: &'a AssetMetadata<S>,
        dependencies: &'a HashSet<AssetId>,
    ) -> Self {
        ProcessContext {
            assets,
            metadata,
            dependencies,
        }
    }

    pub fn asset<A: Asset>(&self, id: AssetId) -> Option<&A> {
        self.dependencies
            .contains(&id)
            .then(|| self.assets.get(id))
            .flatten()
    }

    pub fn metadata(&self) -> &AssetMetadata<S> {
        self.metadata
    }
}

pub trait AssetProcessor: Send + Sync + 'static {
    type Importer: AssetImporter;

    fn process(
        asset: &mut <Self::Importer as AssetImporter>::Asset,
        ctx: &mut ProcessContext<<Self::Importer as AssetImporter>::Settings>,
    );
}

pub trait AssetSaver: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn save(asset: &Self::Asset, metadata: &AssetMetadata<Self::Settings>) -> Vec<u8>;
    fn load(bytes: &[u8]) -> Self::Asset;
}

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

    pub fn id_path(&self, id: AssetId) -> Option<&PathBuf> {
        self.paths.get(&id)
    }

    pub fn path_id(&self, path: &Path) -> Option<&AssetId> {
        self.ids.get(path)
    }
}

pub struct AssetStore {
    assets: HashMap<AssetId, LoadedAsset>,
}

impl AssetStore {
    pub fn new() -> Self {
        AssetStore {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: AssetId, asset: LoadedAsset) {
        self.assets.insert(id, asset);
    }

    pub fn get<A: Asset>(&self, id: AssetId) -> Option<&A> {
        self.assets.get(&id).map(|cell| cell.asset())
    }

    pub fn remove(&mut self, id: AssetId) -> Option<LoadedAsset> {
        self.assets.remove(&id)
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn clear(&mut self) {
        self.assets.clear();
    }
}

pub trait PathExt {
    fn append_extension(&self, ext: &str) -> PathBuf;
    fn ext(&self) -> Option<&str>;
    fn modified_secs(&self) -> std::io::Result<u64>;
}

impl<T: AsRef<Path>> PathExt for T {
    fn append_extension(&self, ext: &str) -> PathBuf {
        PathBuf::from(format!("{}.{}", self.as_ref().display(), ext))
    }

    fn ext(&self) -> Option<&str> {
        self.as_ref().extension().and_then(|ext| ext.to_str())
    }

    fn modified_secs(&self) -> std::io::Result<u64> {
        self.as_ref()
            .metadata()?
            .modified()
            .unwrap_or(std::time::SystemTime::now())
            .elapsed()
            .map_err(|_| std::io::ErrorKind::InvalidData)?
            .as_secs()
            .try_into()
            .map_err(|_| std::io::ErrorKind::InvalidData.into())
    }
}

pub struct ImportedAsset {
    asset: BlobCell,
    metadata: BlobCell,
    artifact: ArtifactMeta,
}

impl ImportedAsset {
    pub fn new<A: Asset, S: Settings>(
        asset: A,
        metadata: AssetMetadata<S>,
        artifact: ArtifactMeta,
    ) -> Self {
        ImportedAsset {
            asset: BlobCell::new(asset),
            metadata: BlobCell::new(metadata),
            artifact,
        }
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }

    pub fn metadata<S: Settings>(&self) -> &AssetMetadata<S> {
        self.metadata.value()
    }

    pub fn artifact(&self) -> &ArtifactMeta {
        &self.artifact
    }

    pub fn mutate<A: Asset, S: Settings>(
        &mut self,
    ) -> (&mut A, &mut AssetMetadata<S>, &ArtifactMeta) {
        (
            self.asset.value_mut(),
            self.metadata.value_mut(),
            &self.artifact,
        )
    }
}

impl Into<LoadedAsset> for ImportedAsset {
    fn into(self) -> LoadedAsset {
        LoadedAsset { asset: self.asset }
    }
}

pub struct LoadedAsset {
    asset: BlobCell,
}

impl LoadedAsset {
    pub fn new<A: Asset>(asset: A) -> Self {
        LoadedAsset {
            asset: BlobCell::new(asset),
        }
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }
}

pub struct SaveInfo {
    pub id: AssetId,
    pub old_artifact: Option<ArtifactMeta>,
    pub removed_dependencies: HashSet<AssetId>,
}

impl SaveInfo {
    pub fn new(
        id: AssetId,
        old_artifact: Option<ArtifactMeta>,
        removed_dependencies: HashSet<AssetId>,
    ) -> Self {
        SaveInfo {
            id,
            old_artifact,
            removed_dependencies,
        }
    }
}

pub struct ErasedAssetImporter {
    import: fn(&Path) -> std::io::Result<ImportedAsset>,
    pub process: Option<fn(&mut ImportedAsset, &mut AssetStore)>,
    save: fn(&Path, &ImportedAsset) -> std::io::Result<SaveInfo>,
    load: fn(&Artifact) -> std::io::Result<LoadedAsset>,
}

impl ErasedAssetImporter {
    pub fn new<I: AssetImporter>() -> Self {
        Self {
            import: |path| {
                let metapath = path.append_extension("meta");
                let metadata = match std::fs::read_to_string(&metapath) {
                    Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
                    Err(_) => AssetMetadata::<I::Settings>::default(),
                };

                let metabytes =
                    toml::to_string(&metadata).map_err(|_| std::io::ErrorKind::InvalidData)?;
                std::fs::write(metapath, &metabytes)?;

                let bytes = std::fs::read(path)?;

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&path, &bytes, &metadata);
                    let asset = I::import(&mut ctx);
                    (asset, ctx.finish())
                };

                let modified = path.modified_secs().unwrap_or_default();

                let mut hasher = crc32fast::Hasher::new();
                bytes.hash(&mut hasher);
                metabytes.hash(&mut hasher);
                let checksum = hasher.finalize() as u32;

                let artifact =
                    ArtifactMeta::from::<I::Asset>(metadata.id(), checksum, modified, dependencies);

                Ok(ImportedAsset::new(asset, metadata, artifact))
            },
            process: None,
            save: |path, imported| {
                let old_artifact = Artifact::read_meta(path).ok();

                let asset = imported.asset::<I::Asset>();
                let metadata = imported.metadata::<I::Settings>();

                let asset = I::Saver::save(asset, metadata);
                let artifact = Artifact::new(imported.artifact().clone(), asset).into_bytes();

                std::fs::write(path, artifact)?;

                let removed = match &old_artifact {
                    Some(old) => old
                        .dependencies()
                        .difference(imported.artifact().dependencies()),
                    None => todo!(),
                }
                .copied()
                .collect::<HashSet<_>>();

                let info = SaveInfo::new(imported.artifact().id(), old_artifact, removed);
                Ok(info)
            },
            load: |artifact| {
                let asset = I::Saver::load(artifact.asset());

                Ok(LoadedAsset::new(asset))
            },
        }
    }

    pub fn set_processer<P: AssetProcessor>(&mut self) {
        self.process = Some(|imported, assets| {
            let (asset, metadata, artifact) = imported.mutate();

            let mut ctx = ProcessContext::new(assets, &metadata, artifact.dependencies());

            P::process(asset, &mut ctx);
        });
    }

    pub fn import(&self, path: &Path) -> std::io::Result<ImportedAsset> {
        (self.import)(path)
    }

    pub fn save(&self, path: &Path, imported: &ImportedAsset) -> std::io::Result<SaveInfo> {
        (self.save)(path, imported)
    }

    pub fn load(&self, artifact: &Artifact) -> std::io::Result<LoadedAsset> {
        (self.load)(artifact)
    }
}

pub struct AssetImporters {
    importers: DenseMap<AssetType, ErasedAssetImporter>,
    types: HashMap<&'static str, AssetType>,
}

impl AssetImporters {
    pub fn new() -> Self {
        AssetImporters {
            importers: DenseMap::new(),
            types: HashMap::new(),
        }
    }

    pub fn register<I: AssetImporter>(&mut self) {
        let ty = AssetType::from::<I::Asset>();
        let importer = ErasedAssetImporter::new::<I>();

        self.importers.insert(ty, importer);
        for ext in I::extensions() {
            self.types.insert(ext, ty);
        }
    }

    pub fn importer(&self, ty: AssetType) -> Option<&ErasedAssetImporter> {
        self.importers.get(&ty)
    }

    pub fn importer_by_ext(&self, ext: &str) -> Option<&ErasedAssetImporter> {
        self.types.get(ext).and_then(|ty| self.importer(*ty))
    }
}

fn import_assets(paths: &[&Path], importers: &AssetImporters, library: &mut AssetLibrary) {
    let mut assets = AssetStore::new();

    for path in paths {
        let ext = match path.ext() {
            Some(ext) => ext,
            None => continue,
        };

        let importer = match importers.importer_by_ext(ext) {
            Some(importer) => importer,
            None => continue,
        };

        let mut imported = match (importer.import)(path) {
            Ok(imported) => imported,
            Err(_) => continue,
        };

        if let Some(process) = importer.process {
            for dependency in imported.artifact().dependencies() {
                if assets.contains(dependency) {
                    continue;
                }

                let path = dependency.to_string();
                let artifact = match std::fs::read(path)
                    .ok()
                    .and_then(|bytes| Artifact::from_bytes(&bytes))
                {
                    Some(artifact) => artifact,
                    None => continue,
                };

                let importer = match importers.importer(artifact.meta().ty()) {
                    Some(importer) => importer,
                    None => continue,
                };

                let asset = match importer.load(&artifact) {
                    Ok(asset) => asset,
                    Err(_) => continue,
                };

                assets.insert(artifact.meta().id, asset);
            }

            (process)(&mut imported, &mut assets);
        }

        let saved = match importer.save(&path, &imported) {
            Ok(saved) => {
                library.insert(imported.artifact().id(), path.to_path_buf());
                assets.insert(imported.artifact().id(), imported.into());
                saved
            }
            Err(_) => continue,
        };
    }
}
