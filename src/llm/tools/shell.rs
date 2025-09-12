//! Shell management tool for persistent shell operations

use super::{BaseTool, ToolRequest, ToolResponse, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use crate::shell::persistent::{get_persistent_shell, get_persistent_working_dir, set_persistent_working_dir};

/// Tool for shell management operations
pub struct ShellTool;

impl ShellTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BaseTool for ShellTool {
    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let operation = request.parameters.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: operation"))?;

        match operation {
            "pwd" => {
                let dir = get_persistent_working_dir().await;
                Ok(ToolResponse {
                    content: dir.display().to_string(),
                    success: true,
                    metadata: Some(json!({
                        "operation": "pwd",
                        "path": dir.display().to_string(),
                    })),
                    error: None,
                })
            }
            
            "cd" => {
                let path = request.parameters.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path for cd operation"))?;
                
                match set_persistent_working_dir(path).await {
                    Ok(_) => {
                        let new_dir = get_persistent_working_dir().await;
                        Ok(ToolResponse {
                            content: format!("Changed directory to: {}", new_dir.display()),
                            success: true,
                            metadata: Some(json!({
                                "operation": "cd",
                                "path": new_dir.display().to_string(),
                            })),
                            error: None,
                        })
                    }
                    Err(e) => Ok(ToolResponse {
                        content: String::new(),
                        success: false,
                        metadata: Some(json!({
                            "operation": "cd",
                            "attempted_path": path,
                        })),
                        error: Some(e.to_string()),
                    })
                }
            }
            
            "history" => {
                let count = request.parameters.get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;
                
                let shell = get_persistent_shell().await;
                let shell = shell.lock().await;
                let history = shell.get_recent_history(count);
                
                Ok(ToolResponse {
                    content: history.join("\n"),
                    success: true,
                    metadata: Some(json!({
                        "operation": "history",
                        "count": history.len(),
                    })),
                    error: None,
                })
            }
            
            "search_history" => {
                let pattern = request.parameters.get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: pattern for search_history"))?;
                
                let shell = get_persistent_shell().await;
                let shell = shell.lock().await;
                let matches = shell.search_history(pattern);
                
                Ok(ToolResponse {
                    content: matches.join("\n"),
                    success: true,
                    metadata: Some(json!({
                        "operation": "search_history",
                        "pattern": pattern,
                        "count": matches.len(),
                    })),
                    error: None,
                })
            }
            
            "clear_history" => {
                let shell = get_persistent_shell().await;
                let mut shell = shell.lock().await;
                shell.clear_history();
                
                Ok(ToolResponse {
                    content: "Command history cleared".to_string(),
                    success: true,
                    metadata: Some(json!({
                        "operation": "clear_history",
                    })),
                    error: None,
                })
            }
            
            "env" => {
                let shell = get_persistent_shell().await;
                let shell = shell.lock().await;
                
                if let Some(key) = request.parameters.get("key").and_then(|v| v.as_str()) {
                    // Get specific env var
                    let value = shell.shell().get_env(key).await;
                    Ok(ToolResponse {
                        content: value.unwrap_or_else(|| "(not set)".to_string()),
                        success: true,
                        metadata: Some(json!({
                            "operation": "env",
                            "key": key,
                        })),
                        error: None,
                    })
                } else {
                    // Get all env vars
                    let env_vars = shell.shell().get_all_env().await;
                    let content: Vec<String> = env_vars.iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    
                    Ok(ToolResponse {
                        content: content.join("\n"),
                        success: true,
                        metadata: Some(json!({
                            "operation": "env",
                            "count": env_vars.len(),
                        })),
                        error: None,
                    })
                }
            }
            
            "setenv" => {
                let key = request.parameters.get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: key for setenv"))?;
                    
                let value = request.parameters.get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: value for setenv"))?;
                
                let shell = get_persistent_shell().await;
                let shell = shell.lock().await;
                shell.shell().set_env(key.to_string(), value.to_string()).await;
                
                Ok(ToolResponse {
                    content: format!("Set {}={}", key, value),
                    success: true,
                    metadata: Some(json!({
                        "operation": "setenv",
                        "key": key,
                        "value": value,
                    })),
                    error: None,
                })
            }
            
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation))
        }
    }

    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Manage the persistent shell environment (working directory, history, environment variables)"
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["pwd", "cd", "history", "search_history", "clear_history", "env", "setenv"],
                    "description": "The shell operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "Path for cd operation"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of history items to retrieve (default: 10)"
                },
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search in history"
                },
                "key": {
                    "type": "string",
                    "description": "Environment variable key"
                },
                "value": {
                    "type": "string",
                    "description": "Environment variable value"
                }
            },
            "required": ["operation"]
        })
    }

    fn requires_permission(&self) -> bool {
        false // Shell management operations are generally safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_pwd_operation() {
        let tool = ShellTool::new();
        let mut params = HashMap::new();
        params.insert("operation".to_string(), json!("pwd"));
        
        let request = ToolRequest {
            tool_name: "shell".to_string(),
            parameters: params,
            working_directory: None,
            permissions: Default::default(),
        };
        
        let response = tool.execute(request).await.unwrap();
        assert!(response.success);
        assert!(!response.content.is_empty());
    }
}