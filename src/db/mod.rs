//! Database module for persistent storage
//!
//! This module provides a comprehensive database layer matching Crush's
//! implementation, with support for sessions, messages, and files.

mod connect;
mod models;
mod queries;
mod migrations;
mod database;

pub use connect::{connect, DatabaseConfig};
pub use models::{Session, Message, File};
pub use queries::{Queries, TransactionQueries, DatabaseOperations};
pub use database::{Database, SessionRow};

// Re-export common types
pub use rusqlite::{Connection, Transaction, Error as SqliteError};