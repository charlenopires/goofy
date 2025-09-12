//! Bash command execution tool

use super::{BaseTool, ToolRequest, ToolResponse, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;
use crate::shell::persistent::{get_persistent_shell, execute_in_persistent};

/// Tool for executing bash commands
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    /// Execute a command with timeout and safety checks using persistent shell
    async fn execute_command(&self, command: &str, working_dir: Option<&str>, timeout_ms: u64) -> ToolResult<(String, String, i32)> {
        // Set working directory if provided
        if let Some(dir) = working_dir {
            let shell = get_persistent_shell().await;
            let shell = shell.lock().await;
            if let Err(e) = shell.shell().set_working_dir(dir).await {
                return Err(anyhow::anyhow!("Failed to set working directory: {}", e));
            }
        }

        let timeout_duration = Duration::from_millis(timeout_ms);
        
        // Execute command with timeout using persistent shell
        match timeout(timeout_duration, execute_in_persistent(command)).await {
            Ok(Ok((stdout, stderr, exit_code))) => {
                Ok((stdout, stderr, exit_code))
            }
            Ok(Err(e)) => Err(anyhow::anyhow!("Command execution failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Command timed out after {}ms", timeout_ms)),
        }
    }

    /// Check if command is potentially dangerous
    /// The shell module already has built-in command blocking, but we keep additional checks here
    fn is_dangerous_command(&self, command: &str) -> bool {
        // Additional dangerous patterns not covered by shell's built-in blocker
        let dangerous_patterns = [
            "curl", "wget", "nc", "netcat", // Network commands (can be restricted)
            "python -c", "perl -e", "ruby -e", // Inline script execution
            "chmod 777 /", "chown root", // Permission changes
        ];

        dangerous_patterns.iter().any(|&pattern| command.contains(pattern))
    }
}

#[async_trait]
impl BaseTool for BashTool {
    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let command = request.parameters.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: command"))?;

        let timeout_ms = request.parameters.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120000); // Default 2 minutes

        let description = request.parameters.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Execute command");

        // Security checks
        if !request.permissions.allow_execute && !request.permissions.yolo_mode {
            return Ok(ToolResponse {
                content: String::new(),
                success: false,
                metadata: None,
                error: Some("Command execution not permitted. Use --yolo flag or grant execute permissions.".to_string()),
            });
        }

        if self.is_dangerous_command(command) && !request.permissions.yolo_mode {
            return Ok(ToolResponse {
                content: String::new(),
                success: false,
                metadata: None,
                error: Some(format!("Potentially dangerous command detected: '{}'. Use --yolo mode to override.", command)),
            });
        }

        // Execute command
        match self.execute_command(command, request.working_directory.as_deref(), timeout_ms).await {
            Ok((stdout, stderr, exit_code)) => {
                let mut output = String::new();
                
                if !stdout.is_empty() {
                    output.push_str(&stdout);
                }
                
                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push_str("\n--- STDERR ---\n");
                    }
                    output.push_str(&stderr);
                }

                if output.is_empty() {
                    output = "(No output)".to_string();
                }

                let metadata = json!({
                    "command": command,
                    "description": description,
                    "exit_code": exit_code,
                    "timeout_ms": timeout_ms,
                    "stdout_length": stdout.len(),
                    "stderr_length": stderr.len(),
                });

                Ok(ToolResponse {
                    content: output,
                    success: exit_code == 0,
                    metadata: Some(metadata),
                    error: if exit_code != 0 {
                        Some(format!("Command exited with code {}", exit_code))
                    } else {
                        None
                    },
                })
            }
            Err(e) => Ok(ToolResponse {
                content: String::new(),
                success: false,
                metadata: Some(json!({
                    "command": command,
                    "description": description,
                })),
                error: Some(e.to_string()),
            })
        }
    }

    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute bash commands in a persistent shell session with optional timeout and safety measures."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in 5-10 words"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in milliseconds (max 600000, default 120000)"
                }
            },
            "required": ["command"]
        })
    }

    fn requires_permission(&self) -> bool {
        true // Command execution always requires permission
    }

    fn validate_request(&self, request: &ToolRequest) -> ToolResult<()> {
        // Basic validation for execute permission
        if !request.permissions.allow_execute && !request.permissions.yolo_mode {
            return Err(anyhow::anyhow!("Tool '{}' requires execute permission", self.name()));
        }
        
        // Additional bash-specific validation
        let timeout = request.parameters.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120000);
            
        if timeout > 600000 {
            return Err(anyhow::anyhow!("Timeout cannot exceed 600000ms (10 minutes)"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::llm::tools::{ToolPermissions, ToolRequest};

    #[tokio::test]
    async fn test_simple_command() {
        let tool = BashTool::new();
        let mut params = HashMap::new();
        params.insert("command".to_string(), json!("echo 'Hello, World!'"));
        
        let mut permissions = ToolPermissions::default();
        permissions.allow_execute = true;
        
        let request = ToolRequest {
            tool_name: "bash".to_string(),
            parameters: params,
            working_directory: None,
            permissions,
        };
        
        let response = tool.execute(request).await.unwrap();
        assert!(response.success);
        assert!(response.content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_permission_denied() {
        let tool = BashTool::new();
        let mut params = HashMap::new();
        params.insert("command".to_string(), json!("echo 'test'"));
        
        let permissions = ToolPermissions::default(); // execute = false by default
        
        let request = ToolRequest {
            tool_name: "bash".to_string(),
            parameters: params,
            working_directory: None,
            permissions,
        };
        
        let response = tool.execute(request).await.unwrap();
        assert!(!response.success);
        assert!(response.error.is_some());
        assert!(response.error.unwrap().contains("not permitted"));
    }

    #[tokio::test]
    async fn test_dangerous_command_detection() {
        let tool = BashTool::new();
        assert!(tool.is_dangerous_command("rm -rf /"));
        assert!(tool.is_dangerous_command("shutdown now"));
        assert!(!tool.is_dangerous_command("ls -la"));
        assert!(!tool.is_dangerous_command("grep pattern file.txt"));
    }

    #[tokio::test]
    async fn test_yolo_mode_override() {
        let tool = BashTool::new();
        let mut params = HashMap::new();
        params.insert("command".to_string(), json!("echo 'dangerous'")); // Not actually dangerous
        
        let mut permissions = ToolPermissions::default();
        permissions.yolo_mode = true; // Should override permission checks
        
        let request = ToolRequest {
            tool_name: "bash".to_string(),
            parameters: params,
            working_directory: None,
            permissions,
        };
        
        let response = tool.execute(request).await.unwrap();
        assert!(response.success);
    }
}