use crate::{
    asset::{
        artifact::{Artifact, ArtifactMeta},
        bytes::IntoBytes,
        io::{AssetFileSystem, AssetIoError},
        Asset, AssetId, AssetMetadata, AssetType, PathExt, Settings,
    },
    blob::BlobCell,
    dense::DenseMap,
};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    path::{Path, PathBuf},
};

use super::AssetDatabase;

pub struct ImportError {
    pub path: PathBuf,
    pub id: AssetId,
    pub artifact: Option<ArtifactMeta>,
    pub error: Box<dyn Error + Send + Sync>,
}

impl ImportError {
    pub fn new<E: Error + Send + Sync + 'static>(
        path: impl AsRef<Path>,
        id: AssetId,
        error: E,
    ) -> Self {
        ImportError {
            path: path.as_ref().to_path_buf(),
            id,
            error: Box::new(error),
            artifact: None,
        }
    }

    pub fn with_artifact(mut self, artifact: ArtifactMeta) -> Self {
        self.artifact = Some(artifact);
        self
    }

    pub fn with_option_artifact(mut self, artifact: Option<ArtifactMeta>) -> Self {
        self.artifact = artifact;
        self
    }
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
    type Error: Error + Send + Sync;

    fn import(ctx: &mut LoadContext<Self::Settings>) -> Result<Self::Asset, Self::Error>;
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
    type Error: Error + Send + Sync;

    fn process(
        asset: &mut <Self::Importer as AssetImporter>::Asset,
        ctx: &mut ProcessContext<<Self::Importer as AssetImporter>::Settings>,
    ) -> Result<(), Self::Error>;
}

pub trait AssetSaver: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings;

    fn save(asset: &Self::Asset, metadata: &AssetMetadata<Self::Settings>) -> Vec<u8>;
    fn load(bytes: &[u8]) -> Self::Asset;
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

    pub fn extend(&mut self, store: AssetStore) {
        self.assets.extend(store.assets);
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

pub struct ImportedAsset {
    asset: BlobCell,
    metadata: BlobCell,
    pub artifact: ArtifactMeta,
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

    pub fn asset_mut<A: Asset>(&mut self) -> &A {
        self.asset.value_mut()
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

pub struct LoadedAsset {
    asset: BlobCell,
    meta: ArtifactMeta,
}

impl LoadedAsset {
    pub fn new<A: Asset>(asset: A, meta: ArtifactMeta) -> Self {
        LoadedAsset {
            asset: BlobCell::new(asset),
            meta,
        }
    }

    pub fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }
}

pub struct SavedAsset {
    pub meta: ArtifactMeta,
    pub prev_meta: Option<ArtifactMeta>,
    pub removed_dependencies: HashSet<AssetId>,
    asset: BlobCell,
}

impl SavedAsset {
    pub fn new<A: Asset>(
        asset: A,
        artifact: ArtifactMeta,
        prev_artifact: Option<ArtifactMeta>,
        removed_dependencies: HashSet<AssetId>,
    ) -> Self {
        SavedAsset {
            meta: artifact,
            prev_meta: prev_artifact,
            removed_dependencies,
            asset: BlobCell::new(asset),
        }
    }

    pub fn asset<A: Asset>(&self) -> &A {
        self.asset.value()
    }
}

impl Into<LoadedAsset> for SavedAsset {
    fn into(self) -> LoadedAsset {
        LoadedAsset {
            asset: self.asset,
            meta: self.meta,
        }
    }
}

pub struct ErasedAssetImporter {
    import: fn(&AssetFileSystem, &Path) -> Result<ImportedAsset, ImportError>,
    pub process: Option<fn(&Path, &mut ImportedAsset, &mut AssetStore) -> Result<(), ImportError>>,
    save: fn(&AssetFileSystem, &Path, ImportedAsset) -> Result<SavedAsset, ImportError>,
    load: fn(Artifact) -> std::io::Result<LoadedAsset>,
}

impl ErasedAssetImporter {
    pub fn new<I: AssetImporter>() -> Self {
        Self {
            import: |fs, path| {
                let metadata = fs.load_metadata::<I::Settings>(path).unwrap_or_default();
                let metabytes = fs
                    .save_metadata(path, &metadata)
                    .map_err(|e| ImportError::new(path, metadata.id(), e))?;
                let bytes = fs
                    .read(path)
                    .map_err(|e| ImportError::new(path, metadata.id(), e))?;

                let (asset, dependencies) = {
                    let mut ctx = LoadContext::new(&path, &bytes, &metadata);
                    let asset = I::import(&mut ctx)
                        .map_err(|e| ImportError::new(path, metadata.id(), e))?;
                    (asset, ctx.finish())
                };

                let modified = AssetFileSystem::modified_secs(path).unwrap_or_default();
                let checksum = AssetFileSystem::calculate_checksum(&bytes, &metabytes);

                let artifact =
                    ArtifactMeta::from::<I::Asset>(metadata.id(), checksum, modified, dependencies);

                Ok(ImportedAsset::new(asset, metadata, artifact))
            },
            process: None,
            save: |fs, path, imported| {
                let prev_artifact = fs.load_artifact_meta(&imported.artifact().id()).ok();

                let asset = imported.asset::<I::Asset>();
                let metadata = imported.metadata::<I::Settings>();
                let id = metadata.id();

                let bytes = I::Saver::save(asset, metadata);
                let artifact = Artifact::new(imported.artifact, bytes);
                let artifact_bytes = artifact.into_bytes();

                if let Err(e) = fs.write(path, artifact_bytes) {
                    let error = ImportError::new(path, id, e).with_option_artifact(prev_artifact);
                    return Err(error);
                }

                let removed = match &prev_artifact {
                    Some(prev) => prev.removed_dependencies(artifact.meta()),
                    None => HashSet::new(),
                };

                let asset = imported.asset.take::<I::Asset>();
                let meta = artifact.meta;
                Ok(SavedAsset::new(asset, meta, prev_artifact, removed))
            },
            load: |artifact| {
                let asset = I::Saver::load(artifact.asset());

                Ok(LoadedAsset::new(asset, artifact.meta))
            },
        }
    }

    pub fn set_processer<P: AssetProcessor>(&mut self) {
        self.process = Some(|path, imported, assets| {
            let (asset, metadata, artifact) = imported.mutate();

            let mut ctx = ProcessContext::new(assets, &metadata, artifact.dependencies());

            P::process(asset, &mut ctx).map_err(|e| ImportError::new(path, metadata.id(), e))
        });
    }

    pub fn import(&self, fs: &AssetFileSystem, path: &Path) -> Result<ImportedAsset, ImportError> {
        (self.import)(fs, path)
    }

    pub fn save(
        &self,
        fs: &AssetFileSystem,
        path: &Path,
        imported: ImportedAsset,
    ) -> Result<SavedAsset, ImportError> {
        (self.save)(fs, path, imported)
    }

    pub fn load(&self, artifact: Artifact) -> std::io::Result<LoadedAsset> {
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

#[derive(Debug)]
pub struct CustomError(String);

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Custom error: {}", self.0)
    }
}

impl<I: AsRef<str>> From<I> for CustomError {
    fn from(error: I) -> Self {
        CustomError(error.as_ref().to_string())
    }
}

impl Error for CustomError {}

#[derive(Default)]
struct DependentUpdates {
    added: HashSet<AssetId>,
    removed: HashSet<AssetId>,
}

impl DependentUpdates {
    fn new() -> Self {
        DependentUpdates {
            added: HashSet::new(),
            removed: HashSet::new(),
        }
    }

    fn add(&mut self, id: AssetId) {
        self.added.insert(id);
    }

    fn remove(&mut self, id: AssetId) {
        self.removed.insert(id);
    }

    pub fn added(&self) -> &HashSet<AssetId> {
        &self.added
    }

    pub fn removed(&self) -> &HashSet<AssetId> {
        &self.removed
    }
}

fn import_asset(
    path: &Path,
    fs: &AssetFileSystem,
    db: &AssetDatabase,
    assets: &mut AssetStore,
) -> Result<SavedAsset, ImportError> {
    let ext = path.ext().ok_or(ImportError::new(
        path,
        AssetId::default(),
        CustomError::from("No extension."),
    ))?;

    let importers = db.importers();
    let importer = importers.importer_by_ext(ext).ok_or(ImportError::new(
        path,
        AssetId::default(),
        CustomError::from("No importer found for extension"),
    ))?;

    let mut imported = importer.import(fs, path)?;

    if let Some(process) = importer.process {
        load_import_dependencies(imported.artifact.dependencies(), fs, db, assets);
        process(path, &mut imported, assets)?;
    }

    importer.save(fs, path, imported)
}

fn full_import<A: AsRef<Path>>(paths: &[A], fs: &AssetFileSystem, db: &AssetDatabase) {
    let mut dependents = HashSet::new();
    dependents.extend(chunked_import(paths, fs, db));

    while !dependents.is_empty() {
        let paths = dependents
            .drain()
            .filter_map(|id| db.library().id_path(&id).cloned())
            .collect::<Vec<_>>();
        dependents.extend(chunked_import(&paths, fs, db));
    }
}

fn chunked_import<A: AsRef<Path>>(
    paths: &[A],
    fs: &AssetFileSystem,
    db: &AssetDatabase,
) -> impl IntoIterator<Item = AssetId> {
    let mut dependents = HashSet::new();

    paths.chunks(250).for_each(|paths| {
        let ids = import_assets(paths, fs, db);
        dependents.extend(ids);
    });

    dependents
}

fn import_assets<A: AsRef<Path>>(
    paths: &[A],
    fs: &AssetFileSystem,
    db: &AssetDatabase,
) -> impl IntoIterator<Item = AssetId> {
    let mut assets = AssetStore::new();
    let mut dep_updates = HashMap::new();

    for path in paths {
        let saved = match import_asset(path.as_ref(), fs, db, &mut assets) {
            Ok(saved) => saved,
            Err(_) => continue,
        };

        for id in saved.meta.dependencies() {
            let updates = dep_updates.entry(*id).or_insert_with(DependentUpdates::new);
            updates.add(saved.meta.id());
        }

        for id in saved.removed_dependencies {
            let updates = dep_updates.entry(id).or_insert_with(DependentUpdates::new);
            updates.remove(saved.meta.id());
        }
    }

    let mut returned = HashSet::<AssetId>::new();
    for (id, updates) in dep_updates {
        let _ = update_dependents(id, fs, &updates);
        returned.extend(updates.added());
    }

    returned
}

fn load_import_dependencies<'a>(
    ids: impl IntoIterator<Item = &'a AssetId>,
    fs: &AssetFileSystem,
    db: &AssetDatabase,
    assets: &mut AssetStore,
) {
    let importers = db.importers();

    for id in ids {
        if assets.contains(id) {
            return;
        }

        let artifact = match fs.load_artifact(id) {
            Ok(artifact) => artifact,
            Err(_) => continue,
        };

        let importer = match importers.importer(artifact.meta().ty()) {
            Some(importer) => importer,
            None => continue,
        };

        match importer.load(artifact) {
            Ok(loaded) => assets.insert(*id, loaded),
            Err(_) => continue,
        };
    }
}

fn update_dependents(
    id: AssetId,
    fs: &AssetFileSystem,
    updates: &DependentUpdates,
) -> Result<(), AssetIoError> {
    let path = fs.config().temp().join("dependents").join(id.to_string());
    let bytes = fs.read(&path)?;

    let mut dependents = HashSet::<AssetId>::from_bytes(&bytes).unwrap_or_default();
    dependents.extend(updates.added());
    dependents.retain(|id| !updates.removed().contains(id));

    if !dependents.is_empty() {
        let bytes = dependents.into_bytes();
        fs.write(&path, &bytes)
    } else {
        fs.remove(&path).map(|_| ())
    }
}
