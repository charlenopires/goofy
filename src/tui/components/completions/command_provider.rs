//! Command completion provider with context awareness

use super::{CompletionItem, CompletionContext, CompletionProvider, ProviderConfig};
use crate::llm::tools::ToolManager;
use anyhow::{Result, Context as AnyhowContext};
use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tracing::debug;

/// Command completion provider
#[derive(Debug, Clone)]
pub struct CommandProvider {
    config: ProviderConfig,
    system_commands: Vec<String>,
    tool_commands: Vec<String>,
    shell_builtins: Vec<String>,
    git_commands: Vec<String>,
    cargo_commands: Vec<String>,
    npm_commands: Vec<String>,
    context_aware: bool,
}

impl CommandProvider {
    /// Create a new command provider
    pub fn new() -> Self {
        let mut provider = Self {
            config: ProviderConfig::default(),
            system_commands: Vec::new(),
            tool_commands: Vec::new(),
            shell_builtins: Self::default_shell_builtins(),
            git_commands: Self::default_git_commands(),
            cargo_commands: Self::default_cargo_commands(),
            npm_commands: Self::default_npm_commands(),
            context_aware: true,
        };
        
        provider.load_system_commands();
        provider
    }

    /// Enable or disable context-aware completions
    pub fn with_context_awareness(mut self, enabled: bool) -> Self {
        self.context_aware = enabled;
        self
    }

    /// Set available tool commands from the tool manager
    pub fn with_tool_commands(mut self, commands: Vec<String>) -> Self {
        self.tool_commands = commands;
        self
    }

    /// Load system commands from PATH
    fn load_system_commands(&mut self) {
        if let Ok(path) = env::var("PATH") {
            let mut commands = std::collections::HashSet::new();
            
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Ok(metadata) = entry.metadata() {
                            if metadata.is_file() {
                                if let Some(name) = entry.file_name().to_str() {
                                    // Skip files with extensions on Unix (likely scripts)
                                    if !name.contains('.') && !name.starts_with('.') {
                                        commands.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            self.system_commands = commands.into_iter().collect();
            self.system_commands.sort();
        }
    }

    /// Get default shell builtin commands
    fn default_shell_builtins() -> Vec<String> {
        vec![
            "cd", "pwd", "ls", "echo", "export", "set", "unset",
            "alias", "unalias", "history", "jobs", "bg", "fg",
            "kill", "wait", "trap", "exit", "source", ".", "test",
            "true", "false", "read", "printf", "shift", "return",
            "break", "continue", "exec", "eval", "ulimit", "umask",
            "type", "which", "command", "builtin", "help"
        ].into_iter().map(String::from).collect()
    }

    /// Get default git subcommands
    fn default_git_commands() -> Vec<String> {
        vec![
            "add", "branch", "checkout", "clone", "commit", "diff",
            "fetch", "init", "log", "merge", "pull", "push", "rebase",
            "reset", "status", "tag", "rm", "mv", "show", "config",
            "remote", "stash", "cherry-pick", "revert", "bisect",
            "blame", "grep", "clean", "gc", "reflog", "archive"
        ].into_iter().map(String::from).collect()
    }

    /// Get default cargo subcommands
    fn default_cargo_commands() -> Vec<String> {
        vec![
            "build", "run", "test", "check", "clean", "doc", "new",
            "init", "add", "remove", "update", "search", "publish",
            "install", "uninstall", "bench", "fmt", "clippy", "fix",
            "tree", "audit", "outdated", "expand", "watch", "nextest"
        ].into_iter().map(String::from).collect()
    }

    /// Get default npm/yarn commands
    fn default_npm_commands() -> Vec<String> {
        vec![
            "install", "uninstall", "update", "run", "start", "test",
            "build", "dev", "serve", "lint", "format", "audit", "fund",
            "version", "publish", "pack", "link", "unlink", "config",
            "cache", "init", "create", "exec", "explore", "doctor"
        ].into_iter().map(String::from).collect()
    }

    /// Detect command context from the input
    fn detect_command_context(&self, context: &CompletionContext) -> CommandContext {
        let text = &context.text[..context.cursor_pos];
        let words: Vec<&str> = text.split_whitespace().collect();

        // If empty or still typing the first word (no space yet), it's a root command
        if words.is_empty() || (words.len() == 1 && !text.ends_with(' ')) {
            return CommandContext::Root;
        }

        match words[0] {
            "git" => CommandContext::Git,
            "cargo" => CommandContext::Cargo,
            "npm" | "yarn" | "pnpm" => CommandContext::Npm,
            "docker" => CommandContext::Docker,
            "kubectl" => CommandContext::Kubernetes,
            _ => {
                // Check if it's a Goofy tool command
                if self.tool_commands.contains(&words[0].to_string()) {
                    CommandContext::Tool
                } else {
                    CommandContext::System
                }
            }
        }
    }

    /// Get completions for root command (first word)
    async fn complete_root_command(&self, prefix: &str) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        
        // Add shell builtins
        for cmd in &self.shell_builtins {
            if cmd.starts_with(prefix) {
                items.push(
                    CompletionItem::new(cmd, cmd, "shell")
                        .with_description("Shell builtin command".to_string())
                        .with_score(0.9)
                );
            }
        }
        
        // Add tool commands (high priority)
        for cmd in &self.tool_commands {
            if cmd.starts_with(prefix) {
                items.push(
                    CompletionItem::new(cmd, cmd, "tool")
                        .with_description("Goofy tool command".to_string())
                        .with_score(1.0)
                );
            }
        }
        
        // Add common system commands
        let common_commands = [
            ("ls", "List directory contents"),
            ("cd", "Change directory"),
            ("pwd", "Print working directory"),
            ("cat", "Display file contents"),
            ("grep", "Search text patterns"),
            ("find", "Find files and directories"),
            ("git", "Version control system"),
            ("cargo", "Rust package manager"),
            ("npm", "Node package manager"),
            ("docker", "Container platform"),
            ("vim", "Text editor"),
            ("nano", "Text editor"),
            ("code", "VS Code editor"),
            ("curl", "Transfer data from servers"),
            ("wget", "Download files"),
            ("tar", "Archive files"),
            ("zip", "Compress files"),
            ("unzip", "Decompress files"),
            ("ps", "List running processes"),
            ("top", "Display running processes"),
            ("kill", "Terminate processes"),
            ("ssh", "Secure shell connection"),
            ("scp", "Secure copy"),
            ("rsync", "Synchronize files"),
        ];
        
        for &(cmd, desc) in &common_commands {
            if cmd.starts_with(prefix) {
                items.push(
                    CompletionItem::new(cmd, cmd, "system")
                        .with_description(desc.to_string())
                        .with_score(0.8)
                );
            }
        }
        
        // Add system commands from PATH (lower priority)
        for cmd in &self.system_commands {
            if cmd.starts_with(prefix) && !items.iter().any(|i| i.title == *cmd) {
                items.push(
                    CompletionItem::new(cmd, cmd, "system")
                        .with_description("System command".to_string())
                        .with_score(0.5)
                );
            }
        }
        
        Ok(items)
    }

    /// Get completions for git subcommands
    async fn complete_git_command(&self, prefix: &str) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        
        for cmd in &self.git_commands {
            if cmd.starts_with(prefix) {
                let description = match cmd.as_str() {
                    "add" => "Add files to staging area",
                    "commit" => "Create a new commit",
                    "push" => "Upload changes to remote",
                    "pull" => "Download changes from remote",
                    "status" => "Show working tree status",
                    "log" => "Show commit history",
                    "diff" => "Show changes between commits",
                    "branch" => "List, create, or delete branches",
                    "checkout" => "Switch branches or restore files",
                    "merge" => "Merge branches",
                    "rebase" => "Reapply commits on top of another base",
                    _ => "Git subcommand",
                };
                
                items.push(
                    CompletionItem::new(cmd, cmd, "git")
                        .with_description(description.to_string())
                        .with_score(0.9)
                );
            }
        }
        
        Ok(items)
    }

    /// Get completions for cargo subcommands
    async fn complete_cargo_command(&self, prefix: &str) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        
        for cmd in &self.cargo_commands {
            if cmd.starts_with(prefix) {
                let description = match cmd.as_str() {
                    "build" => "Compile the current package",
                    "run" => "Run the current package",
                    "test" => "Run tests",
                    "check" => "Check without producing executables",
                    "clean" => "Remove build artifacts",
                    "doc" => "Build documentation",
                    "new" => "Create a new cargo package",
                    "add" => "Add dependencies",
                    "update" => "Update dependencies",
                    "clippy" => "Run the Clippy linter",
                    "fmt" => "Format source code",
                    _ => "Cargo subcommand",
                };
                
                items.push(
                    CompletionItem::new(cmd, cmd, "cargo")
                        .with_description(description.to_string())
                        .with_score(0.9)
                );
            }
        }
        
        Ok(items)
    }

    /// Get completions for npm/yarn commands
    async fn complete_npm_command(&self, prefix: &str) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        
        for cmd in &self.npm_commands {
            if cmd.starts_with(prefix) {
                let description = match cmd.as_str() {
                    "install" => "Install dependencies",
                    "uninstall" => "Remove dependencies",
                    "run" => "Run package scripts",
                    "start" => "Start the application",
                    "test" => "Run tests",
                    "build" => "Build the application",
                    "dev" => "Start development server",
                    "update" => "Update dependencies",
                    "audit" => "Check for vulnerabilities",
                    _ => "NPM subcommand",
                };
                
                items.push(
                    CompletionItem::new(cmd, cmd, "npm")
                        .with_description(description.to_string())
                        .with_score(0.9)
                );
            }
        }
        
        Ok(items)
    }

    /// Get contextual flag completions
    async fn complete_flags(&self, command: &str, prefix: &str) -> Result<Vec<CompletionItem>> {
        let mut items = Vec::new();
        
        // Common flags for all commands
        let common_flags = [
            ("--help", "Show help information"),
            ("--version", "Show version information"),
            ("--verbose", "Enable verbose output"),
            ("--quiet", "Reduce output"),
            ("--dry-run", "Show what would be done"),
        ];
        
        for &(flag, desc) in &common_flags {
            if flag.starts_with(prefix) {
                items.push(
                    CompletionItem::new(flag, flag, "flag")
                        .with_description(desc.to_string())
                        .with_score(0.7)
                );
            }
        }
        
        // Command-specific flags
        let specific_flags = match command {
            "git" => vec![
                ("--all", "Include all refs"),
                ("--force", "Force the operation"),
                ("--no-verify", "Skip pre-commit hooks"),
                ("--amend", "Amend the previous commit"),
            ],
            "cargo" => vec![
                ("--release", "Build in release mode"),
                ("--target", "Specify target triple"),
                ("--features", "Enable specific features"),
                ("--no-default-features", "Disable default features"),
                ("--workspace", "Apply to entire workspace"),
            ],
            "npm" => vec![
                ("--save", "Save to dependencies"),
                ("--save-dev", "Save to devDependencies"),
                ("--global", "Install globally"),
                ("--production", "Skip devDependencies"),
            ],
            _ => vec![],
        };
        
        for (flag, desc) in specific_flags {
            if flag.starts_with(prefix) {
                items.push(
                    CompletionItem::new(flag, flag, "flag")
                        .with_description(desc.to_string())
                        .with_score(0.8)
                );
            }
        }
        
        Ok(items)
    }
}

impl Default for CommandProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Command context types
#[derive(Debug, Clone, PartialEq)]
enum CommandContext {
    Root,      // First word of command
    Git,       // git subcommand
    Cargo,     // cargo subcommand
    Npm,       // npm/yarn subcommand
    Docker,    // docker subcommand
    Kubernetes, // kubectl subcommand
    Tool,      // Goofy tool command
    System,    // Other system command
}

#[async_trait]
impl CompletionProvider for CommandProvider {
    fn name(&self) -> &str {
        "command"
    }

    async fn get_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionItem>> {
        let current_word = context.current_word();
        debug!("Command completion for: '{}'", current_word);
        
        // Check if we're completing a flag
        if current_word.starts_with('-') {
            let text = &context.text[..context.cursor_pos];
            let words: Vec<&str> = text.split_whitespace().collect();
            let base_command = words.first().unwrap_or(&"");
            return self.complete_flags(base_command, current_word).await;
        }
        
        if !self.context_aware {
            return self.complete_root_command(current_word).await;
        }
        
        let cmd_context = self.detect_command_context(context);
        
        match cmd_context {
            CommandContext::Root => self.complete_root_command(current_word).await,
            CommandContext::Git => self.complete_git_command(current_word).await,
            CommandContext::Cargo => self.complete_cargo_command(current_word).await,
            CommandContext::Npm => self.complete_npm_command(current_word).await,
            CommandContext::Tool => {
                // TODO: Implement tool-specific argument completion
                Ok(Vec::new())
            },
            CommandContext::System | CommandContext::Docker | CommandContext::Kubernetes => {
                // For now, just return basic completions
                Ok(Vec::new())
            }
        }
    }

    fn is_applicable(&self, context: &CompletionContext) -> bool {
        // Apply to command contexts
        let text = &context.text[..context.cursor_pos];
        let words: Vec<&str> = text.split_whitespace().collect();
        
        // If we're at the beginning or after whitespace, we might be typing a command
        words.is_empty() || 
        context.prefix().trim().is_empty() ||
        context.command_context.is_some()
    }

    fn get_priority(&self, context: &CompletionContext) -> i32 {
        if context.is_command() {
            15 // High priority for command contexts
        } else {
            5  // Medium priority otherwise
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_provider_basic() {
        let provider = CommandProvider::new();
        
        let context = CompletionContext::new("gi", 2);
        let completions = provider.get_completions(&context).await.unwrap();
        
        assert!(completions.iter().any(|c| c.title == "git"));
    }

    #[tokio::test]
    async fn test_git_subcommand_completion() {
        let provider = CommandProvider::new();
        
        let context = CompletionContext {
            text: "git pu".to_string(),
            cursor_pos: 6,
            command_context: Some("git".to_string()),
            ..Default::default()
        };
        
        let completions = provider.get_completions(&context).await.unwrap();
        
        assert!(completions.iter().any(|c| c.title == "push"));
        assert!(completions.iter().any(|c| c.title == "pull"));
    }

    #[tokio::test]
    async fn test_cargo_subcommand_completion() {
        let provider = CommandProvider::new();
        
        let context = CompletionContext {
            text: "cargo bu".to_string(),
            cursor_pos: 8,
            command_context: Some("cargo".to_string()),
            ..Default::default()
        };
        
        let completions = provider.get_completions(&context).await.unwrap();
        
        assert!(completions.iter().any(|c| c.title == "build"));
    }

    #[tokio::test]
    async fn test_flag_completion() {
        let provider = CommandProvider::new();
        
        let context = CompletionContext::new("git commit --", 12);
        let completions = provider.get_completions(&context).await.unwrap();
        
        assert!(completions.iter().any(|c| c.title == "--help"));
        assert!(completions.iter().any(|c| c.title == "--amend"));
    }

    #[test]
    fn test_command_context_detection() {
        let provider = CommandProvider::new();
        
        let context1 = CompletionContext::new("git status", 6);
        assert_eq!(provider.detect_command_context(&context1), CommandContext::Git);
        
        let context2 = CompletionContext::new("cargo build", 8);
        assert_eq!(provider.detect_command_context(&context2), CommandContext::Cargo);
        
        let context3 = CompletionContext::new("npm install", 8);
        assert_eq!(provider.detect_command_context(&context3), CommandContext::Npm);
    }
}