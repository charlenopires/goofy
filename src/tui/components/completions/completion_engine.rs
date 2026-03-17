//! Core completion engine that orchestrates providers and caching

use super::{
    CompletionItem, CompletionContext, CompletionProvider, CompletionCache,
    fuzzy_match, fuzzy_score, MAX_COMPLETIONS,
};
use anyhow::{Result, Context as AnyhowContext};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// Priority levels for completion providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Registered completion provider with priority
#[derive(Debug, Clone)]
struct RegisteredProvider {
    provider: Arc<dyn CompletionProvider>,
    priority: ProviderPriority,
    enabled: bool,
}

/// Core completion engine that manages providers and orchestrates completion generation
pub struct CompletionEngine {
    providers: Vec<RegisteredProvider>,
    cache: Arc<RwLock<CompletionCache>>,
    min_query_length: usize,
    fuzzy_threshold: f64,
}

impl CompletionEngine {
    /// Create a new completion engine
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: Arc::new(RwLock::new(CompletionCache::new())),
            min_query_length: 1,
            fuzzy_threshold: 0.3,
        }
    }

    /// Register a completion provider with priority
    pub fn register_provider(&mut self, provider: Arc<dyn CompletionProvider>, priority: ProviderPriority) {
        debug!("Registering completion provider: {} with priority: {:?}", 
               provider.name(), priority);
        
        self.providers.push(RegisteredProvider {
            provider,
            priority,
            enabled: true,
        });
        
        // Sort providers by priority (highest first)
        self.providers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Enable or disable a provider
    pub fn set_provider_enabled(&mut self, name: &str, enabled: bool) {
        for provider in &mut self.providers {
            if provider.provider.name() == name {
                provider.enabled = enabled;
                debug!("Set provider '{}' enabled: {}", name, enabled);
                break;
            }
        }
    }

    /// Get completion suggestions for the given context
    pub async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let query = context.current_word();
        
        // Skip if query is too short
        if query.len() < self.min_query_length {
            return Ok(Vec::new());
        }

        debug!("Getting completions for query: '{}' at position: {}", 
               query, context.cursor_pos);

        // Check cache first
        let cache_key = self.generate_cache_key(context);
        {
            let mut cache = self.cache.write().await;
            if let Some(cached_items) = cache.get(&cache_key) {
                debug!("Found {} cached completions", cached_items.len());
                return Ok(self.filter_and_rank_items(cached_items.clone(), query));
            }
        }

        // Collect completions from all enabled providers
        let mut all_items = Vec::new();
        
        for registered in &self.providers {
            if !registered.enabled {
                continue;
            }

            match registered.provider.get_completions(context).await {
                Ok(items) => {
                    debug!("Provider '{}' returned {} completions", 
                           registered.provider.name(), items.len());
                    all_items.extend(items);
                }
                Err(e) => {
                    warn!("Provider '{}' failed: {}", registered.provider.name(), e);
                }
            }
        }

        // Remove duplicates (keep highest scored)
        all_items = self.deduplicate_items(all_items);

        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, all_items.clone());
        }

        // Filter and rank the final results
        Ok(self.filter_and_rank_items(all_items, query))
    }

    /// Filter completions based on query and refresh cache if needed
    pub async fn filter_completions(&self, context: &CompletionContext, query: &str) -> Result<Vec<CompletionItem>> {
        debug!("Filtering completions with query: '{}'", query);

        // Get cached completions for the base context
        let cache_key = self.generate_cache_key(context);
        let base_items = {
            let mut cache = self.cache.write().await;
            cache.get(&cache_key).unwrap_or_default()
        };

        // If we have cached items, filter them
        if !base_items.is_empty() {
            return Ok(self.filter_and_rank_items(base_items, query));
        }

        // Otherwise, get fresh completions
        self.get_completions(context).await
    }

    /// Clear the completion cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        debug!("Completion cache cleared");
    }

    /// Set minimum query length for triggering completions
    pub fn set_min_query_length(&mut self, length: usize) {
        self.min_query_length = length;
    }

    /// Set fuzzy matching threshold
    pub fn set_fuzzy_threshold(&mut self, threshold: f64) {
        self.fuzzy_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Generate cache key for completion context
    fn generate_cache_key(&self, context: &CompletionContext) -> String {
        format!("{}:{}:{}:{}",
                context.text,
                context.cursor_pos,
                context.working_dir.as_deref().unwrap_or(""),
                context.command_context.as_deref().unwrap_or(""))
    }

    /// Filter and rank items based on query using fuzzy matching
    fn filter_and_rank_items(&self, items: Vec<CompletionItem>, query: &str) -> Vec<CompletionItem> {
        if query.is_empty() {
            return items.into_iter()
                .take(MAX_COMPLETIONS)
                .collect();
        }

        let mut scored_items: Vec<(CompletionItem, f64)> = items
            .into_iter()
            .filter_map(|item| {
                // Try exact match first
                if item.title.starts_with(query) || item.value.starts_with(query) {
                    return Some((item, 1.0));
                }

                // Then fuzzy match
                let title_score = fuzzy_score(&item.title, query);
                let value_score = fuzzy_score(&item.value, query);
                let max_score = title_score.max(value_score);

                if max_score >= self.fuzzy_threshold {
                    Some((item, max_score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (highest first), then by original score, then alphabetically
        scored_items.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.0.score.partial_cmp(&a.0.score).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.0.title.cmp(&b.0.title))
        });

        scored_items
            .into_iter()
            .take(MAX_COMPLETIONS)
            .map(|(item, _)| item)
            .collect()
    }

    /// Remove duplicate items, keeping the one with highest score
    fn deduplicate_items(&self, items: Vec<CompletionItem>) -> Vec<CompletionItem> {
        let mut unique_items = Vec::new();
        let mut seen_values = std::collections::HashSet::new();

        for item in items {
            let key = format!("{}:{}", item.title, item.value);
            if !seen_values.contains(&key) {
                seen_values.insert(key);
                unique_items.push(item);
            }
        }

        unique_items
    }

    /// Get list of registered provider names
    pub fn provider_names(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.provider.name().to_string())
            .collect()
    }

    /// Get provider statistics
    pub async fn get_stats(&self) -> CompletionStats {
        let cache = self.cache.read().await;
        CompletionStats {
            provider_count: self.providers.len(),
            enabled_providers: self.providers.iter().filter(|p| p.enabled).count(),
            cache_size: cache.len(),
            cache_hit_rate: cache.hit_rate(),
        }
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the completion engine
#[derive(Debug, Clone)]
pub struct CompletionStats {
    pub provider_count: usize,
    pub enabled_providers: usize,
    pub cache_size: usize,
    pub cache_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::components::completions::{CompletionProvider, CompletionItem};
    use async_trait::async_trait;

    #[derive(Debug)]
    struct MockProvider {
        name: String,
        items: Vec<CompletionItem>,
    }

    impl MockProvider {
        fn new(name: &str, items: Vec<CompletionItem>) -> Self {
            Self {
                name: name.to_string(),
                items,
            }
        }
    }

    #[async_trait]
    impl CompletionProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn get_completions(&self, _context: &CompletionContext) -> Result<Vec<CompletionItem>> {
            Ok(self.items.clone())
        }
    }

    #[tokio::test]
    async fn test_completion_engine_basic() {
        let mut engine = CompletionEngine::new();
        
        let items = vec![
            CompletionItem::new("test1", "test1", "mock"),
            CompletionItem::new("test2", "test2", "mock"),
        ];
        
        let provider = Arc::new(MockProvider::new("mock", items));
        engine.register_provider(provider, ProviderPriority::Medium);

        let context = CompletionContext::new("te", 2);
        let completions = engine.get_completions(&context).await.unwrap();
        
        assert_eq!(completions.len(), 2);
        assert!(completions.iter().any(|c| c.title == "test1"));
        assert!(completions.iter().any(|c| c.title == "test2"));
    }

    #[tokio::test]
    async fn test_fuzzy_filtering() {
        let mut engine = CompletionEngine::new();
        
        let items = vec![
            CompletionItem::new("hello_world", "hello_world", "mock"),
            CompletionItem::new("help_text", "help_text", "mock"),
            CompletionItem::new("application", "application", "mock"),
        ];
        
        let provider = Arc::new(MockProvider::new("mock", items));
        engine.register_provider(provider, ProviderPriority::Medium);

        let context = CompletionContext::new("hlw", 3);
        let completions = engine.get_completions(&context).await.unwrap();
        
        // Should find "hello_world" through fuzzy matching
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.title == "hello_world"));
    }

    #[tokio::test]
    async fn test_provider_priority() {
        let mut engine = CompletionEngine::new();
        
        let high_priority_items = vec![
            CompletionItem::new("high", "high", "high_provider").with_score(1.0),
        ];
        let low_priority_items = vec![
            CompletionItem::new("low", "low", "low_provider").with_score(1.0),
        ];
        
        let high_provider = Arc::new(MockProvider::new("high_provider", high_priority_items));
        let low_provider = Arc::new(MockProvider::new("low_provider", low_priority_items));
        
        engine.register_provider(low_provider, ProviderPriority::Low);
        engine.register_provider(high_provider, ProviderPriority::High);

        // High priority provider should be queried first
        assert_eq!(engine.providers[0].priority, ProviderPriority::High);
        assert_eq!(engine.providers[1].priority, ProviderPriority::Low);
    }
}