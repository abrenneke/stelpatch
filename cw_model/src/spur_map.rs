use nohash_hasher::{BuildNoHashHasher, IsEnabled};
use std::collections::HashMap;
use std::collections::hash_map::{Entry, IntoValues, Values, ValuesMut};

use lasso::Spur;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct InnerSpur(pub Spur);

impl IsEnabled for InnerSpur {}

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct SpurMap<T>(HashMap<InnerSpur, T, BuildNoHashHasher<InnerSpur>>);

// Iterator wrapper types
pub struct SpurMapIntoIter<T> {
    inner: std::collections::hash_map::IntoIter<InnerSpur, T>,
}

impl<T> Iterator for SpurMapIntoIter<T> {
    type Item = (Spur, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(inner_spur, value)| (inner_spur.0, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct SpurMapIter<'a, T> {
    inner: std::collections::hash_map::Iter<'a, InnerSpur, T>,
}

impl<'a, T> Iterator for SpurMapIter<'a, T> {
    type Item = (Spur, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(inner_spur, value)| (inner_spur.0, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct SpurMapIterMut<'a, T> {
    inner: std::collections::hash_map::IterMut<'a, InnerSpur, T>,
}

impl<'a, T> Iterator for SpurMapIterMut<'a, T> {
    type Item = (Spur, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(inner_spur, value)| (inner_spur.0, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> SpurMap<T> {
    /// Creates an empty `SpurMap`.
    pub fn new() -> Self {
        Self(HashMap::with_hasher(
            BuildNoHashHasher::<InnerSpur>::default(),
        ))
    }

    /// Creates an empty `SpurMap` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity_and_hasher(
            capacity,
            BuildNoHashHasher::<InnerSpur>::default(),
        ))
    }

    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: Spur, value: T) -> Option<T> {
        self.0.insert(InnerSpur(key), value)
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: &Spur) -> Option<&T> {
        self.0.get(&InnerSpur(*key))
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut(&mut self, key: &Spur) -> Option<&mut T> {
        self.0.get_mut(&InnerSpur(*key))
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    pub fn remove(&mut self, key: &Spur) -> Option<T> {
        self.0.remove(&InnerSpur(*key))
    }

    /// Returns `true` if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &Spur) -> bool {
        self.0.contains_key(&InnerSpur(*key))
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clears the map, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// An iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = Spur> + '_ {
        self.0.keys().map(|inner_spur| inner_spur.0)
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> Values<'_, InnerSpur, T> {
        self.0.values()
    }

    /// An iterator visiting all values mutably in arbitrary order.
    pub fn values_mut(&mut self) -> ValuesMut<'_, InnerSpur, T> {
        self.0.values_mut()
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    pub fn iter(&self) -> SpurMapIter<'_, T> {
        SpurMapIter {
            inner: self.0.iter(),
        }
    }

    /// An iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
    pub fn iter_mut(&mut self) -> SpurMapIterMut<'_, T> {
        SpurMapIterMut {
            inner: self.0.iter_mut(),
        }
    }

    /// Gets the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: Spur) -> Entry<'_, InnerSpur, T> {
        self.0.entry(InnerSpur(key))
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(Spur, &mut T) -> bool,
    {
        self.0.retain(|inner_spur, value| f(inner_spur.0, value))
    }

    /// Returns the key-value pair corresponding to the supplied key.
    pub fn get_key_value(&self, key: &Spur) -> Option<(Spur, &T)> {
        self.0
            .get_key_value(&InnerSpur(*key))
            .map(|(inner_spur, value)| (inner_spur.0, value))
    }

    /// Removes a key from the map, returning the stored key and value if the key was previously in the map.
    pub fn remove_entry(&mut self, key: &Spur) -> Option<(Spur, T)> {
        self.0
            .remove_entry(&InnerSpur(*key))
            .map(|(inner_spur, value)| (inner_spur.0, value))
    }

    /// Creates a consuming iterator visiting all the keys in arbitrary order.
    pub fn into_keys(self) -> impl Iterator<Item = Spur> {
        self.0.into_keys().map(|inner_spur| inner_spur.0)
    }

    /// Creates a consuming iterator visiting all the values in arbitrary order.
    pub fn into_values(self) -> IntoValues<InnerSpur, T> {
        self.0.into_values()
    }

    /// Shrinks the capacity of the map as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }

    /// Shrinks the capacity of the map with a lower limit.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.0.shrink_to(min_capacity)
    }

    /// Returns the number of elements the map can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    /// Tries to reserve capacity for at least `additional` more elements to be inserted.
    pub fn try_reserve(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.0.try_reserve(additional)
    }

    /// Extends a collection with the contents of an iterator.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (Spur, T)>,
    {
        self.0.extend(
            iter.into_iter()
                .map(|(spur, value)| (InnerSpur(spur), value)),
        )
    }
}

impl<T> Default for SpurMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Consuming iterator - takes ownership of SpurMap
impl<T> IntoIterator for SpurMap<T> {
    type Item = (Spur, T);
    type IntoIter = SpurMapIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        SpurMapIntoIter {
            inner: self.0.into_iter(),
        }
    }
}

// Borrowing iterator - iterates over references
impl<'a, T> IntoIterator for &'a SpurMap<T> {
    type Item = (Spur, &'a T);
    type IntoIter = SpurMapIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Mutable borrowing iterator - iterates over mutable references to values
impl<'a, T> IntoIterator for &'a mut SpurMap<T> {
    type Item = (Spur, &'a mut T);
    type IntoIter = SpurMapIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
