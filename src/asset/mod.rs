use bytes::IntoBytes;
use serde::ser::SerializeStruct;
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

pub mod artifact;
pub mod bytes;
pub mod config;
pub mod database;
pub mod io;

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
