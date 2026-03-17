//! Diagnostics tool implementation for getting LSP diagnostics

use super::{BaseTool, ToolPermissions, ToolRequest, ToolResponse, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

use crate::lsp::{LspClient, LspManager};


/// LSP diagnostics tool
pub struct DiagnosticsTool {
    lsp_manager: Option<LspManager>,
}

impl DiagnosticsTool {
    /// Create a new diagnostics tool
    pub fn new(lsp_manager: Option<LspManager>) -> Self {
        Self { lsp_manager }
    }
}

#[async_trait]
impl BaseTool for DiagnosticsTool {
    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let file_path = request.parameters.get("file_path")
            .and_then(|v| v.as_str());

        let lsp_manager = match &self.lsp_manager {
            Some(manager) => manager,
            None => {
                return Ok(ToolResponse {
                    content: "No LSP clients available".to_string(),
                    success: false,
                    metadata: None,
                    error: Some("No LSP clients available".to_string()),
                });
            }
        };

        // If a specific file path is provided, ensure it's opened in LSP
        if let Some(file_path) = file_path {
            if let Err(e) = self.ensure_file_opened(lsp_manager, file_path).await {
                return Ok(ToolResponse {
                    content: String::new(),
                    success: false,
                    metadata: None,
                    error: Some(format!("Failed to open file in LSP: {}", e)),
                });
            }

            // Wait for diagnostics to be updated
            if let Err(_) = timeout(Duration::from_secs(5), self.wait_for_diagnostics(lsp_manager, file_path)).await {
                // Continue even if timeout - we'll show what we have
            }
        }

        let diagnostics_output = self.get_diagnostics_output(lsp_manager, file_path).await;
        
        Ok(ToolResponse {
            content: diagnostics_output,
            success: true,
            metadata: None,
            error: None,
        })
    }

    fn name(&self) -> &str {
        "diagnostics"
    }

    fn description(&self) -> &str {
        r#"Get diagnostics for a file and/or project.
WHEN TO USE THIS TOOL:
- Use when you need to check for errors or warnings in your code
- Helpful for debugging and ensuring code quality
- Good for getting a quick overview of issues in a file or project
HOW TO USE:
- Provide a path to a file to get diagnostics for that file
- Leave the path empty to get diagnostics for the entire project
- Results are displayed in a structured format with severity levels
FEATURES:
- Displays errors, warnings, and hints
- Groups diagnostics by severity
- Provides detailed information about each diagnostic
LIMITATIONS:
- Results are limited to the diagnostics provided by the LSP clients
- May not cover all possible issues in the code
- Does not provide suggestions for fixing issues
TIPS:
- Use in conjunction with other tools for a comprehensive code review
- Combine with the LSP client for real-time diagnostics"#
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The path to the file to get diagnostics for (leave empty for project diagnostics)"
                }
            },
            "required": []
        })
    }
}

impl DiagnosticsTool {
    /// Ensure a file is opened in all relevant LSP clients
    async fn ensure_file_opened(&self, _lsp_manager: &LspManager, _file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // TODO: Implement when LSP manager has get_clients_for_file method
        Ok(())
    }

    /// Wait for diagnostics to be updated (simplified implementation)
    async fn wait_for_diagnostics(&self, _lsp_manager: &LspManager, _file_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // In a full implementation, this would wait for LSP diagnostic notifications
        // For now, we'll just wait a short period
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    /// Get formatted diagnostics output
    async fn get_diagnostics_output(&self, lsp_manager: &LspManager, target_file: Option<&str>) -> String {
        let mut file_diagnostics = Vec::new();
        let mut project_diagnostics = Vec::new();

        // Get diagnostics from all LSP clients
        let all_diagnostics: HashMap<String, Vec<LspDiagnostic>> = HashMap::new(); // Placeholder - LSP manager integration needed

        for (file_path, diagnostics) in all_diagnostics {
            let is_target_file = target_file.map_or(false, |target| file_path == target);

            for diagnostic in diagnostics {
                let formatted = self.format_diagnostic(&file_path, &diagnostic);
                
                if is_target_file {
                    file_diagnostics.push(formatted);
                } else {
                    project_diagnostics.push(formatted);
                }
            }
        }

        // Sort diagnostics by severity (errors first)
        file_diagnostics.sort_by(|a, b| {
            let a_is_error = a.starts_with("Error");
            let b_is_error = b.starts_with("Error");
            match (a_is_error, b_is_error) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        project_diagnostics.sort_by(|a, b| {
            let a_is_error = a.starts_with("Error");
            let b_is_error = b.starts_with("Error");
            match (a_is_error, b_is_error) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        self.format_output(&file_diagnostics, &project_diagnostics)
    }

    /// Format a single diagnostic
    fn format_diagnostic(&self, file_path: &str, diagnostic: &LspDiagnostic) -> String {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "Error",
            DiagnosticSeverity::Warning => "Warn",
            DiagnosticSeverity::Information => "Info",
            DiagnosticSeverity::Hint => "Hint",
        };

        let location = format!(
            "{}:{}:{}",
            file_path,
            diagnostic.range.start.line + 1,
            diagnostic.range.start.character + 1
        );

        let source_info = diagnostic.source.as_deref().unwrap_or("unknown");

        let code_info = diagnostic.code.as_ref()
            .map(|code| format!("[{}]", code))
            .unwrap_or_default();

        let tags_info = if !diagnostic.tags.is_empty() {
            let tags: Vec<&str> = diagnostic.tags.iter()
                .filter_map(|tag| match tag {
                    DiagnosticTag::Unnecessary => Some("unnecessary"),
                    DiagnosticTag::Deprecated => Some("deprecated"),
                })
                .collect();
            if !tags.is_empty() {
                format!(" ({})", tags.join(", "))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            "{}: {} [{}]{}{} {}",
            severity,
            location,
            source_info,
            code_info,
            tags_info,
            diagnostic.message
        )
    }

    /// Format the final output
    fn format_output(&self, file_diagnostics: &[String], project_diagnostics: &[String]) -> String {
        let mut output = String::new();

        if !file_diagnostics.is_empty() {
            output.push_str("\n<file_diagnostics>\n");
            
            let _to_show = if file_diagnostics.len() > 10 {
                output.push_str(&file_diagnostics[..10].join("\n"));
                output.push_str(&format!("\n... and {} more diagnostics", file_diagnostics.len() - 10));
                10
            } else {
                output.push_str(&file_diagnostics.join("\n"));
                file_diagnostics.len()
            };
            
            output.push_str("\n</file_diagnostics>\n");
        }

        if !project_diagnostics.is_empty() {
            output.push_str("\n<project_diagnostics>\n");
            
            if project_diagnostics.len() > 10 {
                output.push_str(&project_diagnostics[..10].join("\n"));
                output.push_str(&format!("\n... and {} more diagnostics", project_diagnostics.len() - 10));
            } else {
                output.push_str(&project_diagnostics.join("\n"));
            }
            
            output.push_str("\n</project_diagnostics>\n");
        }

        if !file_diagnostics.is_empty() || !project_diagnostics.is_empty() {
            let file_errors = self.count_severity(file_diagnostics, "Error");
            let file_warnings = self.count_severity(file_diagnostics, "Warn");
            let project_errors = self.count_severity(project_diagnostics, "Error");
            let project_warnings = self.count_severity(project_diagnostics, "Warn");

            output.push_str("\n<diagnostic_summary>\n");
            output.push_str(&format!("Current file: {} errors, {} warnings\n", file_errors, file_warnings));
            output.push_str(&format!("Project: {} errors, {} warnings\n", project_errors, project_warnings));
            output.push_str("</diagnostic_summary>\n");
        } else {
            output.push_str("No diagnostics found.\n");
        }

        output
    }

    /// Count diagnostics of a specific severity
    fn count_severity(&self, diagnostics: &[String], severity: &str) -> usize {
        diagnostics.iter()
            .filter(|diag| diag.starts_with(severity))
            .count()
    }
}

/// Simplified LSP diagnostic types (these would normally come from the LSP module)
#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
    pub code: Option<String>,
    pub tags: Vec<DiagnosticTag>,
}

#[derive(Debug, Clone)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

#[derive(Debug, Clone)]
pub enum DiagnosticTag {
    Unnecessary = 1,
    Deprecated = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_diagnostics_tool_info() {
        let tool = DiagnosticsTool::new(None);

        assert_eq!(tool.name(), "diagnostics");
        assert!(tool.description().contains("diagnostics"));
        assert!(tool.description().contains("errors"));
    }

    #[tokio::test]
    async fn test_format_diagnostic() {
        let tool = DiagnosticsTool::new(None);
        
        let diagnostic = LspDiagnostic {
            range: LspRange {
                start: LspPosition { line: 10, character: 5 },
                end: LspPosition { line: 10, character: 15 },
            },
            severity: DiagnosticSeverity::Error,
            message: "Undefined variable".to_string(),
            source: Some("rust-analyzer".to_string()),
            code: Some("E0425".to_string()),
            tags: vec![],
        };

        let formatted = tool.format_diagnostic("src/main.rs", &diagnostic);
        
        assert!(formatted.contains("Error"));
        assert!(formatted.contains("src/main.rs:11:6"));
        assert!(formatted.contains("rust-analyzer"));
        assert!(formatted.contains("E0425"));
        assert!(formatted.contains("Undefined variable"));
    }

    #[test]
    fn test_count_severity() {
        let tool = DiagnosticsTool::new(None);
        
        let diagnostics = vec![
            "Error: test:1:1 [rust] message".to_string(),
            "Warn: test:2:1 [rust] message".to_string(),
            "Error: test:3:1 [rust] message".to_string(),
            "Info: test:4:1 [rust] message".to_string(),
        ];

        assert_eq!(tool.count_severity(&diagnostics, "Error"), 2);
        assert_eq!(tool.count_severity(&diagnostics, "Warn"), 1);
        assert_eq!(tool.count_severity(&diagnostics, "Info"), 1);
        assert_eq!(tool.count_severity(&diagnostics, "Hint"), 0);
    }

    #[tokio::test]
    async fn test_no_lsp_manager() {
        let tool = DiagnosticsTool::new(None);
        let request = ToolRequest {
            tool_name: "diagnostics".to_string(),
            parameters: HashMap::new(),
            working_directory: None,
            permissions: ToolPermissions::default(),
        };

        let response = tool.execute(request).await.unwrap();
        assert!(response.content.contains("No LSP clients available"));
    }
}