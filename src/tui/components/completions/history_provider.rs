//! History-based completion provider that learns from user patterns

use super::{CompletionItem, CompletionContext, CompletionProvider, ProviderConfig};
use crate::llm::{Message, MessageRole};
use crate::session::Database;
use anyhow::{Result, Context as AnyhowContext};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// History-based completion provider
#[derive(Debug, Clone)]
pub struct HistoryProvider {
    config: ProviderConfig,
    max_history_items: usize,
    min_frequency: usize,
    boost_recent: bool,
    database_path: Option<String>,
}

impl HistoryProvider {
    /// Create a new history provider
    pub fn new() -> Self {
        Self {
            config: ProviderConfig::default(),
            max_history_items: 100,
            min_frequency: 2,
            boost_recent: true,
            database_path: None,
        }
    }

    /// Set maximum number of history items to consider
    pub fn with_max_history(mut self, max_items: usize) -> Self {
        self.max_history_items = max_items;
        self
    }

    /// Set minimum frequency for suggestions
    pub fn with_min_frequency(mut self, min_freq: usize) -> Self {
        self.min_frequency = min_freq;
        self
    }

    /// Enable/disable boost for recent items
    pub fn with_recent_boost(mut self, boost: bool) -> Self {
        self.boost_recent = boost;
        self
    }

    /// Set database path for session history
    pub fn with_database_path(mut self, path: String) -> Self {
        self.database_path = Some(path);
        self
    }

    /// Extract commands and phrases from message history
    async fn extract_patterns_from_history(&self, messages: &[Message]) -> HashMap<String, PatternInfo> {
        let mut patterns = HashMap::new();
        
        for message in messages {
            // Only consider user messages
            if message.role != MessageRole::User {
                continue;
            }

            // Extract words and phrases from the message
            let text = message.get_text_content().unwrap_or_default();
            self.extract_patterns_from_text(&text, &mut patterns, message.timestamp.timestamp());
        }

        patterns
    }

    /// Extract completion patterns from text
    fn extract_patterns_from_text(&self, text: &str, patterns: &mut HashMap<String, PatternInfo>, timestamp: i64) {
        // Extract individual words
        let words: Vec<&str> = text.split_whitespace().collect();
        
        for word in &words {
            // Skip very short words and common words
            if word.len() >= 3 && !self.is_common_word(word) {
                let pattern = word.to_lowercase();
                let entry = patterns.entry(pattern).or_insert_with(|| PatternInfo::new(word));
                entry.increment(timestamp);
            }
        }

        // Extract phrases (2-3 words)
        for window in words.windows(2) {
            let phrase = window.join(" ");
            if phrase.len() >= 6 && phrase.len() <= 50 {
                let pattern = phrase.to_lowercase();
                let entry = patterns.entry(pattern).or_insert_with(|| PatternInfo::new(&phrase));
                entry.increment(timestamp);
            }
        }

        // Extract file paths
        for word in &words {
            if self.looks_like_path(word) && word.len() >= 3 {
                let pattern = word.to_lowercase();
                let entry = patterns.entry(pattern).or_insert_with(|| PatternInfo::new(word));
                entry.increment(timestamp);
                entry.mark_as_path();
            }
        }

        // Extract command patterns
        if let Some(first_word) = words.first() {
            if self.looks_like_command(first_word) {
                let command_pattern = format!("cmd:{}", first_word.to_lowercase());
                let entry = patterns.entry(command_pattern).or_insert_with(|| PatternInfo::new(first_word));
                entry.increment(timestamp);
                entry.mark_as_command();
            }
        }
    }

    /// Check if a word is too common to be useful for completion
    fn is_common_word(&self, word: &str) -> bool {
        const COMMON_WORDS: &[&str] = &[
            "the", "and", "or", "but", "for", "with", "this", "that", "these", "those",
            "can", "will", "would", "should", "could", "may", "might", "must",
            "have", "has", "had", "are", "was", "were", "been", "being",
            "do", "does", "did", "get", "got", "give", "take", "make", "use",
            "see", "know", "think", "say", "tell", "ask", "try", "come", "go",
            "want", "need", "like", "help", "show", "find", "look", "work",
            "file", "files", "directory", "folder", "path", "name", "type",
            "how", "what", "when", "where", "why", "who", "which",
        ];
        
        COMMON_WORDS.contains(&word.to_lowercase().as_str())
    }

    /// Check if a string looks like a file path
    fn looks_like_path(&self, text: &str) -> bool {
        text.contains('/') || 
        text.contains('\\') || 
        text.starts_with('.') ||
        text.starts_with('~') ||
        text.contains('.') && !text.contains(' ')
    }

    /// Check if a string looks like a command
    fn looks_like_command(&self, text: &str) -> bool {
        // Simple heuristics for command detection
        text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') &&
        !text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_ascii_uppercase()) && // Not all caps (likely constant)
        text.len() >= 2 && text.len() <= 20
    }

    /// Get recent message history from database
    async fn get_recent_history(&self) -> Result<Vec<Message>> {
        let db_path = match &self.database_path {
            Some(path) => path.clone(),
            None => {
                // Try to find the default session database
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.goofy/sessions.db", home)
            }
        };

        let db_manager = Database::new(&db_path).await?;
        
        // Get recent messages from all sessions
        let sessions = db_manager.list_sessions(Some(5)).await?; // Only get last 5 sessions
        let mut all_messages = Vec::new();
        
        for session in sessions.iter() {
            let messages = db_manager.get_messages(&session.id, Some(20)).await?; // Limit messages per session
            all_messages.extend(messages);
        }

        // Sort by timestamp and take most recent
        all_messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_messages.truncate(self.max_history_items);

        Ok(all_messages)
    }

    /// Calculate relevance score for a pattern
    fn calculate_pattern_score(&self, pattern: &PatternInfo, query: &str, current_time: i64) -> f64 {
        if pattern.frequency < self.min_frequency {
            return 0.0;
        }

        let mut score = 0.0;

        // Base score from frequency
        score += (pattern.frequency as f64).ln() * 0.3;

        // Boost for recent usage
        if self.boost_recent && pattern.last_used > 0 {
            let time_diff = current_time - pattern.last_used;
            let hours_ago = time_diff as f64 / 3600.0;
            
            if hours_ago < 1.0 {
                score += 0.5; // Recent use within last hour
            } else if hours_ago < 24.0 {
                score += 0.3; // Recent use within last day
            } else if hours_ago < 168.0 {
                score += 0.1; // Recent use within last week
            }
        }

        // Boost for exact prefix match
        if pattern.text.to_lowercase().starts_with(&query.to_lowercase()) {
            score += 0.4;
        }

        // Boost for word boundary matches
        if pattern.text.to_lowercase().contains(&format!(" {}", query.to_lowercase())) {
            score += 0.2;
        }

        // Boost based on pattern type
        if pattern.is_command {
            score += 0.2;
        }
        if pattern.is_path {
            score += 0.1;
        }

        // Penalty for very long patterns
        if pattern.text.len() > 50 {
            score -= 0.1;
        }

        score.max(0.0)
    }

    /// Filter patterns by context
    fn filter_by_context(&self, patterns: &HashMap<String, PatternInfo>, context: &CompletionContext) -> HashMap<String, PatternInfo> {
        let mut filtered = HashMap::new();

        for (key, pattern) in patterns {
            let should_include = if context.is_command() {
                // In command context, prefer commands and short patterns
                pattern.is_command || pattern.text.split_whitespace().count() <= 2
            } else if context.is_file_path() {
                // In file path context, prefer paths
                pattern.is_path || self.looks_like_path(&pattern.text)
            } else {
                // General context, include all relevant patterns
                !pattern.is_command || pattern.text.len() >= 3
            };

            if should_include {
                filtered.insert(key.clone(), pattern.clone());
            }
        }

        filtered
    }
}

impl Default for HistoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a completion pattern from history
#[derive(Debug, Clone)]
struct PatternInfo {
    text: String,
    frequency: usize,
    last_used: i64,
    first_used: i64,
    is_command: bool,
    is_path: bool,
}

impl PatternInfo {
    fn new(text: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            text: text.to_string(),
            frequency: 0,
            last_used: now,
            first_used: now,
            is_command: false,
            is_path: false,
        }
    }

    fn increment(&mut self, timestamp: i64) {
        self.frequency += 1;
        self.last_used = self.last_used.max(timestamp);
        if self.first_used == 0 || timestamp < self.first_used {
            self.first_used = timestamp;
        }
    }

    fn mark_as_command(&mut self) {
        self.is_command = true;
    }

    fn mark_as_path(&mut self) {
        self.is_path = true;
    }
}

#[async_trait]
impl CompletionProvider for HistoryProvider {
    fn name(&self) -> &str {
        "history"
    }

    async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let query = context.current_word();
        
        if query.len() < 2 {
            return Ok(Vec::new());
        }

        debug!("History completion for query: '{}'", query);

        // Get recent message history
        let messages = match self.get_recent_history().await {
            Ok(messages) => messages,
            Err(e) => {
                warn!("Failed to load history: {}", e);
                return Ok(Vec::new());
            }
        };

        // Extract patterns from history
        let all_patterns = self.extract_patterns_from_history(&messages).await;
        
        // Filter patterns by context
        let filtered_patterns = self.filter_by_context(&all_patterns, context);

        // Score and rank patterns
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut scored_patterns: Vec<_> = filtered_patterns
            .values()
            .filter_map(|pattern| {
                let score = self.calculate_pattern_score(pattern, query, current_time);
                if score > 0.0 {
                    Some((pattern, score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score
        scored_patterns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Convert to completion items
        let mut items = Vec::new();
        for (pattern, score) in scored_patterns.into_iter().take(context.max_results) {
            let description = if pattern.is_command {
                format!("Command (used {} times)", pattern.frequency)
            } else if pattern.is_path {
                format!("Path (used {} times)", pattern.frequency)
            } else {
                format!("From history (used {} times)", pattern.frequency)
            };

            let item = CompletionItem::new(&pattern.text, &pattern.text, "history")
                .with_description(description)
                .with_score(score);

            items.push(item);
        }

        debug!("Found {} history completions", items.len());
        Ok(items)
    }

    fn is_applicable(&self, _context: &CompletionContext) -> bool {
        true // History can apply to any context
    }

    fn get_priority(&self, _context: &CompletionContext) -> i32 {
        3 // Medium-low priority - supplement other providers
    }

    fn cache_ttl(&self) -> Option<u64> {
        Some(300) // Cache for 5 minutes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{Message, MessageRole, ContentBlock};

    #[test]
    fn test_pattern_extraction() {
        let provider = HistoryProvider::new();
        let mut patterns = HashMap::new();
        
        provider.extract_patterns_from_text(
            "cargo build --release src/main.rs",
            &mut patterns,
            1234567890
        );

        assert!(patterns.contains_key("cargo"));
        assert!(patterns.contains_key("build"));
        assert!(patterns.contains_key("src/main.rs"));
        assert!(patterns.contains_key("cmd:cargo"));
        
        let cargo_pattern = &patterns["cargo"];
        assert_eq!(cargo_pattern.frequency, 1);
    }

    #[test]
    fn test_common_word_filtering() {
        let provider = HistoryProvider::new();
        
        assert!(provider.is_common_word("the"));
        assert!(provider.is_common_word("and"));
        assert!(!provider.is_common_word("cargo"));
        assert!(!provider.is_common_word("build"));
    }

    #[test]
    fn test_path_detection() {
        let provider = HistoryProvider::new();
        
        assert!(provider.looks_like_path("src/main.rs"));
        assert!(provider.looks_like_path("./config.toml"));
        assert!(provider.looks_like_path("~/documents"));
        assert!(provider.looks_like_path("file.txt"));
        assert!(!provider.looks_like_path("hello world"));
        assert!(!provider.looks_like_path("cargo"));
    }

    #[test]
    fn test_command_detection() {
        let provider = HistoryProvider::new();
        
        assert!(provider.looks_like_command("cargo"));
        assert!(provider.looks_like_command("git"));
        assert!(provider.looks_like_command("ls"));
        assert!(provider.looks_like_command("docker-compose"));
        assert!(!provider.looks_like_command("THIS_IS_CONSTANT"));
        assert!(!provider.looks_like_command("very-long-command-name-that-is-too-long"));
        assert!(!provider.looks_like_command("a"));
    }

    #[test]
    fn test_pattern_scoring() {
        let provider = HistoryProvider::new();
        let current_time = 1234567890;
        
        let mut pattern = PatternInfo::new("test_pattern");
        pattern.frequency = 5;
        pattern.last_used = current_time - 1800; // 30 minutes ago
        
        let score = provider.calculate_pattern_score(&pattern, "test", current_time);
        assert!(score > 0.0);
        
        // Test frequency threshold
        pattern.frequency = 1; // Below min_frequency
        let score = provider.calculate_pattern_score(&pattern, "test", current_time);
        assert_eq!(score, 0.0);
    }

    #[tokio::test]
    async fn test_pattern_extraction_from_messages() {
        let provider = HistoryProvider::new();
        
        let messages = vec![
            Message {
                id: "1".to_string(),
                role: MessageRole::User,
                content: vec![ContentBlock::Text { text: "cargo build --release".to_string() }],
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            },
            Message {
                id: "2".to_string(),
                role: MessageRole::User,
                content: vec![ContentBlock::Text { text: "git commit -m 'update'".to_string() }],
                timestamp: chrono::Utc::now(),
                metadata: std::collections::HashMap::new(),
            },
        ];

        let patterns = provider.extract_patterns_from_history(&messages).await;
        
        assert!(patterns.contains_key("cargo"));
        assert!(patterns.contains_key("build"));
        assert!(patterns.contains_key("commit"));
        assert!(patterns.contains_key("cmd:cargo"));
        assert!(patterns.contains_key("cmd:git"));
    }
}