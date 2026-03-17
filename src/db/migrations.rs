//! Database migrations

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub up_sql: &'static str,
}

/// All database migrations in order
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        up_sql: r#"
-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    parent_session_id TEXT,
    title TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0 CHECK (message_count >= 0),
    prompt_tokens INTEGER NOT NULL DEFAULT 0 CHECK (prompt_tokens >= 0),
    completion_tokens INTEGER NOT NULL DEFAULT 0 CHECK (completion_tokens >= 0),
    cost REAL NOT NULL DEFAULT 0.0 CHECK (cost >= 0.0),
    updated_at INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    created_at INTEGER NOT NULL,   -- Unix timestamp in milliseconds
    summary_message_id TEXT
);

-- Files table
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    content TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    updated_at INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE,
    UNIQUE(path, session_id, version)
);

CREATE INDEX IF NOT EXISTS idx_files_session_id ON files (session_id);
CREATE INDEX IF NOT EXISTS idx_files_path ON files (path);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    parts TEXT NOT NULL DEFAULT '[]',
    model TEXT,
    created_at INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    updated_at INTEGER NOT NULL,  -- Unix timestamp in milliseconds
    finished_at INTEGER,  -- Unix timestamp in milliseconds
    provider TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages (session_id);

-- Triggers for updated_at
CREATE TRIGGER IF NOT EXISTS update_sessions_updated_at
AFTER UPDATE ON sessions
BEGIN
    UPDATE sessions SET updated_at = (strftime('%s', 'now') * 1000) WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS update_files_updated_at
AFTER UPDATE ON files
BEGIN
    UPDATE files SET updated_at = (strftime('%s', 'now') * 1000) WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS update_messages_updated_at
AFTER UPDATE ON messages
BEGIN
    UPDATE messages SET updated_at = (strftime('%s', 'now') * 1000) WHERE id = new.id;
END;

-- Triggers for message count
CREATE TRIGGER IF NOT EXISTS update_session_message_count_on_insert
AFTER INSERT ON messages
BEGIN
    UPDATE sessions SET message_count = message_count + 1 WHERE id = new.session_id;
END;

CREATE TRIGGER IF NOT EXISTS update_session_message_count_on_delete
AFTER DELETE ON messages
BEGIN
    UPDATE sessions SET message_count = message_count - 1 WHERE id = old.session_id;
END;
        "#,
    },
    Migration {
        version: 2,
        name: "add_created_at_indexes",
        up_sql: r#"
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions (created_at DESC);
        "#,
    },
];