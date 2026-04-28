-- Knot Database Schema - Phase 0
-- Complete schema with all 11 tables

-- Projects
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    tech_stack TEXT, -- JSON array
    created_at INTEGER,
    last_scanned_at INTEGER,
    health_score REAL DEFAULT 0
);

-- Scan jobs (incremental indexing)
CREATE TABLE IF NOT EXISTS scan_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    status TEXT CHECK(status IN ('pending','scanning','parsed','indexed','failed','skipped')),
    priority INTEGER DEFAULT 0,
    started_at INTEGER,
    completed_at INTEGER,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    content_hash TEXT,
    file_size INTEGER,
    last_modified INTEGER,
    UNIQUE(project_id, file_path)
);

CREATE INDEX IF NOT EXISTS idx_scan_jobs_status_priority ON scan_jobs(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_scan_jobs_project ON scan_jobs(project_id);

-- Core entities
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    type TEXT CHECK(type IN ('file','function','class','interface','type','variable','module','api_endpoint')),
    name TEXT NOT NULL,
    file_path TEXT,
    line_start INTEGER,
    line_end INTEGER,
    signature TEXT,
    is_public BOOLEAN DEFAULT 0,
    complexity_score REAL,
    created_at INTEGER,
    modified_at INTEGER,
    content_hash TEXT,
    metadata TEXT -- JSON: language, framework-specific data
);

-- Relationships
CREATE TABLE IF NOT EXISTS relationships (
    id TEXT PRIMARY KEY,
    source_id TEXT REFERENCES entities(id) ON DELETE CASCADE,
    target_id TEXT REFERENCES entities(id) ON DELETE CASCADE,
    type TEXT CHECK(type IN ('imports','extends','implements','calls','exports','depends_on','contains','references')),
    strength REAL DEFAULT 1.0,
    created_at INTEGER,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_id);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id);
CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(type);

-- Full-text search - store data directly (no content= parameter to avoid rowid mismatch)
CREATE VIRTUAL TABLE IF NOT EXISTS entity_fts USING fts5(
    name,
    signature,
    file_path
);

-- Triggers to sync entities with FTS5
CREATE TRIGGER IF NOT EXISTS entity_fts_insert AFTER INSERT ON entities
BEGIN
    INSERT INTO entity_fts(name, signature, file_path)
    VALUES (NEW.name, NEW.signature, NEW.file_path);
END;

CREATE TRIGGER IF NOT EXISTS entity_fts_update AFTER UPDATE ON entities
BEGIN
    DELETE FROM entity_fts WHERE rowid = (SELECT rowid FROM entity_fts WHERE name = OLD.name AND file_path = OLD.file_path LIMIT 1);
    INSERT INTO entity_fts(name, signature, file_path)
    VALUES (NEW.name, NEW.signature, NEW.file_path);
END;

CREATE TRIGGER IF NOT EXISTS entity_fts_delete AFTER DELETE ON entities
BEGIN
    DELETE FROM entity_fts WHERE name = OLD.name AND file_path = OLD.file_path;
END;

-- Temporal events
CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    entity_id TEXT REFERENCES entities(id) ON DELETE CASCADE,
    event_type TEXT CHECK(event_type IN ('created','modified','deleted','renamed','refactored')),
    commit_hash TEXT,
    author TEXT,
    diff_summary TEXT,
    decision_note TEXT,
    timestamp INTEGER,
    parent_event_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_project ON events(project_id);
CREATE INDEX IF NOT EXISTS idx_events_entity ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);

-- Decisions (ADRs)
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT CHECK(status IN ('proposed','accepted','deprecated','superseded')),
    context TEXT NOT NULL,
    decision TEXT NOT NULL,
    consequences TEXT, -- JSON array
    alternatives_considered TEXT, -- JSON array
    linked_entities TEXT, -- JSON array of entity IDs
    linked_events TEXT, -- JSON array of event IDs
    author TEXT,
    created_at INTEGER,
    updated_at INTEGER,
    superseded_by TEXT,
    valid_from INTEGER,
    valid_until INTEGER
);

CREATE INDEX IF NOT EXISTS idx_decisions_project ON decisions(project_id);
CREATE INDEX IF NOT EXISTS idx_decisions_status ON decisions(status);

-- Agent sessions
CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    agent_type TEXT,
    status TEXT CHECK(status IN ('active','idle','disconnected','error')),
    context_pack_version INTEGER,
    current_task TEXT,
    last_context_pack_id TEXT,
    created_at INTEGER,
    last_active_at INTEGER,
    resumed_at INTEGER,
    resume_count INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_project ON agent_sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);

-- Session files (separate from agent_sessions - no JSON array for open_files)
CREATE TABLE IF NOT EXISTS session_files (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES agent_sessions(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    opened_at INTEGER,
    UNIQUE(session_id, file_path)
);

CREATE INDEX IF NOT EXISTS idx_session_files_session ON session_files(session_id);
CREATE INDEX IF NOT EXISTS idx_session_files_path ON session_files(file_path);

-- Context packs
CREATE TABLE IF NOT EXISTS context_packs (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES agent_sessions(id) ON DELETE CASCADE,
    pack_data TEXT, -- JSON: structured context
    entities_included TEXT, -- JSON array of entity IDs
    assumptions TEXT, -- JSON array
    estimated_tokens INTEGER,
    actual_tokens INTEGER,
    cost_usd REAL,
    created_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_context_packs_session ON context_packs(session_id);

-- Cost tracking
CREATE TABLE IF NOT EXISTS cost_log (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    operation TEXT,
    provider TEXT, -- 'openai','anthropic','google','local'
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost_usd REAL,
    task_type TEXT, -- 'simple','complex','critical'
    timestamp INTEGER
);

CREATE INDEX IF NOT EXISTS idx_cost_log_project ON cost_log(project_id);
CREATE INDEX IF NOT EXISTS idx_cost_log_timestamp ON cost_log(timestamp);

-- Conventions & rules
CREATE TABLE IF NOT EXISTS conventions (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    category TEXT, -- 'naming','error_handling','testing','architecture'
    rule TEXT,
    confidence REAL, -- 0-1
    evidence TEXT, -- JSON: examples found
    is_auto_detected BOOLEAN DEFAULT 1,
    is_user_defined BOOLEAN DEFAULT 0,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_conventions_project ON conventions(project_id);
CREATE INDEX IF NOT EXISTS idx_conventions_category ON conventions(category);
