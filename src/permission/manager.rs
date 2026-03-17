//! Permission manager for coordinating permission decisions

use super::{PermissionConfig, PermissionContext, PermissionResult, PermissionValidator};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Manages permission decisions and user interactions
pub struct PermissionManager {
    validator: Arc<RwLock<PermissionValidator>>,
    session_grants: Arc<RwLock<HashMap<String, bool>>>, // Cache for session-based decisions
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new(config: PermissionConfig) -> Self {
        Self {
            validator: Arc::new(RwLock::new(PermissionValidator::new(config))),
            session_grants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check and potentially prompt for permission
    pub async fn check_permission(&self, context: PermissionContext) -> anyhow::Result<bool> {
        let (result, log_decisions) = {
            let validator = self.validator.read().await;
            let result = validator.check_permission(&context);
            let log_decisions = validator.get_config().log_decisions;
            (result, log_decisions)
        }; // Release the lock early

        match result {
            PermissionResult::Allowed => {
                if log_decisions {
                    info!("Permission granted for tool '{}' operation '{}'", 
                          context.tool_name, context.operation);
                }
                Ok(true)
            }
            PermissionResult::Denied(reason) => {
                warn!("Permission denied for tool '{}': {}", context.tool_name, reason);
                Err(anyhow::anyhow!("Permission denied: {}", reason))
            }
            PermissionResult::Prompt(message) => {
                self.handle_permission_prompt(context, message).await
            }
        }
    }

    /// Handle permission prompts (interactive or automatic based on mode)
    async fn handle_permission_prompt(&self, context: PermissionContext, message: String) -> anyhow::Result<bool> {
        // Create a unique key for this permission request
        let permission_key = format!("{}:{}:{}", 
            context.tool_name, 
            context.operation,
            context.file_path.as_ref()
                .map(|p| p.to_string_lossy())
                .unwrap_or_else(|| context.command.as_ref().map(|c| c.as_str()).unwrap_or("").into())
        );

        // Check if we already have a decision for this session
        {
            let session_grants = self.session_grants.read().await;
            if let Some(&granted) = session_grants.get(&permission_key) {
                return Ok(granted);
            }
        }

        // For now, we'll implement a simple auto-deny for non-interactive scenarios
        // In a full implementation, this would show a TUI prompt or use a callback
        let granted = self.auto_decide_permission(&context, &message).await;

        // Cache the decision for this session
        {
            let mut session_grants = self.session_grants.write().await;
            session_grants.insert(permission_key, granted);
        }

        if granted {
            info!("Permission granted after prompt for tool '{}': {}", context.tool_name, message);
            Ok(granted)
        } else {
            warn!("Permission denied after prompt for tool '{}': {}", context.tool_name, message);
            Err(anyhow::anyhow!("Permission denied: {}", message))
        }
    }

    /// Auto-decide permission based on risk assessment
    async fn auto_decide_permission(&self, context: &PermissionContext, _message: &str) -> bool {
        // For now, implement conservative auto-decisions
        // In a full implementation, this would be more sophisticated

        // Always allow read operations on non-restricted paths
        if matches!(context.risk_level, super::PermissionLevel::Read) {
            return true;
        }

        // Be more cautious with write and execute operations
        match context.tool_name.as_str() {
            "file" | "ls" | "grep" | "rg" => true,  // Generally safe tools
            "edit" => {
                // Allow edits in safe directories
                if let Some(file_path) = &context.file_path {
                    file_path.starts_with("/tmp") || 
                    file_path.starts_with("/var/tmp") ||
                    file_path.to_string_lossy().contains("/project/") // Assume project files are safe
                } else {
                    false
                }
            }
            "bash" => {
                // Very conservative with command execution
                if let Some(command) = &context.command {
                    // Allow only very safe commands
                    let safe_commands = ["ls", "pwd", "echo", "cat", "head", "tail", "grep", "find"];
                    safe_commands.iter().any(|&safe_cmd| command.trim().starts_with(safe_cmd))
                } else {
                    false
                }
            }
            _ => false, // Deny unknown tools by default
        }
    }

    /// Update the permission configuration
    pub async fn update_config(&self, config: PermissionConfig) {
        let mut validator = self.validator.write().await;
        validator.update_config(config);
    }

    /// Get current configuration
    pub async fn get_config(&self) -> PermissionConfig {
        let validator = self.validator.read().await;
        validator.get_config().clone()
    }

    /// Clear session-based permission cache
    pub async fn clear_session_cache(&self) {
        let mut session_grants = self.session_grants.write().await;
        session_grants.clear();
        info!("Permission session cache cleared");
    }

    /// Grant temporary permission for a specific operation
    pub async fn grant_temporary_permission(&self, tool_name: &str, operation: &str, target: &str) {
        let permission_key = format!("{}:{}:{}", tool_name, operation, target);
        let mut session_grants = self.session_grants.write().await;
        session_grants.insert(permission_key, true);
        info!("Temporary permission granted for '{}' '{}' on '{}'", tool_name, operation, target);
    }

    /// Deny temporary permission for a specific operation
    pub async fn deny_temporary_permission(&self, tool_name: &str, operation: &str, target: &str) {
        let permission_key = format!("{}:{}:{}", tool_name, operation, target);
        let mut session_grants = self.session_grants.write().await;
        session_grants.insert(permission_key, false);
        info!("Temporary permission denied for '{}' '{}' on '{}'", tool_name, operation, target);
    }

    /// Enable YOLO mode (bypass most restrictions)
    pub async fn enable_yolo_mode(&self) {
        let mut validator = self.validator.write().await;
        let mut config = validator.get_config().clone();
        config.yolo_mode = true;
        validator.update_config(config);
        warn!("YOLO mode enabled - most safety restrictions bypassed!");
    }

    /// Disable YOLO mode
    pub async fn disable_yolo_mode(&self) {
        let mut validator = self.validator.write().await;
        let mut config = validator.get_config().clone();
        config.yolo_mode = false;
        validator.update_config(config);
        info!("YOLO mode disabled - safety restrictions restored");
    }

    /// Check if YOLO mode is enabled
    pub async fn is_yolo_mode(&self) -> bool {
        let validator = self.validator.read().await;
        validator.get_config().yolo_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::permission::PermissionLevel;

    #[tokio::test]
    async fn test_permission_manager_creation() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        assert!(!manager.is_yolo_mode().await);
    }

    #[tokio::test]
    async fn test_yolo_mode_toggle() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        assert!(!manager.is_yolo_mode().await);
        
        manager.enable_yolo_mode().await;
        assert!(manager.is_yolo_mode().await);
        
        manager.disable_yolo_mode().await;
        assert!(!manager.is_yolo_mode().await);
    }

    #[tokio::test]
    async fn test_safe_operations_allowed() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        let context = PermissionContext::new("file".to_string(), "read".to_string())
            .with_file_path(PathBuf::from("/tmp/test.txt"));
        
        let result = manager.check_permission(context).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_dangerous_operations_denied() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        let context = PermissionContext::new("bash".to_string(), "execute".to_string())
            .with_command("rm -rf /".to_string())
            .with_risk_level(PermissionLevel::Dangerous);
        
        let result = manager.check_permission(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_cache() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        manager.grant_temporary_permission("test", "read", "/tmp/test.txt").await;
        
        let context = PermissionContext::new("test".to_string(), "read".to_string())
            .with_file_path(PathBuf::from("/tmp/test.txt"));
        
        let result = manager.check_permission(context).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_clear_session_cache() {
        let config = PermissionConfig::default();
        let manager = PermissionManager::new(config);
        
        manager.grant_temporary_permission("test", "read", "/tmp/test.txt").await;
        manager.clear_session_cache().await;
        
        // After clearing cache, the permission should be re-evaluated
        let context = PermissionContext::new("test".to_string(), "read".to_string())
            .with_file_path(PathBuf::from("/tmp/test.txt"));
        
        // This should now go through normal permission checking
        let result = manager.check_permission(context).await;
        // The result depends on the auto-decision logic, but it should not use cached grant
        assert!(result.is_ok()); // /tmp is generally safe
    }
}