//! Cross-platform shell execution module
//!
//! This module provides shell execution capabilities with:
//! - Cross-platform support (Unix/Windows)
//! - Persistent shell state management
//! - Command blocking for security
//! - Environment variable management
//! - Working directory tracking

mod shell;
pub mod persistent;
mod coreutils;
mod parser;

pub use shell::{Shell, ShellOptions, ShellType, CommandBlocker, ExitStatus};
pub use persistent::{PersistentShell, get_persistent_shell, execute_in_persistent, get_persistent_working_dir, set_persistent_working_dir};
pub use coreutils::CoreUtils;
pub use parser::CommandParser;