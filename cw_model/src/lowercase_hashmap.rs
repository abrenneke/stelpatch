use std::collections::HashMap;
use std::collections::hash_map::{Entry, Iter, Keys, Values, ValuesMut};
use std::ops::{Index, IndexMut};

/// A HashMap wrapper that automatically converts string keys to lowercase
/// for case-insensitive key operations
#[derive(Debug, Clone, PartialEq)]
pub struct LowerCaseHashMap<V> {
    inner: HashMap<String, V>,
}

impl<V> LowerCaseHashMap<V> {
    /// Create a new empty LowerCaseHashMap
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Create a new LowerCaseHashMap with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(capacity),
        }
    }

    /// Insert a key-value pair, converting the key to lowercase
    pub fn insert<K: AsRef<str>>(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key.as_ref().to_lowercase(), value)
    }

    /// Get a value by key, converting the key to lowercase for lookup
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&V> {
        self.inner.get(&key.as_ref().to_lowercase())
    }

    /// Get a mutable reference to a value by key, converting the key to lowercase for lookup
    pub fn get_mut<K: AsRef<str>>(&mut self, key: K) -> Option<&mut V> {
        self.inner.get_mut(&key.as_ref().to_lowercase())
    }

    /// Remove a key-value pair, converting the key to lowercase for lookup
    pub fn remove<K: AsRef<str>>(&mut self, key: K) -> Option<V> {
        self.inner.remove(&key.as_ref().to_lowercase())
    }

    /// Check if the map contains a key, converting the key to lowercase for lookup
    pub fn contains_key<K: AsRef<str>>(&self, key: K) -> bool {
        self.inner.contains_key(&key.as_ref().to_lowercase())
    }

    /// Get an entry for the given key, converting the key to lowercase
    pub fn entry<K: AsRef<str>>(&mut self, key: K) -> Entry<'_, String, V> {
        self.inner.entry(key.as_ref().to_lowercase())
    }

    /// Return the number of elements in the map
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all elements from the map
    pub fn clear(&mut self) {
        self.inner.clear()
    }

    /// Get an iterator over the keys (which are all lowercase)
    pub fn keys(&self) -> Keys<'_, String, V> {
        self.inner.keys()
    }

    /// Get an iterator over the values
    pub fn values(&self) -> Values<'_, String, V> {
        self.inner.values()
    }

    /// Get a mutable iterator over the values
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, V> {
        self.inner.values_mut()
    }

    /// Get an iterator over key-value pairs
    pub fn iter(&self) -> Iter<'_, String, V> {
        self.inner.iter()
    }

    /// Convert into the underlying HashMap
    pub fn into_inner(self) -> HashMap<String, V> {
        self.inner
    }

    /// Get a reference to the underlying HashMap
    pub fn as_inner(&self) -> &HashMap<String, V> {
        &self.inner
    }

    /// Get a mutable reference to the underlying HashMap
    pub fn as_inner_mut(&mut self) -> &mut HashMap<String, V> {
        &mut self.inner
    }
}

impl<V> Default for LowerCaseHashMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> FromIterator<(String, V)> for LowerCaseHashMap<V> {
    fn from_iter<T: IntoIterator<Item = (String, V)>>(iter: T) -> Self {
        let mut map = Self::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<K: AsRef<str>, V: Clone> From<&[(K, V)]> for LowerCaseHashMap<V> {
    fn from(slice: &[(K, V)]) -> Self {
        let mut map = Self::new();
        for (key, value) in slice {
            map.insert(key.as_ref(), value.clone());
        }
        map
    }
}

impl<K: AsRef<str>, V, const N: usize> From<[(K, V); N]> for LowerCaseHashMap<V> {
    fn from(arr: [(K, V); N]) -> Self {
        let mut map = Self::new();
        for (key, value) in arr {
            map.insert(key.as_ref(), value);
        }
        map
    }
}

impl<K: AsRef<str>, V> Extend<(K, V)> for LowerCaseHashMap<V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key.as_ref(), value);
        }
    }
}

impl<'a, K: AsRef<str>, V: Clone> Extend<(&'a K, &'a V)> for LowerCaseHashMap<V> {
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key.as_ref(), value.clone());
        }
    }
}

impl<K: AsRef<str>, V> Index<K> for LowerCaseHashMap<V> {
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}

impl<K: AsRef<str>, V> IndexMut<K> for LowerCaseHashMap<V> {
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        self.get_mut(key).expect("no entry found for key")
    }
}

impl<V> IntoIterator for LowerCaseHashMap<V> {
    type Item = (String, V);
    type IntoIter = std::collections::hash_map::IntoIter<String, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a LowerCaseHashMap<V> {
    type Item = (&'a String, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, String, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, V> IntoIterator for &'a mut LowerCaseHashMap<V> {
    type Item = (&'a String, &'a mut V);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_insensitive_operations() {
        let mut map = LowerCaseHashMap::new();

        // Insert with different cases
        map.insert("Hello", "world");
        map.insert("WORLD", "hello");

        // Get with different cases should work
        assert_eq!(map.get("hello"), Some(&"world"));
        assert_eq!(map.get("HELLO"), Some(&"world"));
        assert_eq!(map.get("world"), Some(&"hello"));
        assert_eq!(map.get("World"), Some(&"hello"));

        // Contains key with different cases
        assert!(map.contains_key("hello"));
        assert!(map.contains_key("HELLO"));
        assert!(map.contains_key("world"));
        assert!(map.contains_key("WORLD"));

        // Keys should be stored in lowercase
        let keys: Vec<_> = map.keys().cloned().collect();
        assert!(keys.contains(&"hello".to_string()));
        assert!(keys.contains(&"world".to_string()));
    }

    #[test]
    fn test_overwrite_with_different_case() {
        let mut map = LowerCaseHashMap::new();

        map.insert("Key", "value1");
        map.insert("KEY", "value2"); // Should overwrite

        assert_eq!(map.get("key"), Some(&"value2"));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_remove_with_different_case() {
        let mut map = LowerCaseHashMap::new();

        map.insert("Remove", "me");
        assert_eq!(map.remove("REMOVE"), Some("me"));
        assert!(map.is_empty());
    }

    #[test]
    fn test_entry_api() {
        let mut map = LowerCaseHashMap::new();

        // Insert via entry
        map.entry("Key").or_insert("value");
        assert_eq!(map.get("KEY"), Some(&"value"));

        // Modify via entry
        *map.entry("key").or_insert("default") = "modified";
        assert_eq!(map.get("Key"), Some(&"modified"));
    }
}
