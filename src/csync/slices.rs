//! Thread-safe slice implementations

use std::sync::{Arc, RwLock};
use std::thread;

/// Lazy-loaded thread-safe slice
pub struct LazySlice<T> {
    inner: Arc<RwLock<Option<Vec<T>>>>,
    loader: Arc<RwLock<Option<Box<dyn FnOnce() -> Vec<T> + Send>>>>,
}

impl<T> LazySlice<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new lazy slice with a loader function
    pub fn new<F>(loader: F) -> Self
    where
        F: FnOnce() -> Vec<T> + Send + 'static,
    {
        let inner = Arc::new(RwLock::new(None));
        let inner_clone = Arc::clone(&inner);
        
        // Start loading in background
        thread::spawn(move || {
            let data = loader();
            let mut guard = inner_clone.write().unwrap();
            *guard = Some(data);
        });
        
        Self {
            inner,
            loader: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Get the slice, blocking until loaded
    pub fn get(&self) -> Vec<T> {
        loop {
            let guard = self.inner.read().unwrap();
            if let Some(data) = &*guard {
                return data.clone();
            }
            drop(guard);
            thread::yield_now();
        }
    }
    
    /// Check if the slice is loaded
    pub fn is_loaded(&self) -> bool {
        let guard = self.inner.read().unwrap();
        guard.is_some()
    }
    
    /// Iterate over elements
    pub fn iter<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let data = self.get();
        for item in &data {
            f(item);
        }
    }
}

/// Thread-safe slice implementation
pub struct Slice<T> {
    inner: Arc<RwLock<Vec<T>>>,
}

impl<T> Slice<T>
where
    T: Clone,
{
    /// Create a new empty slice
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create a slice from an existing Vec
    pub fn from(vec: Vec<T>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(vec)),
        }
    }
    
    /// Append elements to the slice
    pub fn append(&self, items: &[T]) {
        let mut slice = self.inner.write().unwrap();
        slice.extend_from_slice(items);
    }
    
    /// Prepend an element to the slice
    pub fn prepend(&self, item: T) {
        let mut slice = self.inner.write().unwrap();
        slice.insert(0, item);
    }
    
    /// Delete element at index
    pub fn delete(&self, index: usize) -> bool {
        let mut slice = self.inner.write().unwrap();
        if index < slice.len() {
            slice.remove(index);
            true
        } else {
            false
        }
    }
    
    /// Get element at index
    pub fn get(&self, index: usize) -> Option<T> {
        let slice = self.inner.read().unwrap();
        slice.get(index).cloned()
    }
    
    /// Set element at index
    pub fn set(&self, index: usize, item: T) -> bool {
        let mut slice = self.inner.write().unwrap();
        if index < slice.len() {
            slice[index] = item;
            true
        } else {
            false
        }
    }
    
    /// Get the length of the slice
    pub fn len(&self) -> usize {
        let slice = self.inner.read().unwrap();
        slice.len()
    }
    
    /// Check if the slice is empty
    pub fn is_empty(&self) -> bool {
        let slice = self.inner.read().unwrap();
        slice.is_empty()
    }
    
    /// Replace the entire slice
    pub fn set_slice(&self, items: Vec<T>) {
        let mut slice = self.inner.write().unwrap();
        *slice = items;
    }
    
    /// Get a copy of the entire slice
    pub fn to_vec(&self) -> Vec<T> {
        let slice = self.inner.read().unwrap();
        slice.clone()
    }
    
    /// Iterate over elements
    pub fn iter<F>(&self, mut f: F)
    where
        F: FnMut(&T),
    {
        let slice = self.inner.read().unwrap();
        for item in slice.iter() {
            f(item);
        }
    }
    
    /// Iterate with index
    pub fn iter_with_index<F>(&self, mut f: F)
    where
        F: FnMut(usize, &T),
    {
        let slice = self.inner.read().unwrap();
        for (i, item) in slice.iter().enumerate() {
            f(i, item);
        }
    }
    
    /// Clear the slice
    pub fn clear(&self) {
        let mut slice = self.inner.write().unwrap();
        slice.clear();
    }
    
    /// Push an element to the end
    pub fn push(&self, item: T) {
        let mut slice = self.inner.write().unwrap();
        slice.push(item);
    }
    
    /// Pop an element from the end
    pub fn pop(&self) -> Option<T> {
        let mut slice = self.inner.write().unwrap();
        slice.pop()
    }
}

impl<T> Clone for Slice<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Default for Slice<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Create a Slice from a Vec
pub fn SliceFrom<T>(vec: Vec<T>) -> Slice<T>
where
    T: Clone,
{
    Slice::from(vec)
}