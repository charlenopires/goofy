//! Tools system for AI agent interactions
//!
//! This module provides a comprehensive set of tools that AI agents can use
//! to interact with the file system, execute commands, and perform various
//! development tasks.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

pub mod bash;
pub mod file;
pub mod edit;
pub mod multiedit;
pub mod grep;
pub mod rg;
pub mod glob;
pub mod ls;
pub mod safe;
pub mod download;
pub mod diagnostics;
pub mod fetch;
pub mod view;
pub mod write;
pub mod shell;

pub use bash::BashTool;
pub use file::FileTool;
pub use edit::EditTool;
pub use multiedit::MultiEditTool;
pub use grep::GrepTool;
pub use rg::RgTool;
pub use glob::GlobTool;
pub use ls::LsTool;
pub use safe::SafeValidator;
pub use download::DownloadTool;
pub use diagnostics::DiagnosticsTool;
pub use fetch::FetchTool;
pub use view::ViewTool;
pub use write::WriteTool;
pub use shell::ShellTool;

// Re-export for easier access in tests (types defined below)

/// Request structure for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub working_directory: Option<String>,
    pub permissions: ToolPermissions,
}

/// Tool execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub content: String,
    pub success: bool,
    pub metadata: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Permission settings for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_execute: bool,
    pub allow_network: bool,
    pub restricted_paths: Vec<String>,
    pub yolo_mode: bool,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            allow_read: true,
            allow_write: false,
            allow_execute: false,
            allow_network: false,
            restricted_paths: vec![
                "/etc".to_string(),
                "/sys".to_string(),
                "/proc".to_string(),
                "/dev".to_string(),
            ],
            yolo_mode: false,
        }
    }
}

/// Result type for tool operations
pub type ToolResult<T> = Result<T>;

/// Core trait that all tools must implement
#[async_trait]
pub trait BaseTool: Send + Sync {
    /// Execute the tool with the given request
    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse>;
    
    /// Get the tool's name
    fn name(&self) -> &str;
    
    /// Get the tool's description
    fn description(&self) -> &str;
    
    /// Get the tool's parameter schema (JSON Schema)
    fn parameters(&self) -> serde_json::Value;
    
    /// Check if this tool requires special permissions
    fn requires_permission(&self) -> bool {
        true
    }
    
    /// Validate the tool request before execution
    fn validate_request(&self, request: &ToolRequest) -> ToolResult<()> {
        // Basic validation - can be overridden by specific tools
        if !request.permissions.yolo_mode && self.requires_permission() {
            if request.tool_name == "bash" && !request.permissions.allow_execute {
                return Err(anyhow::anyhow!("Tool '{}' requires execute permission", self.name()));
            }
            if (request.tool_name == "edit" || request.tool_name == "multiedit") && !request.permissions.allow_write {
                return Err(anyhow::anyhow!("Tool '{}' requires write permission", self.name()));
            }
        }
        Ok(())
    }
}

/// Tool manager for registering and executing tools
pub struct ToolManager {
    tools: HashMap<String, Box<dyn BaseTool>>,
    permissions: ToolPermissions,
}

impl ToolManager {
    /// Create a new tool manager
    pub fn new(permissions: ToolPermissions) -> Self {
        let mut manager = Self {
            tools: HashMap::new(),
            permissions,
        };
        
        // Register default tools
        manager.register_default_tools();
        manager
    }
    
    /// Register all default tools
    fn register_default_tools(&mut self) {
        self.register_tool(Box::new(FileTool::new()));
        self.register_tool(Box::new(EditTool::new()));
        self.register_tool(Box::new(MultiEditTool::new()));
        self.register_tool(Box::new(BashTool::new()));
        self.register_tool(Box::new(ShellTool::new()));
        self.register_tool(Box::new(GrepTool::new()));
        self.register_tool(Box::new(RgTool::new()));
        self.register_tool(Box::new(GlobTool::new()));
        self.register_tool(Box::new(LsTool::new()));
        self.register_tool(Box::new(DownloadTool::new()));
        self.register_tool(Box::new(DiagnosticsTool::new(None))); // No LSP manager by default
        self.register_tool(Box::new(FetchTool::new()));
        self.register_tool(Box::new(ViewTool::new()));
        self.register_tool(Box::new(WriteTool::new()));
    }
    
    /// Register a tool
    pub fn register_tool(&mut self, tool: Box<dyn BaseTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    /// Execute a tool by name
    pub async fn execute_tool(&self, tool_name: &str, parameters: HashMap<String, serde_json::Value>) -> ToolResult<ToolResponse> {
        let tool = self.tools.get(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;
        
        let request = ToolRequest {
            tool_name: tool_name.to_string(),
            parameters,
            working_directory: None, // Could be set from context
            permissions: self.permissions.clone(),
        };
        
        // Validate request
        tool.validate_request(&request)?;
        
        // Execute tool
        tool.execute(request).await
    }
    
    /// Get list of available tools
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }
    
    /// Get tool definitions for LLM providers
    pub fn get_tool_definitions(&self) -> Vec<crate::llm::types::Tool> {
        self.tools.values().map(|tool| {
            crate::llm::types::Tool {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.parameters(),
            }
        }).collect()
    }
    
    /// Update permissions
    pub fn update_permissions(&mut self, permissions: ToolPermissions) {
        self.permissions = permissions;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_manager_creation() {
        let permissions = ToolPermissions::default();
        let manager = ToolManager::new(permissions);
        
        // Should have registered default tools
        assert!(!manager.list_tools().is_empty());
        assert!(manager.list_tools().contains(&"file".to_string()));
        assert!(manager.list_tools().contains(&"edit".to_string()));
    }
    
    #[tokio::test]
    async fn test_tool_definitions() {
        let permissions = ToolPermissions::default();
        let manager = ToolManager::new(permissions);
        let definitions = manager.get_tool_definitions();
        
        assert!(!definitions.is_empty());
        assert!(definitions.iter().any(|t| t.name == "file"));
    }
}