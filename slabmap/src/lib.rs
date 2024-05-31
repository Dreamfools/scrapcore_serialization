use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::rc::Rc;

#[cfg(feature = "serde")]
use serde::Deserializer;

use mem::{Ref, Wrapper};

mod mem;

#[derive(Debug)]
pub struct SlabMapId<V>(usize, PhantomData<V>);

impl<V> SlabMapId<V> {
    fn new(id: usize) -> Self {
        Self(id, Default::default())
    }

    pub fn raw(&self) -> usize {
        self.0
    }

    pub fn as_untyped(&self) -> SlabMapUntypedId {
        SlabMapUntypedId::new(self.0)
    }
}

impl<V> PartialEq for SlabMapId<V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<V> Eq for SlabMapId<V> {}

impl<V> Hash for SlabMapId<V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> Clone for SlabMapId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SlabMapId<T> {}

impl<V> nohash_hasher::IsEnabled for SlabMapId<V> {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SlabMapUntypedId(usize);

impl SlabMapUntypedId {
    fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn raw(&self) -> usize {
        self.0
    }

    /// Performs unchecked conversion into a typed slab map ID
    ///
    /// Indexing directly with a resulting ID results in undesired and
    /// undocumented behavior if the original ID did not belong to the
    /// indexed SlabMap
    pub fn as_typed_unchecked<T>(&self) -> SlabMapId<T> {
        SlabMapId::new(self.0)
    }

    /// Performs unchecked conversion from a raw slab index to a SlabMap key
    ///
    /// Indexing directly with a resulting ID results in undesired and
    /// undocumented behavior if the original ID did not belong to the
    /// indexed SlabMap
    pub fn from_raw_unchecked(value: usize) -> SlabMapUntypedId {
        SlabMapUntypedId::new(value)
    }
}

impl Hash for SlabMapUntypedId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl nohash_hasher::IsEnabled for SlabMapUntypedId {}

#[derive(Debug, Copy, Clone)]
pub enum SlabMapKeyOrId<K, V> {
    Id(SlabMapId<V>),
    Key(K),
}

impl<K, V> From<SlabMapId<V>> for SlabMapKeyOrId<K, V> {
    fn from(value: SlabMapId<V>) -> Self {
        SlabMapKeyOrId::Id(value)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SlabMapKeyOrUntypedId<K> {
    Id(SlabMapUntypedId),
    Key(K),
}

impl<K> From<SlabMapUntypedId> for SlabMapKeyOrUntypedId<K> {
    fn from(value: SlabMapUntypedId) -> Self {
        SlabMapKeyOrUntypedId::Id(value)
    }
}

#[derive(Debug, Clone)]
pub struct SlabMap<K: Eq + Hash, V, Hasher: BuildHasher = BuildHasherDefault<ahash::AHasher>> {
    items: Vec<V>,
    ids: HashMap<Ref<K>, usize, Hasher>,
    keys: Vec<Ref<K>>,
}

// safe because internal Rcs are not exposed by the api and the reference counts
// only change in methods with &mut self
unsafe impl<K: Eq + Hash + Send, V: Send, Hasher: BuildHasher + Send> Send
    for SlabMap<K, V, Hasher>
{
}
unsafe impl<K: Eq + Hash + Sync, V: Sync, Hasher: BuildHasher + Sync> Sync
    for SlabMap<K, V, Hasher>
{
}

#[derive(Debug)]
pub struct SlabMapDuplicateError<K, V>(pub K, pub V);

impl<K: Eq + Hash, V, Hasher: BuildHasher> SlabMap<K, V, Hasher> {
    pub fn insert(&mut self, key: K, value: V) -> (SlabMapId<V>, Option<V>) {
        match self.ids.get(Wrapper::wrap(&key)) {
            None => {
                let id = self.items.len();
                self.items.push(value);

                self.add_key_mapping(key, id);

                (SlabMapId::new(id), None)
            }
            Some(id) => {
                let mut old = value;
                std::mem::swap(&mut self.items[*id], &mut old);
                (SlabMapId::new(*id), Some(old))
            }
        }
    }

    pub fn insert_with_id(
        &mut self,
        key: K,
        item: impl FnOnce(SlabMapId<V>) -> V,
    ) -> (SlabMapId<V>, Option<V>) {
        match self.ids.get(Wrapper::wrap(&key)) {
            None => {
                let id = self.items.len();
                let id = SlabMapId::<V>::new(id);
                let item = item(id);
                self.items.push(item);

                self.add_key_mapping(key, id.0);

                (id, None)
            }
            Some(id) => {
                let slab_id = SlabMapId::<V>::new(*id);
                let mut old = item(slab_id);
                std::mem::swap(&mut self.items[*id], &mut old);
                (slab_id, Some(old))
            }
        }
    }

    pub fn insert_new(
        &mut self,
        key: K,
        value: V,
    ) -> Result<SlabMapId<V>, SlabMapDuplicateError<K, V>> {
        if self.ids.contains_key(Wrapper::wrap(&key)) {
            return Err(SlabMapDuplicateError(key, value));
        }
        let id = self.items.len();
        self.items.push(value);

        self.add_key_mapping(key, id);

        Ok(SlabMapId::new(id))
    }

    pub fn insert_new_with_id(
        &mut self,
        key: K,
        item: impl FnOnce(SlabMapId<V>) -> V,
    ) -> Result<SlabMapId<V>, SlabMapDuplicateError<K, V>> {
        if let Some(id) = self.ids.get(Wrapper::wrap(&key)) {
            return Err(SlabMapDuplicateError(key, item(SlabMapId::new(*id))));
        }

        let id = self.items.len();
        let id = SlabMapId::<V>::new(id);

        let item = item(id);
        self.items.push(item);

        self.add_key_mapping(key, id.0);

        Ok(id)
    }

    fn add_key_mapping(&mut self, key: K, id: usize) {
        let key = Ref(Rc::new(key));
        self.keys.push(key.clone());
        self.ids.insert(key, id);

        debug_assert_eq!(self.items.len(), self.keys.len());
        debug_assert_eq!(self.items.len(), self.ids.len());
    }

    pub fn get_by_id(&self, id: SlabMapId<V>) -> Option<&V> {
        self.items.get(id.0)
    }

    pub fn get_by_id_mut(&mut self, id: SlabMapId<V>) -> Option<&mut V> {
        self.items.get_mut(id.0)
    }
    pub fn contains_id(&self, id: SlabMapId<V>) -> bool {
        id.0 < self.items.len()
    }

    pub fn get_by_untyped_id(&self, id: SlabMapUntypedId) -> Option<&V> {
        self.items.get(id.0)
    }

    pub fn get_by_untyped_id_mut(&mut self, id: SlabMapUntypedId) -> Option<&mut V> {
        self.items.get_mut(id.0)
    }
    pub fn contains_untyped_id(&self, id: SlabMapUntypedId) -> bool {
        id.0 < self.items.len()
    }

    pub fn get_by_raw(&self, id: usize) -> Option<&V> {
        self.items.get(id)
    }

    pub fn get_by_raw_mut(&mut self, id: usize) -> Option<&mut V> {
        self.items.get_mut(id)
    }
    pub fn contains_raw(&self, id: usize) -> bool {
        id < self.items.len()
    }

    pub fn get_by_key<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.ids
            .get(Wrapper::wrap(key))
            .and_then(|e| self.get_by_raw(*e))
    }

    pub fn get_by_key_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.ids
            .get(Wrapper::wrap(key))
            .copied()
            .and_then(|e| self.get_by_raw_mut(e))
    }
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.ids.contains_key(Wrapper::wrap(key))
    }

    pub fn get(&self, k: SlabMapKeyOrId<K, V>) -> Option<&V> {
        match k {
            SlabMapKeyOrId::Id(id) => self.get_by_id(id),
            SlabMapKeyOrId::Key(key) => self.get_by_key(&key),
        }
    }

    pub fn get_mut(&mut self, k: SlabMapKeyOrId<K, V>) -> Option<&mut V> {
        match k {
            SlabMapKeyOrId::Id(id) => self.get_by_id_mut(id),
            SlabMapKeyOrId::Key(key) => self.get_by_key_mut(&key),
        }
    }

    pub fn get_by_untyped(&self, k: SlabMapKeyOrUntypedId<K>) -> Option<&V> {
        match k {
            SlabMapKeyOrUntypedId::Id(id) => self.get_by_raw(id.0),
            SlabMapKeyOrUntypedId::Key(key) => self.get_by_key(&key),
        }
    }

    pub fn get_by_untyped_mut(&mut self, k: SlabMapKeyOrUntypedId<K>) -> Option<&mut V> {
        match k {
            SlabMapKeyOrUntypedId::Id(id) => self.get_by_raw_mut(id.0),
            SlabMapKeyOrUntypedId::Key(key) => self.get_by_key_mut(&key),
        }
    }

    pub fn key_to_id<Q>(&self, key: &Q) -> Option<SlabMapId<V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.ids.get(Wrapper::wrap(key)).map(|e| SlabMapId::new(*e))
    }

    pub fn id_to_key(&self, id: SlabMapId<V>) -> Option<&K> {
        self.keys.get(id.0).map(|r| &*r.0)
    }

    pub fn untyped_to_key(&self, id: SlabMapUntypedId) -> Option<&K> {
        self.keys.get(id.0).map(|r| &*r.0)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.items.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.items.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (SlabMapId<V>, &V)> {
        self.items
            .iter()
            .enumerate()
            .map(|(id, e)| (SlabMapId::new(id), e))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (SlabMapId<V>, &mut V)> {
        self.items
            .iter_mut()
            .enumerate()
            .map(|(id, e)| (SlabMapId::new(id), e))
    }

    pub fn into_iter(mut self) -> impl Iterator<Item = (K, usize, V)> {
        // drop keys map to free RCs
        drop(self.ids);

        self.keys
            .into_iter()
            .zip(self.items)
            .enumerate()
            .map(|(id, (key, value))| {
                let key = Rc::into_inner(key.0).expect("Key RCs should be freed");
                (key, id, value)
            })
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter().map(|k| &*k.0)
    }

    pub fn ids(&self) -> impl Iterator<Item = SlabMapId<V>> + '_ {
        self.ids.iter().enumerate().map(|(i, _)| SlabMapId::new(i))
    }

    pub fn keys_ids(&self) -> impl Iterator<Item = (&'_ K, SlabMapId<V>)> + '_ {
        self.keys
            .iter()
            .enumerate()
            .map(|(id, k)| (&*k.0, SlabMapId::new(id)))
    }
}

impl<K: Eq + Hash, V, Hasher: BuildHasher> Index<SlabMapId<V>> for SlabMap<K, V, Hasher> {
    type Output = V;

    fn index(&self, index: SlabMapId<V>) -> &Self::Output {
        &self.items[index.0]
    }
}

impl<K: Eq + Hash, V, Hasher: BuildHasher> IndexMut<SlabMapId<V>> for SlabMap<K, V, Hasher> {
    fn index_mut(&mut self, index: SlabMapId<V>) -> &mut Self::Output {
        &mut self.items[index.0]
    }
}

impl<K: Eq + Hash, V, Hasher: BuildHasher + Default> Default for SlabMap<K, V, Hasher> {
    fn default() -> Self {
        Self {
            items: Default::default(),
            ids: Default::default(),
            keys: Default::default(),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de, K: serde::Deserialize<'de>, V> serde::Deserialize<'de> for SlabMapKeyOrId<K, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        K::deserialize(deserializer).map(|k| Self::Key(k))
    }
}
