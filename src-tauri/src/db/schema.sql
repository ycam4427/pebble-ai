-- Local AI Computer Assistant — local SQLite schema.
-- Everything stays on-device. No cloud, ever.

CREATE TABLE IF NOT EXISTS conversations (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,          -- user | assistant | system
    content         TEXT NOT NULL,
    actions_json    TEXT,                   -- snapshot of proposed actions
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, created_at);

CREATE TABLE IF NOT EXISTS plans (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT,
    summary         TEXT NOT NULL,
    max_tier        INTEGER NOT NULL,
    status          TEXT NOT NULL,          -- pending | approved | rejected | executed
    created_at      TEXT NOT NULL
);

-- Every executed operation is logged here so it can be undone and audited.
CREATE TABLE IF NOT EXISTS action_log (
    id              TEXT PRIMARY KEY,
    plan_id         TEXT,
    op_index        INTEGER NOT NULL,
    kind            TEXT NOT NULL,          -- move | rename | delete | execute
    tier            INTEGER NOT NULL,
    source          TEXT NOT NULL,
    destination     TEXT,
    status          TEXT NOT NULL,          -- executed | failed | undone
    undo_data_json  TEXT,                   -- how to reverse this operation
    error           TEXT,
    executed_at     TEXT NOT NULL,
    undone_at       TEXT
);
CREATE INDEX IF NOT EXISTS idx_action_log_time ON action_log(executed_at DESC);

-- The AI Trash: nothing is ever permanently deleted by a command.
CREATE TABLE IF NOT EXISTS trash_items (
    id            TEXT PRIMARY KEY,
    original_path TEXT NOT NULL,
    trash_path    TEXT NOT NULL,
    name          TEXT NOT NULL,
    size          INTEGER NOT NULL,
    is_dir        INTEGER NOT NULL,
    deleted_at    TEXT NOT NULL,
    expires_at    TEXT NOT NULL,
    restored_at   TEXT
);
CREATE INDEX IF NOT EXISTS idx_trash_expires ON trash_items(expires_at);

-- Key/value user + folder preferences (JSON-encoded values).
CREATE TABLE IF NOT EXISTS preferences (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Frequently used / remembered locations.
CREATE TABLE IF NOT EXISTS locations (
    id         TEXT PRIMARY KEY,
    path       TEXT NOT NULL UNIQUE,
    label      TEXT,
    kind       TEXT,
    use_count  INTEGER NOT NULL DEFAULT 0,
    last_used  TEXT
);

-- Pebble's opt-in long-term memory about the user (he curates what's stored).
CREATE TABLE IF NOT EXISTS user_memory (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,          -- fact | event
    content     TEXT NOT NULL,
    event_date  TEXT,                   -- YYYY-MM-DD for dated events, else NULL
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_user_memory_date ON user_memory(event_date);
