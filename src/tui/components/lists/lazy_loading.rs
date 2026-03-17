//! Lazy loading implementation for high-performance list rendering.
//!
//! This module provides lazy loading capabilities that defer item rendering
//! and data fetching until items become visible, dramatically improving
//! performance for large datasets.

use super::ListItem;
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

/// Lazy loading manager for list components
pub struct LazyLoader<T: ListItem> {
    /// Configuration for lazy loading behavior
    config: LazyLoadConfig,

    /// Cache of loaded items
    item_cache: Arc<RwLock<HashMap<String, CachedItem<T>>>>,

    /// Queue of items to load
    load_queue: VecDeque<LoadRequest>,

    /// Currently loading items
    loading_items: HashMap<String, LoadingState>,

    /// Item provider for fetching data
    item_provider: Option<Arc<dyn ItemProvider<T>>>,

    /// Placeholder generator
    placeholder_generator: Option<Arc<dyn PlaceholderGenerator<T>>>,

    /// Loading state callbacks
    state_callbacks: Vec<Arc<dyn Fn(LazyLoadEvent) + Send + Sync>>,

    /// Performance metrics
    metrics: LazyLoadMetrics,

    /// Background task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,

    /// Channel for communicating with background task
    load_sender: Option<mpsc::UnboundedSender<LoadRequest>>,
}

impl<T: ListItem> std::fmt::Debug for LazyLoader<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyLoader")
            .field("config", &self.config)
            .field("load_queue_len", &self.load_queue.len())
            .field("loading_items", &self.loading_items.len())
            .field("metrics", &self.metrics)
            .finish()
    }
}

/// Configuration for lazy loading behavior
#[derive(Debug, Clone)]
pub struct LazyLoadConfig {
    /// Maximum number of items to keep in cache
    pub max_cache_size: usize,
    
    /// Number of items to preload ahead of current position
    pub preload_count: usize,
    
    /// Number of items to preload behind current position
    pub preload_behind_count: usize,
    
    /// Maximum time to keep items in cache
    pub cache_ttl: Duration,
    
    /// Maximum concurrent loading operations
    pub max_concurrent_loads: usize,
    
    /// Timeout for individual load operations
    pub load_timeout: Duration,
    
    /// Whether to enable background preloading
    pub background_preloading: bool,
    
    /// Debounce delay for load requests
    pub load_debounce: Duration,
    
    /// Whether to cache failed loads to avoid retries
    pub cache_failures: bool,
    
    /// Retry configuration for failed loads
    pub retry_config: RetryConfig,
}

/// Retry configuration for failed loads
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    
    /// Base delay between retries
    pub base_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
}

impl Default for LazyLoadConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            preload_count: 20,
            preload_behind_count: 10,
            cache_ttl: Duration::from_secs(300), // 5 minutes
            max_concurrent_loads: 5,
            load_timeout: Duration::from_secs(10),
            background_preloading: true,
            load_debounce: Duration::from_millis(100),
            cache_failures: true,
            retry_config: RetryConfig {
                max_attempts: 3,
                base_delay: Duration::from_millis(500),
                backoff_multiplier: 2.0,
                max_delay: Duration::from_secs(30),
            },
        }
    }
}

/// Cached item with metadata
#[derive(Debug, Clone)]
struct CachedItem<T: ListItem> {
    item: T,
    loaded_at: Instant,
    access_count: usize,
    last_accessed: Instant,
    load_duration: Duration,
}

/// Load request for items
#[derive(Debug, Clone)]
struct LoadRequest {
    item_id: String,
    priority: LoadPriority,
    requested_at: Instant,
    retry_count: usize,
}

/// Priority levels for load requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadPriority {
    /// Low priority for background preloading
    Low = 0,
    /// Normal priority for items that will be visible soon
    Normal = 1,
    /// High priority for items that are currently visible
    High = 2,
    /// Critical priority for items that are immediately needed
    Critical = 3,
}

/// Current loading state for an item
#[derive(Debug, Clone)]
struct LoadingState {
    started_at: Instant,
    priority: LoadPriority,
    attempt: usize,
}

/// Lazy loading events
#[derive(Debug, Clone)]
pub enum LazyLoadEvent {
    /// Item loading started
    LoadStarted { item_id: String, priority: LoadPriority },
    
    /// Item loading completed successfully
    LoadCompleted { item_id: String, duration: Duration },
    
    /// Item loading failed
    LoadFailed { item_id: String, error: String, retry_count: usize },
    
    /// Cache was updated (item added/removed)
    CacheUpdated { cache_size: usize, memory_usage: usize },
    
    /// Background preloading status changed
    PreloadingStatusChanged { active: bool, queue_size: usize },
}

/// Performance metrics for lazy loading
#[derive(Debug, Clone, Default)]
pub struct LazyLoadMetrics {
    /// Total number of load requests
    pub total_requests: u64,
    
    /// Number of successful loads
    pub successful_loads: u64,
    
    /// Number of failed loads
    pub failed_loads: u64,
    
    /// Number of cache hits
    pub cache_hits: u64,
    
    /// Number of cache misses
    pub cache_misses: u64,
    
    /// Average load time in milliseconds
    pub avg_load_time_ms: f64,
    
    /// Current cache size
    pub cache_size: usize,
    
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: usize,
    
    /// Number of items currently loading
    pub items_loading: usize,
    
    /// Size of load queue
    pub queue_size: usize,
}

/// Trait for providing items to the lazy loader
pub trait ItemProvider<T: ListItem>: Send + Sync {
    /// Load a single item by ID
    fn load_item(&self, item_id: &str) -> Pin<Box<dyn Future<Output = Result<T>> + Send>>;
    
    /// Load multiple items in batch (optional optimization)
    fn load_items(&self, item_ids: &[String]) -> Pin<Box<dyn Future<Output = Result<Vec<T>>> + Send + '_>> {
        let item_ids = item_ids.to_vec();
        Box::pin(async move {
            let mut results = Vec::new();
            for id in item_ids {
                match self.load_item(&id).await {
                    Ok(item) => results.push(item),
                    Err(e) => return Err(e),
                }
            }
            Ok(results)
        })
    }
    
    /// Check if an item exists without loading it (optional optimization)
    fn item_exists(&self, item_id: &str) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let id = item_id.to_string();
        Box::pin(async move {
            // Default implementation tries to load the item
            match self.load_item(&id).await {
                Ok(_) => true,
                Err(_) => false,
            }
        })
    }
}

/// Trait for generating placeholder items while real items are loading
pub trait PlaceholderGenerator<T: ListItem>: Send + Sync {
    /// Generate a placeholder item for the given ID
    fn generate_placeholder(&self, item_id: &str) -> T;
    
    /// Generate a loading placeholder (shown while item is being loaded)
    fn generate_loading_placeholder(&self, item_id: &str) -> T {
        self.generate_placeholder(item_id)
    }
    
    /// Generate an error placeholder (shown when loading fails)
    fn generate_error_placeholder(&self, item_id: &str, _error: &str) -> T {
        self.generate_placeholder(item_id)
    }
}

impl<T: ListItem + 'static> LazyLoader<T> {
    /// Create a new lazy loader with default configuration
    pub fn new() -> Self {
        Self::with_config(LazyLoadConfig::default())
    }
    
    /// Create a new lazy loader with custom configuration
    pub fn with_config(config: LazyLoadConfig) -> Self {
        Self {
            config,
            item_cache: Arc::new(RwLock::new(HashMap::new())),
            load_queue: VecDeque::new(),
            loading_items: HashMap::new(),
            item_provider: None,
            placeholder_generator: None,
            state_callbacks: Vec::new(),
            metrics: LazyLoadMetrics::default(),
            task_handle: None,
            load_sender: None,
        }
    }
    
    /// Set the item provider
    pub fn set_item_provider<P>(&mut self, provider: P)
    where
        P: ItemProvider<T> + 'static,
    {
        self.item_provider = Some(Arc::new(provider));
    }
    
    /// Set the placeholder generator
    pub fn set_placeholder_generator<G>(&mut self, generator: G)
    where
        G: PlaceholderGenerator<T> + 'static,
    {
        self.placeholder_generator = Some(Arc::new(generator));
    }
    
    /// Add a state change callback
    pub fn add_state_callback<F>(&mut self, callback: F)
    where
        F: Fn(LazyLoadEvent) + Send + Sync + 'static,
    {
        self.state_callbacks.push(Arc::new(callback));
    }
    
    /// Start the background loading task
    pub fn start_background_task(&mut self) -> Result<()> {
        if self.task_handle.is_some() {
            return Ok(()); // Already started
        }
        
        let (sender, receiver) = mpsc::unbounded_channel();
        self.load_sender = Some(sender);
        
        let cache = Arc::clone(&self.item_cache);
        let provider = self.item_provider.clone();
        let config = self.config.clone();
        let callbacks = self.state_callbacks.clone();
        
        let handle = tokio::spawn(async move {
            Self::background_loader_task(receiver, cache, provider, config, callbacks).await;
        });
        
        self.task_handle = Some(handle);
        Ok(())
    }
    
    /// Stop the background loading task
    pub async fn stop_background_task(&mut self) -> Result<()> {
        if let Some(handle) = self.task_handle.take() {
            drop(self.load_sender.take()); // Close the channel
            handle.await?;
        }
        Ok(())
    }
    
    /// Get an item, loading it if necessary
    pub async fn get_item(&mut self, item_id: &str) -> Result<T> {
        // Check cache first
        {
            let cache = self.item_cache.read().await;
            if let Some(cached) = cache.get(item_id) {
                self.metrics.cache_hits += 1;
                self.update_access_time(item_id).await;
                return Ok(cached.item.clone());
            }
        }
        
        self.metrics.cache_misses += 1;
        
        // Check if item is currently loading
        if self.loading_items.contains_key(item_id) {
            // Return placeholder while loading
            if let Some(generator) = &self.placeholder_generator {
                return Ok(generator.generate_loading_placeholder(item_id));
            }
        }
        
        // Start loading the item
        self.request_load(item_id.to_string(), LoadPriority::Critical).await?;
        
        // Return placeholder for now
        if let Some(generator) = &self.placeholder_generator {
            Ok(generator.generate_loading_placeholder(item_id))
        } else {
            Err(anyhow::anyhow!("Item not available and no placeholder generator configured"))
        }
    }
    
    /// Get an item if it's already cached, otherwise return None
    pub async fn get_cached_item(&self, item_id: &str) -> Option<T> {
        let cache = self.item_cache.read().await;
        if let Some(cached) = cache.get(item_id) {
            self.update_access_time_sync(item_id, &cache);
            Some(cached.item.clone())
        } else {
            None
        }
    }
    
    /// Preload items around a specific position
    pub async fn preload_around(&mut self, center_item_id: &str, visible_items: &[String]) -> Result<()> {
        let mut requests = Vec::new();
        
        // High priority for visible items
        for item_id in visible_items {
            if !self.is_cached(item_id).await && !self.loading_items.contains_key(item_id) {
                requests.push(LoadRequest {
                    item_id: item_id.clone(),
                    priority: LoadPriority::High,
                    requested_at: Instant::now(),
                    retry_count: 0,
                });
            }
        }
        
        // Lower priority for preload items
        // Note: This would need access to the full item list to determine adjacent items
        // For now, this is a placeholder implementation
        
        // Send requests
        for request in requests {
            self.request_load_with_priority(request.item_id, request.priority).await?;
        }
        
        Ok(())
    }
    
    /// Request loading of an item
    async fn request_load(&mut self, item_id: String, priority: LoadPriority) -> Result<()> {
        self.request_load_with_priority(item_id, priority).await
    }
    
    /// Request loading of an item with specific priority
    async fn request_load_with_priority(&mut self, item_id: String, priority: LoadPriority) -> Result<()> {
        // Check if already loading with higher or equal priority
        if let Some(loading_state) = self.loading_items.get(&item_id) {
            if loading_state.priority >= priority {
                return Ok(());
            }
        }
        
        let request = LoadRequest {
            item_id: item_id.clone(),
            priority,
            requested_at: Instant::now(),
            retry_count: 0,
        };
        
        // Add to loading state
        self.loading_items.insert(item_id.clone(), LoadingState {
            started_at: Instant::now(),
            priority,
            attempt: 1,
        });
        
        // Send to background task if available
        if let Some(sender) = &self.load_sender {
            sender.send(request)?;
        } else {
            // Load synchronously if no background task
            self.load_item_sync(request).await?;
        }
        
        self.metrics.total_requests += 1;
        self.emit_event(LazyLoadEvent::LoadStarted { item_id, priority });
        
        Ok(())
    }
    
    /// Load an item synchronously
    async fn load_item_sync(&mut self, request: LoadRequest) -> Result<()> {
        if let Some(provider) = &self.item_provider {
            let start_time = Instant::now();
            
            match provider.load_item(&request.item_id).await {
                Ok(item) => {
                    let duration = start_time.elapsed();
                    self.cache_item(request.item_id.clone(), item, duration).await;
                    self.loading_items.remove(&request.item_id);
                    self.metrics.successful_loads += 1;
                    self.emit_event(LazyLoadEvent::LoadCompleted {
                        item_id: request.item_id,
                        duration,
                    });
                }
                Err(e) => {
                    self.loading_items.remove(&request.item_id);
                    self.metrics.failed_loads += 1;
                    self.emit_event(LazyLoadEvent::LoadFailed {
                        item_id: request.item_id,
                        error: e.to_string(),
                        retry_count: request.retry_count,
                    });
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Cache a loaded item
    async fn cache_item(&mut self, item_id: String, item: T, load_duration: Duration) {
        let cached_item = CachedItem {
            item,
            loaded_at: Instant::now(),
            access_count: 1,
            last_accessed: Instant::now(),
            load_duration,
        };

        let cache_arc = Arc::clone(&self.item_cache);
        let (cache_size, memory_usage) = {
            let mut cache = cache_arc.write().await;
            cache.insert(item_id, cached_item);

            // Clean up cache if it's too large
            let max_cache_size = self.config.max_cache_size;
            let cache_ttl = self.config.cache_ttl;
            if cache.len() > max_cache_size {
                // Inline eviction to avoid borrow conflicts
                let cutoff = Instant::now() - cache_ttl;
                let to_remove: Vec<String> = cache.iter()
                    .filter(|(_, item)| item.loaded_at < cutoff || item.last_accessed < cutoff)
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in to_remove {
                    cache.remove(&id);
                }

                // If still too many, remove least recently used
                if cache.len() > max_cache_size {
                    let mut items: Vec<(String, Instant)> = cache.iter()
                        .map(|(k, v)| (k.clone(), v.last_accessed))
                        .collect();
                    items.sort_by_key(|(_, t)| *t);
                    let excess = cache.len() - max_cache_size;
                    for (id, _) in items.iter().take(excess) {
                        cache.remove(id);
                    }
                }
            }

            let size = cache.len();
            let mem = size * 1024; // Rough estimate
            (size, mem)
        };

        // Update metrics (after releasing the write lock)
        self.metrics.cache_size = cache_size;
        self.update_avg_load_time(load_duration);

        self.emit_event(LazyLoadEvent::CacheUpdated {
            cache_size,
            memory_usage,
        });
    }
    
    /// Check if an item is cached
    async fn is_cached(&self, item_id: &str) -> bool {
        let cache = self.item_cache.read().await;
        cache.contains_key(item_id)
    }
    
    /// Update access time for an item
    async fn update_access_time(&self, item_id: &str) {
        let mut cache = self.item_cache.write().await;
        if let Some(cached) = cache.get_mut(item_id) {
            cached.last_accessed = Instant::now();
            cached.access_count += 1;
        }
    }
    
    /// Update access time synchronously (when already holding read lock)
    fn update_access_time_sync(&self, _item_id: &str, _cache: &HashMap<String, CachedItem<T>>) {
        // This would require interior mutability, but for metrics it's not critical
        // In practice, we'd use a more sophisticated caching solution
    }
    
    /// Evict old items from cache
    fn evict_old_items(&self, cache: &mut HashMap<String, CachedItem<T>>) {
        let cutoff = Instant::now() - self.config.cache_ttl;
        
        // Remove items based on TTL and access patterns
        let mut to_remove = Vec::new();
        for (id, item) in cache.iter() {
            if item.loaded_at < cutoff || item.last_accessed < cutoff {
                to_remove.push(id.clone());
            }
        }
        
        // If still too many items, remove least recently used
        if cache.len() - to_remove.len() > self.config.max_cache_size {
            let mut items: Vec<_> = cache.iter().collect();
            items.sort_by_key(|(_, item)| item.last_accessed);
            
            let excess = cache.len() - to_remove.len() - self.config.max_cache_size;
            for (id, _) in items.iter().take(excess) {
                to_remove.push((*id).clone());
            }
        }
        
        for id in to_remove {
            cache.remove(&id);
        }
    }
    
    /// Update average load time metric
    fn update_avg_load_time(&mut self, duration: Duration) {
        let new_time = duration.as_millis() as f64;
        if self.metrics.successful_loads <= 1 {
            self.metrics.avg_load_time_ms = new_time;
        } else {
            // Exponential moving average
            let alpha = 0.1; // Smoothing factor
            self.metrics.avg_load_time_ms = 
                alpha * new_time + (1.0 - alpha) * self.metrics.avg_load_time_ms;
        }
    }
    
    /// Estimate memory usage of the cache
    fn estimate_memory_usage(&self, cache: &HashMap<String, CachedItem<T>>) -> usize {
        // Rough estimate - in practice you'd implement this based on your item types
        cache.len() * 1024 // Assume 1KB per item
    }
    
    /// Emit an event to all listeners
    fn emit_event(&self, event: LazyLoadEvent) {
        for callback in &self.state_callbacks {
            callback(event.clone());
        }
    }
    
    /// Background loader task
    async fn background_loader_task(
        mut receiver: mpsc::UnboundedReceiver<LoadRequest>,
        cache: Arc<RwLock<HashMap<String, CachedItem<T>>>>,
        provider: Option<Arc<dyn ItemProvider<T>>>,
        config: LazyLoadConfig,
        callbacks: Vec<Arc<dyn Fn(LazyLoadEvent) + Send + Sync>>,
    ) {
        let mut active_loads = 0;
        let mut pending_requests = VecDeque::new();
        
        while let Some(request) = receiver.recv().await {
            pending_requests.push_back(request);
            
            // Process requests while we have capacity
            while active_loads < config.max_concurrent_loads && !pending_requests.is_empty() {
                if let Some(req) = pending_requests.pop_front() {
                    if let Some(provider) = &provider {
                        let provider_clone = Arc::clone(provider);
                        let cache_clone = Arc::clone(&cache);
                        let callbacks_clone = callbacks.clone();
                        
                        active_loads += 1;
                        
                        tokio::spawn(async move {
                            let start_time = Instant::now();
                            
                            match provider_clone.load_item(&req.item_id).await {
                                Ok(item) => {
                                    let duration = start_time.elapsed();
                                    
                                    // Cache the item
                                    let cached_item = CachedItem {
                                        item,
                                        loaded_at: Instant::now(),
                                        access_count: 0,
                                        last_accessed: Instant::now(),
                                        load_duration: duration,
                                    };
                                    
                                    {
                                        let mut cache = cache_clone.write().await;
                                        cache.insert(req.item_id.clone(), cached_item);
                                    }
                                    
                                    // Emit success event
                                    for callback in &callbacks_clone {
                                        callback(LazyLoadEvent::LoadCompleted {
                                            item_id: req.item_id.clone(),
                                            duration,
                                        });
                                    }
                                }
                                Err(e) => {
                                    // Emit failure event
                                    for callback in &callbacks_clone {
                                        callback(LazyLoadEvent::LoadFailed {
                                            item_id: req.item_id.clone(),
                                            error: e.to_string(),
                                            retry_count: req.retry_count,
                                        });
                                    }
                                }
                            }
                        });
                    }
                }
            }
        }
    }
    
    /// Get current metrics
    pub fn metrics(&self) -> &LazyLoadMetrics {
        &self.metrics
    }
    
    /// Clear the cache
    pub async fn clear_cache(&mut self) {
        let mut cache = self.item_cache.write().await;
        cache.clear();
        self.metrics.cache_size = 0;
    }
    
    /// Get cache statistics
    pub async fn cache_stats(&self) -> HashMap<String, serde_json::Value> {
        let cache = self.item_cache.read().await;
        let mut stats = HashMap::new();
        
        stats.insert("size".to_string(), serde_json::Value::from(cache.len()));
        stats.insert("memory_usage".to_string(), 
            serde_json::Value::from(self.estimate_memory_usage(&cache)));
        
        if !cache.is_empty() {
            let avg_access_count: f64 = cache.values()
                .map(|item| item.access_count as f64)
                .sum::<f64>() / cache.len() as f64;
            stats.insert("avg_access_count".to_string(), 
                serde_json::Value::from(avg_access_count));
            
            let avg_age = cache.values()
                .map(|item| item.loaded_at.elapsed().as_secs())
                .sum::<u64>() / cache.len() as u64;
            stats.insert("avg_age_seconds".to_string(), 
                serde_json::Value::from(avg_age));
        }
        
        stats
    }
}

impl<T: ListItem + 'static> Default for LazyLoader<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::lists::SimpleListItem;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct TestProvider {
        load_count: AtomicUsize,
    }
    
    impl TestProvider {
        fn new() -> Self {
            Self {
                load_count: AtomicUsize::new(0),
            }
        }
        
        fn load_count(&self) -> usize {
            self.load_count.load(Ordering::Relaxed)
        }
    }
    
    impl ItemProvider<SimpleListItem> for TestProvider {
        fn load_item(&self, item_id: &str) -> Pin<Box<dyn Future<Output = Result<SimpleListItem>> + Send>> {
            let id = item_id.to_string();
            let count = self.load_count.fetch_add(1, Ordering::Relaxed);
            
            Box::pin(async move {
                // Simulate loading time
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(SimpleListItem::from_text(id.clone(), format!("Item {} (load #{})", id, count)))
            })
        }
    }
    
    struct TestPlaceholderGenerator;
    
    impl PlaceholderGenerator<SimpleListItem> for TestPlaceholderGenerator {
        fn generate_placeholder(&self, item_id: &str) -> SimpleListItem {
            SimpleListItem::from_text(item_id.to_string(), format!("Loading item {}...", item_id))
        }
    }
    
    #[tokio::test]
    async fn test_lazy_loader_creation() {
        let loader: LazyLoader<SimpleListItem> = LazyLoader::new();
        assert_eq!(loader.metrics().cache_size, 0);
        assert_eq!(loader.metrics().total_requests, 0);
    }
    
    #[tokio::test]
    async fn test_item_loading() {
        let mut loader = LazyLoader::new();
        let provider = TestProvider::new();
        loader.set_item_provider(provider);
        loader.set_placeholder_generator(TestPlaceholderGenerator);
        
        // Request an item
        let item = loader.get_item("test1").await.unwrap();
        
        // Should get placeholder initially
        let line_text: String = item.content()[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(line_text.contains("Loading"));
    }
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let mut loader = LazyLoader::new();
        let provider = TestProvider::new();
        loader.set_item_provider(provider);
        loader.start_background_task().unwrap();
        
        // Load an item
        loader.request_load("test1".to_string(), LoadPriority::High).await.unwrap();
        
        // Wait a bit for background loading
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Check if item is cached
        let cached = loader.get_cached_item("test1").await;
        assert!(cached.is_some());
        
        loader.stop_background_task().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_preloading() {
        let mut loader = LazyLoader::new();
        let provider = TestProvider::new();
        loader.set_item_provider(provider);
        loader.start_background_task().unwrap();
        
        // Preload items around a center
        let visible_items = vec!["item1".to_string(), "item2".to_string(), "item3".to_string()];
        loader.preload_around("item2", &visible_items).await.unwrap();
        
        // Wait for background loading
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check metrics
        let metrics = loader.metrics();
        assert!(metrics.total_requests > 0);
        
        loader.stop_background_task().await.unwrap();
    }
}