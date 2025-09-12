//! Thread-safe map implementation

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Thread-safe map implementation
pub struct Map<K, V> {
    inner: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> Map<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new empty map
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a map from an existing HashMap
    pub fn from(map: HashMap<K, V>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(map)),
        }
    }
    
    /// Set a value for the given key
    pub fn set(&self, key: K, value: V) {
        let mut map = self.inner.write().unwrap();
        map.insert(key, value);
    }
    
    /// Get a value for the given key
    pub fn get(&self, key: &K) -> Option<V> {
        let map = self.inner.read().unwrap();
        map.get(key).cloned()
    }
    
    /// Delete a key from the map
    pub fn del(&self, key: &K) {
        let mut map = self.inner.write().unwrap();
        map.remove(key);
    }
    
    /// Get the number of items in the map
    pub fn len(&self) -> usize {
        let map = self.inner.read().unwrap();
        map.len()
    }
    
    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        let map = self.inner.read().unwrap();
        map.is_empty()
    }
    
    /// Get or set a value using the provided function
    pub fn get_or_set<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        // Try to get first with read lock
        {
            let map = self.inner.read().unwrap();
            if let Some(value) = map.get(&key) {
                return value.clone();
            }
        }
        
        // Need to set, acquire write lock
        let mut map = self.inner.write().unwrap();
        // Check again in case another thread set it
        if let Some(value) = map.get(&key) {
            return value.clone();
        }
        
        let value = f();
        map.insert(key.clone(), value.clone());
        value
    }
    
    /// Take a value (get and delete)
    pub fn take(&self, key: &K) -> Option<V> {
        let mut map = self.inner.write().unwrap();
        map.remove(key)
    }
    
    /// Iterate over key-value pairs
    pub fn iter<F>(&self, mut f: F)
    where
        F: FnMut(&K, &V),
    {
        let map = self.inner.read().unwrap();
        for (k, v) in map.iter() {
            f(k, v);
        }
    }
    
    /// Get all keys
    pub fn keys(&self) -> Vec<K> {
        let map = self.inner.read().unwrap();
        map.keys().cloned().collect()
    }
    
    /// Get all values
    pub fn values(&self) -> Vec<V> {
        let map = self.inner.read().unwrap();
        map.values().cloned().collect()
    }
    
    /// Clear the map
    pub fn clear(&self) {
        let mut map = self.inner.write().unwrap();
        map.clear();
    }
}

impl<K, V> Clone for Map<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K, V> Default for Map<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Create a Map from a HashMap
pub fn MapFrom<K, V>(map: HashMap<K, V>) -> Map<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    Map::from(map)
}

// JSON serialization support for Map<String, V>
impl<V> Serialize for Map<String, V>
where
    V: Serialize + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let map = self.inner.read().unwrap();
        map.serialize(serializer)
    }
}

impl<'de, V> Deserialize<'de> for Map<String, V>
where
    V: Deserialize<'de> + Clone,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<String, V>::deserialize(deserializer)?;
        Ok(Map::from(map))
    }
}