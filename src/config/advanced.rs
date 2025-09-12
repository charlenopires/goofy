//! Advanced configuration system for Goofy
//! 
//! This module provides comprehensive configuration management including
//! provider settings, UI customization, permissions, and advanced features.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, error, info};

use crate::llm::LlmProvider;

/// Advanced configuration for Goofy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Model configurations for different types
    #[serde(default)]
    pub models: HashMap<ModelType, SelectedModel>,
    
    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    
    /// TUI options and customization
    #[serde(default)]
    pub tui: TUIOptions,
    
    /// Permission settings
    #[serde(default)]
    pub permissions: Permissions,
    
    /// Context and workspace settings
    #[serde(default)]
    pub workspace: WorkspaceOptions,
    
    /// Advanced features configuration
    #[serde(default)]
    pub features: FeatureFlags,
    
    /// Keyboard shortcuts and key bindings
    #[serde(default)]
    pub keybindings: KeyBindings,
    
    /// Appearance and theme settings
    #[serde(default)]
    pub appearance: AppearanceConfig,
}

/// Model type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    /// Large model for complex tasks
    Large,
    /// Small model for quick tasks
    Small,
    /// Embedding model for context
    Embedding,
}

/// Selected model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedModel {
    /// Model ID as used by provider
    pub model: String,
    
    /// Provider ID
    pub provider: String,
    
    /// Maximum tokens for responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Temperature setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// Whether to enable thinking mode (for supported models)
    #[serde(default)]
    pub think: bool,
    
    /// Reasoning effort level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
}

/// Reasoning effort levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// API base URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    
    /// Provider type
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    
    /// API key (can use environment variable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    
    /// Whether provider is disabled
    #[serde(default)]
    pub disabled: bool,
    
    /// Custom system prompt prefix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt_prefix: Option<String>,
    
    /// Extra HTTP headers
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    
    /// Extra request body fields
    #[serde(default)]
    pub extra_body: HashMap<String, serde_json::Value>,
    
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    /// Request retry attempts
    #[serde(default = "default_retries")]
    pub retries: u32,
}

/// TUI options and customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TUIOptions {
    /// Enable compact mode
    #[serde(default)]
    pub compact_mode: bool,
    
    /// Show line numbers in code blocks
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    
    /// Enable animations
    #[serde(default = "default_true")]
    pub enable_animations: bool,
    
    /// Animation speed (1.0 = normal)
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f32,
    
    /// Enable mouse support
    #[serde(default = "default_true")]
    pub enable_mouse: bool,
    
    /// Enable auto-completion
    #[serde(default = "default_true")]
    pub enable_completion: bool,
    
    /// Completion delay in milliseconds
    #[serde(default = "default_completion_delay")]
    pub completion_delay: u64,
    
    /// Maximum completion suggestions
    #[serde(default = "default_max_completions")]
    pub max_completions: usize,
    
    /// Enable syntax highlighting
    #[serde(default = "default_true")]
    pub enable_syntax_highlighting: bool,
    
    /// Default editor for file editing
    #[serde(default)]
    pub default_editor: Option<String>,
    
    /// Terminal title format
    #[serde(default = "default_title_format")]
    pub title_format: String,
    
    /// Status line format
    #[serde(default = "default_status_format")]
    pub status_format: String,
}

/// Permission settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    /// Tools that don't require permission prompts
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    
    /// Automatically approve all tool usage
    #[serde(default)]
    pub auto_approve: bool,
    
    /// Allowed file patterns for file operations
    #[serde(default)]
    pub allowed_file_patterns: Vec<String>,
    
    /// Blocked file patterns
    #[serde(default)]
    pub blocked_file_patterns: Vec<String>,
    
    /// Allowed network hosts
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    
    /// Blocked network hosts
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
    
    /// Maximum file size for operations (MB)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: u64,
    
    /// Maximum execution time for commands (seconds)
    #[serde(default = "default_max_execution_time")]
    pub max_execution_time: u64,
}

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOptions {
    /// Paths to context files
    #[serde(default = "default_context_paths")]
    pub context_paths: Vec<String>,
    
    /// Data directory for storing application data
    #[serde(default = "default_data_directory")]
    pub data_directory: String,
    
    /// Enable debug logging
    #[serde(default)]
    pub debug: bool,
    
    /// Disable automatic conversation summarization
    #[serde(default)]
    pub disable_auto_summarize: bool,
    
    /// Maximum conversation history length
    #[serde(default = "default_max_history")]
    pub max_conversation_history: usize,
    
    /// Auto-save interval in seconds
    #[serde(default = "default_autosave_interval")]
    pub autosave_interval: u64,
    
    /// Session timeout in minutes
    #[serde(default = "default_session_timeout")]
    pub session_timeout: u64,
}

/// Feature flags for experimental features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable experimental features
    #[serde(default)]
    pub experimental: bool,
    
    /// Enable image support
    #[serde(default = "default_true")]
    pub images: bool,
    
    /// Enable markdown rendering
    #[serde(default = "default_true")]
    pub markdown: bool,
    
    /// Enable file diff viewer
    #[serde(default = "default_true")]
    pub diff_viewer: bool,
    
    /// Enable virtual scrolling for large lists
    #[serde(default = "default_true")]
    pub virtual_scrolling: bool,
    
    /// Enable fuzzy search
    #[serde(default = "default_true")]
    pub fuzzy_search: bool,
    
    /// Enable telemetry
    #[serde(default)]
    pub telemetry: bool,
    
    /// Enable crash reporting
    #[serde(default)]
    pub crash_reporting: bool,
}

/// Keyboard shortcuts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Global key bindings
    #[serde(default)]
    pub global: HashMap<String, String>,
    
    /// Chat-specific key bindings
    #[serde(default)]
    pub chat: HashMap<String, String>,
    
    /// Editor key bindings
    #[serde(default)]
    pub editor: HashMap<String, String>,
    
    /// File browser key bindings
    #[serde(default)]
    pub file_browser: HashMap<String, String>,
}

/// Appearance and theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// Current theme name
    #[serde(default = "default_theme")]
    pub theme: String,
    
    /// Custom theme overrides
    #[serde(default)]
    pub theme_overrides: HashMap<String, String>,
    
    /// Font settings
    #[serde(default)]
    pub font: FontConfig,
    
    /// UI layout preferences
    #[serde(default)]
    pub layout: LayoutConfig,
    
    /// Color scheme preferences
    #[serde(default)]
    pub colors: ColorConfig,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Font family
    #[serde(default)]
    pub family: Option<String>,
    
    /// Font size multiplier
    #[serde(default = "default_font_size")]
    pub size: f32,
    
    /// Enable ligatures
    #[serde(default)]
    pub ligatures: bool,
}

/// Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Default panel layout
    #[serde(default = "default_layout")]
    pub default: String,
    
    /// Panel sizes as percentages
    #[serde(default)]
    pub panel_sizes: HashMap<String, f32>,
    
    /// Show borders around panels
    #[serde(default = "default_true")]
    pub show_borders: bool,
    
    /// Border style
    #[serde(default = "default_border_style")]
    pub border_style: String,
}

/// Color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    /// Enable true color support
    #[serde(default = "default_true")]
    pub true_color: bool,
    
    /// Color depth preference
    #[serde(default = "default_color_depth")]
    pub color_depth: u8,
    
    /// Custom color overrides
    #[serde(default)]
    pub custom_colors: HashMap<String, String>,
}

/// Configuration manager for advanced settings
pub struct AdvancedConfigManager {
    config: AdvancedConfig,
    config_path: PathBuf,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            models: default_models(),
            providers: HashMap::new(),
            tui: TUIOptions::default(),
            permissions: Permissions::default(),
            workspace: WorkspaceOptions::default(),
            features: FeatureFlags::default(),
            keybindings: KeyBindings::default(),
            appearance: AppearanceConfig::default(),
        }
    }
}

impl Default for TUIOptions {
    fn default() -> Self {
        Self {
            compact_mode: false,
            show_line_numbers: true,
            enable_animations: true,
            animation_speed: 1.0,
            enable_mouse: true,
            enable_completion: true,
            completion_delay: 200,
            max_completions: 10,
            enable_syntax_highlighting: true,
            default_editor: None,
            title_format: "Goofy - {session}".to_string(),
            status_format: "{provider} | {model} | {tokens}".to_string(),
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            allowed_tools: vec!["view".to_string(), "ls".to_string()],
            auto_approve: false,
            allowed_file_patterns: vec!["**/*.md".to_string(), "**/*.txt".to_string()],
            blocked_file_patterns: vec!["**/.env".to_string(), "**/secret*".to_string()],
            allowed_hosts: vec!["github.com".to_string(), "api.github.com".to_string()],
            blocked_hosts: vec![],
            max_file_size_mb: 10,
            max_execution_time: 30,
        }
    }
}

impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self {
            context_paths: default_context_paths(),
            data_directory: ".goofy".to_string(),
            debug: false,
            disable_auto_summarize: false,
            max_conversation_history: 100,
            autosave_interval: 60,
            session_timeout: 1440, // 24 hours
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            experimental: false,
            images: true,
            markdown: true,
            diff_viewer: true,
            virtual_scrolling: true,
            fuzzy_search: true,
            telemetry: false,
            crash_reporting: false,
        }
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            global: default_global_keybindings(),
            chat: default_chat_keybindings(),
            editor: default_editor_keybindings(),
            file_browser: default_file_browser_keybindings(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "goofy_dark".to_string(),
            theme_overrides: HashMap::new(),
            font: FontConfig::default(),
            layout: LayoutConfig::default(),
            colors: ColorConfig::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: None,
            size: 1.0,
            ligatures: false,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default: "vertical".to_string(),
            panel_sizes: HashMap::new(),
            show_borders: true,
            border_style: "rounded".to_string(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            true_color: true,
            color_depth: 24,
            custom_colors: HashMap::new(),
        }
    }
}

impl AdvancedConfigManager {
    /// Create a new configuration manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config: AdvancedConfig::default(),
            config_path,
        }
    }
    
    /// Load configuration from file
    pub async fn load(&mut self) -> Result<()> {
        if !self.config_path.exists() {
            info!("Configuration file not found, creating default config");
            self.save().await?;
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.config_path)
            .await
            .context("Failed to read configuration file")?;
        
        self.config = serde_json::from_str(&content)
            .context("Failed to parse configuration file")?;
        
        debug!("Loaded configuration from {:?}", self.config_path);
        Ok(())
    }
    
    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }
        
        let content = serde_json::to_string_pretty(&self.config)
            .context("Failed to serialize configuration")?;
        
        fs::write(&self.config_path, content)
            .await
            .context("Failed to write configuration file")?;
        
        debug!("Saved configuration to {:?}", self.config_path);
        Ok(())
    }
    
    /// Get current configuration
    pub fn config(&self) -> &AdvancedConfig {
        &self.config
    }
    
    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut AdvancedConfig {
        &mut self.config
    }
    
    /// Update a specific configuration field
    pub async fn update_field<T: Serialize>(&mut self, path: &str, value: T) -> Result<()> {
        // For now, we'll reload, update, and save
        // In a more sophisticated implementation, we could use JSON patching
        match path {
            "tui.compact_mode" => {
                if let Ok(val) = serde_json::from_value(serde_json::to_value(value)?) {
                    self.config.tui.compact_mode = val;
                }
            }
            "appearance.theme" => {
                if let Ok(val) = serde_json::from_value(serde_json::to_value(value)?) {
                    self.config.appearance.theme = val;
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported configuration path: {}", path));
            }
        }
        
        self.save().await?;
        Ok(())
    }
    
    /// Add or update a provider
    pub async fn add_provider(&mut self, id: String, config: ProviderConfig) -> Result<()> {
        self.config.providers.insert(id, config);
        self.save().await?;
        Ok(())
    }
    
    /// Remove a provider
    pub async fn remove_provider(&mut self, id: &str) -> Result<()> {
        self.config.providers.remove(id);
        self.save().await?;
        Ok(())
    }
    
    /// Update model configuration
    pub async fn update_model(&mut self, model_type: ModelType, model: SelectedModel) -> Result<()> {
        self.config.models.insert(model_type, model);
        self.save().await?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Check that all selected models have corresponding providers
        for (model_type, selected_model) in &self.config.models {
            if !self.config.providers.contains_key(&selected_model.provider) {
                return Err(anyhow::anyhow!(
                    "Model {:?} references unknown provider: {}",
                    model_type,
                    selected_model.provider
                ));
            }
        }
        
        // Validate provider configurations
        for (id, provider) in &self.config.providers {
            if provider.id != *id {
                return Err(anyhow::anyhow!(
                    "Provider ID mismatch: key '{}' vs config.id '{}'",
                    id,
                    provider.id
                ));
            }
        }
        
        Ok(())
    }
}

// Default value functions
fn default_provider_type() -> String {
    "openai".to_string()
}

fn default_timeout() -> u64 {
    30
}

fn default_retries() -> u32 {
    3
}

fn default_true() -> bool {
    true
}

fn default_animation_speed() -> f32 {
    1.0
}

fn default_completion_delay() -> u64 {
    200
}

fn default_max_completions() -> usize {
    10
}

fn default_title_format() -> String {
    "Goofy - {session}".to_string()
}

fn default_status_format() -> String {
    "{provider} | {model} | {tokens}".to_string()
}

fn default_max_file_size() -> u64 {
    10
}

fn default_max_execution_time() -> u64 {
    30
}

fn default_context_paths() -> Vec<String> {
    vec![
        ".cursorrules".to_string(),
        "CLAUDE.md".to_string(),
        "GOOFY.md".to_string(),
        "goofy.md".to_string(),
        ".goofy/context.md".to_string(),
    ]
}

fn default_data_directory() -> String {
    ".goofy".to_string()
}

fn default_max_history() -> usize {
    100
}

fn default_autosave_interval() -> u64 {
    60
}

fn default_session_timeout() -> u64 {
    1440
}

fn default_theme() -> String {
    "goofy_dark".to_string()
}

fn default_font_size() -> f32 {
    1.0
}

fn default_layout() -> String {
    "vertical".to_string()
}

fn default_border_style() -> String {
    "rounded".to_string()
}

fn default_color_depth() -> u8 {
    24
}

fn default_models() -> HashMap<ModelType, SelectedModel> {
    let mut models = HashMap::new();
    
    models.insert(
        ModelType::Large,
        SelectedModel {
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            think: false,
            reasoning_effort: None,
        },
    );
    
    models.insert(
        ModelType::Small,
        SelectedModel {
            model: "gpt-3.5-turbo".to_string(),
            provider: "openai".to_string(),
            max_tokens: Some(2048),
            temperature: Some(0.5),
            think: false,
            reasoning_effort: None,
        },
    );
    
    models
}

fn default_global_keybindings() -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    bindings.insert("quit".to_string(), "Ctrl+q".to_string());
    bindings.insert("help".to_string(), "F1".to_string());
    bindings.insert("settings".to_string(), "Ctrl+,".to_string());
    bindings.insert("new_session".to_string(), "Ctrl+n".to_string());
    bindings.insert("save_session".to_string(), "Ctrl+s".to_string());
    bindings
}

fn default_chat_keybindings() -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    bindings.insert("send_message".to_string(), "Enter".to_string());
    bindings.insert("new_line".to_string(), "Shift+Enter".to_string());
    bindings.insert("clear_input".to_string(), "Ctrl+l".to_string());
    bindings.insert("scroll_up".to_string(), "PageUp".to_string());
    bindings.insert("scroll_down".to_string(), "PageDown".to_string());
    bindings
}

fn default_editor_keybindings() -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    bindings.insert("save".to_string(), "Ctrl+s".to_string());
    bindings.insert("undo".to_string(), "Ctrl+z".to_string());
    bindings.insert("redo".to_string(), "Ctrl+y".to_string());
    bindings.insert("copy".to_string(), "Ctrl+c".to_string());
    bindings.insert("paste".to_string(), "Ctrl+v".to_string());
    bindings.insert("cut".to_string(), "Ctrl+x".to_string());
    bindings
}

fn default_file_browser_keybindings() -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    bindings.insert("open".to_string(), "Enter".to_string());
    bindings.insert("back".to_string(), "Backspace".to_string());
    bindings.insert("refresh".to_string(), "F5".to_string());
    bindings.insert("new_file".to_string(), "Ctrl+n".to_string());
    bindings.insert("delete".to_string(), "Delete".to_string());
    bindings
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let mut manager = AdvancedConfigManager::new(config_path.clone());
        manager.config.tui.compact_mode = true;
        manager.config.appearance.theme = "custom_theme".to_string();
        
        // Save config
        manager.save().await.unwrap();
        assert!(config_path.exists());
        
        // Load config
        let mut new_manager = AdvancedConfigManager::new(config_path);
        new_manager.load().await.unwrap();
        
        assert_eq!(new_manager.config.tui.compact_mode, true);
        assert_eq!(new_manager.config.appearance.theme, "custom_theme");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = AdvancedConfig::default();
        let manager = AdvancedConfigManager::new(PathBuf::from("test"));
        
        // Should validate successfully with default config
        assert!(manager.validate().is_ok());
        
        // Add a model with unknown provider
        config.models.insert(
            ModelType::Large,
            SelectedModel {
                model: "test-model".to_string(),
                provider: "unknown-provider".to_string(),
                max_tokens: None,
                temperature: None,
                think: false,
                reasoning_effort: None,
            },
        );
        
        let manager = AdvancedConfigManager { config, config_path: PathBuf::from("test") };
        assert!(manager.validate().is_err());
    }
    
    #[test]
    fn test_default_values() {
        let config = AdvancedConfig::default();
        
        assert!(!config.tui.compact_mode);
        assert!(config.tui.enable_animations);
        assert_eq!(config.appearance.theme, "goofy_dark");
        assert_eq!(config.workspace.data_directory, ".goofy");
        assert!(!config.permissions.auto_approve);
    }
}