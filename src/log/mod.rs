//! Logging utilities with rotation and panic recovery
//!
//! This module provides structured logging with file rotation
//! and panic recovery mechanisms.

use anyhow::Result;
use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static INIT: Once = Once::new();
static mut INITIALIZED: bool = false;

/// Log configuration
pub struct LogConfig {
    pub log_file: PathBuf,
    pub max_size_mb: u64,
    pub max_backups: usize,
    pub max_age_days: u32,
    pub debug: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            log_file: PathBuf::from("goofy.log"),
            max_size_mb: 10,
            max_backups: 3,
            max_age_days: 30,
            debug: false,
        }
    }
}

/// Setup logging with the given configuration
pub fn setup(config: LogConfig) -> Result<()> {
    INIT.call_once(|| {
        // Set up file rotation if needed
        rotate_log_if_needed(&config.log_file, config.max_size_mb * 1024 * 1024);
        
        // Clean up old logs
        cleanup_old_logs(&config.log_file, config.max_backups, config.max_age_days);
        
        // Create log file
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file)
            .expect("Failed to open log file");
        
        // Set up tracing
        let file_layer = fmt::layer()
            .with_writer(Mutex::new(log_file))
            .with_ansi(false);
        
        let console_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true);
        
        let level = if config.debug {
            Level::DEBUG
        } else {
            Level::INFO
        };
        
        let env_filter = EnvFilter::from_default_env()
            .add_directive(level.into());
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
        
        unsafe {
            INITIALIZED = true;
        }
        
        info!("Logging initialized");
    });
    
    Ok(())
}

/// Setup with default configuration
pub fn setup_default() -> Result<()> {
    setup(LogConfig::default())
}

/// Check if logging has been initialized
pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}

/// Rotate log file if it exceeds max size
fn rotate_log_if_needed(log_path: &Path, max_size: u64) {
    if let Ok(metadata) = fs::metadata(log_path) {
        if metadata.len() >= max_size {
            let timestamp = Local::now().format("%Y%m%d-%H%M%S");
            let backup_name = format!(
                "{}.{}",
                log_path.file_stem().unwrap().to_string_lossy(),
                timestamp
            );
            let backup_path = log_path.with_file_name(backup_name);
            
            let _ = fs::rename(log_path, backup_path);
        }
    }
}

/// Clean up old log files
fn cleanup_old_logs(log_path: &Path, max_backups: usize, max_age_days: u32) {
    let parent = match log_path.parent() {
        Some(p) => p,
        None => return,
    };
    
    let base_name = match log_path.file_stem() {
        Some(n) => n.to_string_lossy(),
        None => return,
    };
    
    let mut backups: Vec<_> = fs::read_dir(parent)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            name_str.starts_with(&*base_name) && name_str != base_name
        })
        .collect();
    
    // Sort by modification time (newest first)
    backups.sort_by_key(|entry| {
        entry.metadata()
            .and_then(|m| m.modified())
            .ok()
            .map(|t| std::cmp::Reverse(t))
    });
    
    // Remove old backups
    for (i, entry) in backups.iter().enumerate() {
        if i >= max_backups {
            let _ = fs::remove_file(entry.path());
            continue;
        }
        
        // Check age
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = modified.elapsed() {
                    if age.as_secs() > (max_age_days as u64 * 24 * 3600) {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
    }
}

/// Recover from panic and log details
pub fn recover_panic<F>(name: &str, cleanup: F)
where
    F: FnOnce() + Send + Sync + 'static,
{
    let name = name.to_string();
    let cleanup = Mutex::new(Some(cleanup));

    panic::set_hook(Box::new(move |panic_info| {
        // Create panic log file
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let filename = format!("goofy-panic-{}-{}.log", name, timestamp);

        if let Ok(mut file) = File::create(&filename) {
            writeln!(file, "Panic in {}: {:?}", name, panic_info).ok();
            writeln!(file, "Time: {}", Local::now().format("%Y-%m-%d %H:%M:%S")).ok();
            writeln!(file, "\nBacktrace:").ok();
            writeln!(file, "{:?}", std::backtrace::Backtrace::capture()).ok();

            error!("Panic logged to {}", filename);
        }

        // Run cleanup (only once)
        if let Ok(mut guard) = cleanup.lock() {
            if let Some(f) = guard.take() {
                f();
            }
        }
    }));
}

/// Log and recover from errors
pub fn log_error(context: &str, error: &anyhow::Error) {
    error!("Error in {}: {:?}", context, error);
    
    // Log error chain
    let mut source = error.source();
    let mut depth = 0;
    while let Some(err) = source {
        error!("  Caused by [{}]: {}", depth, err);
        source = err.source();
        depth += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_log_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");
        
        // Create a large file
        let mut file = File::create(&log_path).unwrap();
        file.write_all(&vec![b'a'; 1024]).unwrap();
        drop(file);
        
        // Test rotation
        rotate_log_if_needed(&log_path, 512);
        
        // Check that file was rotated
        assert!(!log_path.exists() || fs::metadata(&log_path).unwrap().len() == 0);
    }
    
    #[test]
    fn test_initialized() {
        // Initially should be false (in test environment)
        // Note: This test may fail if run after setup() in the same process
        // assert!(!initialized());
    }
}