use std::collections::hash_map::Entry;

use crate::registry::entry::RegistryEntrySerialized;
use crate::registry::path_identifier::PathIdentifier;
use crate::registry::{
    poison_on_err, AssetsHolder, MaybeRawItem, MaybeRawSingleton, PartialCollectionHolder,
    PartialRegistry, PartialSingletonHolder,
};
use crate::serialization::error::{
    DeserializationError, DeserializationErrorKind, DeserializationErrorStackItem,
};
use crate::serialization::migrate::Migrate;
use crate::serialization::SerializationFallback;

/// Insert a raw item into a registry, returning an error if the item with the
/// same ID is already added
pub fn registry_insert<
    T: SerializationFallback,
    V: Migrate<T::Fallback, Registry>,
    Registry: PartialCollectionHolder<T>,
>(
    registry: &mut Registry,
    path: impl Into<PathIdentifier>,
    item: RegistryEntrySerialized<V>,
) -> Result<(), DeserializationError<Registry>> {
    poison_on_err(registry, |registry| {
        let item = item.migrate(registry)?;
        let path = path.into();
        let raw = registry.get_collection();
        if let Some(entry) = raw.get_by_key(&item.id) {
            return Err(DeserializationErrorKind::DuplicateItem {
                id: item.id.clone(),
                kind: Registry::kind(),
                path_a: entry.0.clone(),
                path_b: path.clone(),
            }
            .into_err()
            .context(DeserializationErrorStackItem::ItemByPath(
                path,
                Registry::kind(),
            )));
        }
        raw.insert(item.id.clone(), (path, MaybeRawItem::Raw(item)));
        Ok(())
    })
}

/// Insert a raw singleton into a registry, returning an error if singleton of
/// the same type is already added
pub fn singleton_insert<T: SerializationFallback, Registry: PartialSingletonHolder<T>>(
    registry: &mut Registry,
    path: impl Into<PathIdentifier>,
    item: T::Fallback,
) -> Result<(), DeserializationError<Registry>> {
    poison_on_err(registry, |registry| {
        let path = path.into();
        let entry = registry.get_singleton();

        if let Some((path_b, _)) = entry.take() {
            return Err(DeserializationErrorKind::DuplicateSingleton {
                kind: Registry::kind(),
                path_a: path_b,
                path_b: path.clone(),
            }
            .into_err()
            .context(DeserializationErrorStackItem::ItemByPath(
                path,
                Registry::kind(),
            )));
        } else {
            *entry = Some((path, MaybeRawSingleton::Raw(item)))
        }

        Ok(())
    })
}

/// Insert an asset into a registry, returning an error if the asset with
/// the same file name is already added
pub fn asset_insert<T, Registry: PartialRegistry + AssetsHolder<T>>(
    registry: &mut Registry,
    path: PathIdentifier,
    item: T,
) -> Result<(), DeserializationError<Registry>> {
    poison_on_err(registry, |registry| {
        let Some(name) = path.file_name() else {
            return Err(DeserializationErrorKind::MissingName(path).into());
        };

        let Some(name) = name.to_str() else {
            return Err(DeserializationErrorKind::NonUtf8Path(path).into());
        };
        let name = name.to_ascii_lowercase();
        let assets = registry.get_assets_mut();
        match assets.entry(name.clone()) {
            Entry::Occupied(entry) => Err(DeserializationErrorKind::DuplicateAsset {
                kind: Registry::asset_kind(),
                name,
                path_a: entry.get().1.clone(),
                path_b: path,
            }
            .into()),
            Entry::Vacant(entry) => {
                entry.insert((item, path));
                Ok(())
            }
        }
    })
}
