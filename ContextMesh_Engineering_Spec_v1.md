# CONTEXTMESH — REVISED ENGINEERING SPECIFICATION
# Addressing all hard problems from critical review
# Version: 1.0 | Date: 2026-04-26

---

## 1. THE BOOTSTRAP PROBLEM — SOLVED

### Scan Architecture
- **Incremental from Day 1** — No "full scan then optimize"
- **Priority Queue** — Files ranked by: git recency, import centrality, user activity
- **Resume Capability** — scan_jobs table tracks every file's state

### scan_jobs Table (NEW)

```sql
CREATE TABLE scan_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    file_path TEXT NOT NULL,
    status TEXT CHECK(status IN ('pending','scanning','parsed','indexed','failed','skipped')),
    priority INTEGER DEFAULT 0, -- higher = scan first
    started_at INTEGER,
    completed_at INTEGER,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    content_hash TEXT, -- for change detection
    file_size INTEGER,
    last_modified INTEGER,
    UNIQUE(project_id, file_path)
);

CREATE INDEX idx_scan_jobs_status_priority ON scan_jobs(status, priority DESC);
CREATE INDEX idx_scan_jobs_project ON scan_jobs(project_id);
```

### Scan Worker (Rust)
- 4 concurrent workers (configurable)
- Chunked processing: 50 files per transaction
- Progress streaming to frontend via Tauri events
- p95 target: <30s for 500-file codebase (measured, not guessed)

### Resume Logic
```
On app start:
  1. Check scan_jobs for project_id
  2. If any status='scanning' → mark as 'pending', increment retry_count
  3. If retry_count > 3 → mark as 'failed', log error
  4. Queue all 'pending' by priority
  5. Begin incremental scan
```

---

## 2. TREE-SITTER — RUST-NATIVE, NOT WASM

### Architecture Correction
```
Tauri Backend (Rust)
├── tree-sitter crate (native Rust bindings)
│   ├── tree-sitter-javascript
│   ├── tree-sitter-typescript
│   ├── tree-sitter-python
│   ├── tree-sitter-rust
│   └── tree-sitter-go
│
└── Parser Pipeline
    1. Read file bytes
    2. Detect language from extension + shebang
    3. Parse → AST
    4. Extract: symbols, imports, exports, calls
    5. Serialize to JSON
    6. Send via Tauri IPC to TypeScript layer
    7. TypeScript → SQLite graph insert
```

### Why Rust-Native
- First-class tree-sitter Rust crate (not WASM shim)
- Zero memory boundary issues
- Shared memory with file watcher (same process)
- 10-50x faster than WASM in WebView

### Language Detection Map
```rust
fn detect_language(path: &Path) -> Option<Language> {
    match path.extension()?.to_str()? {
        "js" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::TSX),
        "py" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        _ => detect_from_shebang(path), // fallback
    }
}
```

---

## 3. CONTEXT PACK — FULLY SPECIFIED

### Pack Format (Claude Code — First Agent)

```typescript
interface ContextPack {
  version: "1.0.0";
  pack_id: string; // uuid
  generated_at: ISO8601;
  project: {
    id: string;
    name: string;
    tech_stack: string[];
    conventions: Convention[]; // from conventions table
  };

  // The actual context — structured for Claude Code
  context: {
    // System-level instructions
    system_prompt: string;

    // Project memory (what the agent MUST know)
    project_memory: {
      architecture: string; // ADR summary
      key_decisions: Decision[];
      active_work: string; // current branch, recent commits
      conventions: string; // formatted rules
    };

    // Relevant code (semantic search results)
    relevant_entities: EntitySnippet[];

    // Current task context
    task_context: {
      description: string;
      target_files: string[];
      related_tests: string[];
      dependencies: string[];
    };

    // Confidence manifest
    assumptions: Assumption[];

    // Token budget
    token_budget: {
      estimated_total: number;
      system_prompt: number;
      project_memory: number;
      relevant_entities: number;
      task_context: number;
      reserved_for_response: number;
    };
  };
}

interface EntitySnippet {
  id: string;
  type: "file" | "function" | "class" | "interface";
  name: string;
  file_path: string;
  line_start: number;
  line_end: number;
  content: string; // actual code snippet
  relevance_score: number; // 0-1
  why_relevant: string; // human-readable reason
  relationships: {
    imports: string[];
    called_by: string[];
    calls: string[];
  };
}

interface Assumption {
  id: string;
  statement: string; // "Assumes UserAuthService uses JWT"
  confidence: "high" | "medium" | "low";
  verify_by: string; // action to verify
  source: string; // where this assumption came from
}

interface Decision {
  id: string;
  title: string;
  context: string;
  decision: string;
  consequences: string[];
  linked_entities: string[];
  date: ISO8601;
}
```

### Semantic Search Strategy (Phase 1: BM25 + Graph Proximity)

**Why not embeddings yet:**
- SQLite-vec adds native dependency complexity
- BM25 over entity names + signatures + comments is 80% effective
- Graph proximity (2-hop neighbors) captures relationships embeddings miss
- **Future:** Add sqlite-vec when needed, schema is forward-compatible

**Implementation:**
```sql
-- Full-text search on entity content
CREATE VIRTUAL TABLE entity_fts USING fts5(
    name,
    signature,
    file_path,
    content='entities',
    content_rowid='id'
);

-- Search query
SELECT e.*, rank
FROM entity_fts ef
JOIN entities e ON ef.rowid = e.id
WHERE entity_fts MATCH ?
ORDER BY rank;

-- Then expand via graph (2-hop)
WITH RECURSIVE relevant AS (
    -- Seed: BM25 results
    SELECT id, 1.0 as score, 0 as depth FROM entity_fts WHERE ...
    UNION ALL
    -- Expand: neighbors get 0.7^depth score decay
    SELECT r.target_id, rel.score * 0.7, depth + 1
    FROM relevant rel
    JOIN relationships r ON rel.id = r.source_id
    WHERE depth < 2
)
SELECT * FROM relevant ORDER BY score DESC;
```

### Token Estimation
```typescript
function estimateTokens(pack: ContextPack): number {
  // tiktoken or approximate: 1 token ≈ 4 chars for code
  const systemTokens = pack.context.system_prompt.length / 4;
  const memoryTokens = JSON.stringify(pack.context.project_memory).length / 4;
  const entityTokens = pack.context.relevant_entities.reduce(
    (sum, e) => sum + e.content.length / 4 + 50, // 50 for metadata
    0
  );
  const taskTokens = JSON.stringify(pack.context.task_context).length / 4;

  return Math.ceil(systemTokens + memoryTokens + entityTokens + taskTokens);
}
```

---

## 4. MCP SERVER — FULLY SCOPED (P0, but scoped)

### MCP Tools API Surface (7 tools)

```typescript
// Tool 1: Get context pack for current task
interface GetContextPackInput {
  task_description: string;
  target_files?: string[]; // if known
  agent_type: "claude" | "cursor" | "copilot" | "generic";
  max_tokens?: number; // budget constraint
  include_tests?: boolean;
  include_dependencies?: boolean;
}

interface GetContextPackOutput {
  pack: ContextPack;
  pack_id: string;
  estimated_tokens: number;
  relevant_entity_count: number;
  assumptions_count: number;
}

// Tool 2: Query knowledge graph
interface QueryGraphInput {
  query_type: "entity" | "relationship" | "path" | "impact";
  entity_id?: string;
  entity_name?: string;
  file_path?: string;
  relationship_type?: string;
  depth?: number; // for path queries
}

interface QueryGraphOutput {
  results: (Entity | Relationship | Path)[];
  result_count: number;
  query_time_ms: number;
}

// Tool 3: Record decision (ADR)
interface RecordDecisionInput {
  title: string;
  context: string;
  decision: string;
  alternatives_considered: string[];
  consequences: string[];
  linked_entity_ids?: string[];
  linked_file_paths?: string[];
  author?: string;
}

interface RecordDecisionOutput {
  decision_id: string;
  inserted: boolean;
  linked_entities_updated: number;
}

// Tool 4: Update graph from change
interface RecordChangeInput {
  file_path: string;
  change_type: "created" | "modified" | "deleted" | "renamed";
  diff_summary?: string;
  commit_hash?: string;
  author?: string;
  decision_note?: string;
}

interface RecordChangeOutput {
  event_id: string;
  entities_affected: number;
  relationships_updated: number;
}

// Tool 5: Get codebase health
interface GetHealthInput {
  scope: "project" | "file" | "entity";
  file_path?: string;
  entity_id?: string;
}

interface GetHealthOutput {
  health_score: number; // 0-100
  metrics: {
    complexity: number;
    coupling: number;
    churn_rate: number;
    test_coverage?: number; // null if no test data
  };
  hotspots: Hotspot[];
  issues: Issue[];
}

// Tool 6: Get conventions
interface GetConventionsInput {
  category?: "naming" | "error_handling" | "testing" | "architecture" | "all";
  file_path?: string; // context-specific conventions
}

interface GetConventionsOutput {
  conventions: Convention[];
  auto_detected_count: number;
  user_defined_count: number;
  confidence_distribution: { high: number; medium: number; low: number };
}

// Tool 7: Sync agent state
interface SyncAgentStateInput {
  agent_type: string;
  agent_session_id?: string; // for resumption
  current_task?: string;
  files_open?: string[];
  context_version?: string;
}

interface SyncAgentStateOutput {
  session_id: string;
  context_freshness: { [file_path: string]: "fresh" | "stale" | "unknown" };
  recommended_sync_actions: string[];
  pack_version: string;
}
```

### MCP Server Architecture
```
ContextMesh App
├── MCP Server (stdio transport)
│   ├── Listens on stdin/stdout
│   ├── Maintains session state in SQLite
│   ├── Handles tool calls via JSON-RPC
│   └── Auto-starts when agent connects
│
└── Session Lifecycle
    1. Agent (Claude Code) starts, sees ContextMesh in config
    2. Spawns: `contextmesh mcp-server --project-id=xxx`
    3. Server loads project graph from SQLite
    4. Agent calls tools as needed
    5. On disconnect: session marked idle, not deleted
    6. On reconnect: session resumed via session_id
```

### Session Resumption Protocol
```sql
-- Extended agent_sessions schema
CREATE TABLE agent_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    agent_type TEXT,
    status TEXT CHECK(status IN ('active','idle','disconnected','error')),
    context_pack_version INTEGER,
    current_task TEXT,
    open_files TEXT, -- JSON array
    last_context_pack_id TEXT,
    created_at INTEGER,
    last_active_at INTEGER,
    resumed_at INTEGER, -- NEW
    resume_count INTEGER DEFAULT 0 -- NEW
);
```

---

## 5. DECISIONS TABLE — FIRST-CLASS CITIZEN

```sql
CREATE TABLE decisions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    status TEXT CHECK(status IN ('proposed','accepted','deprecated','superseded')),

    -- ADR format
    context TEXT NOT NULL, -- why we needed this decision
    decision TEXT NOT NULL, -- what we decided
    consequences TEXT, -- JSON array of strings
    alternatives_considered TEXT, -- JSON array

    -- Linkage
    linked_entities TEXT, -- JSON array of entity IDs
    linked_events TEXT, -- JSON array of event IDs

    -- Metadata
    author TEXT,
    created_at INTEGER,
    updated_at INTEGER,
    superseded_by TEXT, -- references another decision.id

    -- For temporal validity
    valid_from INTEGER,
    valid_until INTEGER -- null if still valid
);

CREATE INDEX idx_decisions_project ON decisions(project_id);
CREATE INDEX idx_decisions_status ON decisions(status);
```

---

## 6. REVISED PHASE ORDER

### Phase 0: Foundation (Week 1)
1. Tauri scaffold + React UI shell
2. SQLite schema (all tables, including scan_jobs, decisions)
3. File watcher (Rust native)
4. **Tree-sitter parser (Rust native)** — JS/TS only first
5. Basic graph population

### Phase 1: MCP Proof-of-Life (Week 2)
1. MCP server with 3 tools: get_context_pack, query_graph, record_decision
2. Claude Code integration test on real project
3. Validate context quality with developer feedback
4. **STOP HERE** — if context quality is poor, fix before proceeding

### Phase 2: Agent Router + Cost Control (Week 3)
1. API key management
2. Cost estimation + tracking
3. Task classification (simple/complex/critical)
4. Model assignment logic

### Phase 3: Visualization (Week 4)
1. Only AFTER context quality is validated
2. Module dependency graph (D3.js)
3. Health dashboard (real metrics, no coverage yet)
4. Hotspot detection

### Phase 4: Multi-Agent + Polish (Week 5-6)
1. Cursor adapter
2. Copilot adapter
3. Convention auto-detection
4. UI polish

---

## 7. HEALTH SCORE — REALISTIC V1

```typescript
interface HealthMetrics {
  // V1 (no test runner integration)
  complexity: number;     // cyclomatic from AST
  coupling: number;       // afferent/efferent coupling
  churn_rate: number;     // git log frequency
  cohesion: number;       // related functions stay together

  // V2 (future)
  // test_coverage: number; // requires test runner integration
  // type_safety: number;  // requires TypeScript compiler API
}

function calculateHealthScore(metrics: HealthMetrics): number {
  // Weighted composite
  const weights = {
    complexity: 0.25,
    coupling: 0.25,
    churn_rate: 0.25,
    cohesion: 0.25,
  };

  // Normalize each to 0-100 (lower raw = higher score)
  const normalized = {
    complexity: Math.max(0, 100 - metrics.complexity * 5),
    coupling: Math.max(0, 100 - metrics.coupling * 10),
    churn_rate: Math.max(0, 100 - metrics.churn_rate * 20),
    cohesion: metrics.cohesion * 100,
  };

  return Math.round(
    normalized.complexity * weights.complexity +
    normalized.coupling * weights.coupling +
    normalized.churn_rate * weights.churn_rate +
    normalized.cohesion * weights.cohesion
  );
}
```

---

## 8. COMPETITIVE MOAT — INDIAN DEVELOPER WORKFLOW

### Differentiation from Pieces, Zed, Claude Code Memory:

| Feature | ContextMesh | Others |
|---------|-------------|--------|
| Cost optimization | Native — routes free/paid by task | No cost awareness |
| Free tier orchestration | Designed for tier-hopping | Assumes single subscription |
| Local-first | 100% offline capable | Cloud-dependent |
| Agent-agnostic | Universal adapter | Tool-specific |
| Visual graph | Interactive D3.js exploration | List views only |
| Decision provenance | First-class ADR tracking | Git history only |

### Indian Market Specifics:
- **Jio/limited bandwidth**: Offline-first is essential
- **Multiple free accounts**: Orchestrate Claude free tier + Cursor free + Copilot student
- **Cost sensitivity**: Per-rupee optimization, not just convenience
- **Team coordination**: Distributed teams, async workflows

---

## 9. CORRECTED FILE STRUCTURE

```
contextmesh/
├── src/
│   ├── main/                          # Tauri Rust backend
│   │   ├── main.rs
│   │   ├── commands/
│   │   │   ├── fs.rs                 # File operations
│   │   │   ├── graph.rs              # Graph queries
│   │   │   └── agent.rs              # Agent session mgmt
│   │   ├── parser/                   # Tree-sitter (RUST)
│   │   │   ├── mod.rs
│   │   │   ├── languages.rs          # Language detection
│   │   │   ├── extractors/           # Per-language extractors
│   │   │   │   ├── javascript.rs
│   │   │   │   ├── typescript.rs
│   │   │   │   └── python.rs
│   │   │   └── output.rs             # JSON serialization
│   │   ├── watcher/
│   │   │   ├── mod.rs
│   │   │   └── debouncer.rs          # 300ms debounce
│   │   └── mcp_server/               # MCP implementation
│   │       ├── mod.rs
│   │       ├── transport.rs          # stdio JSON-RPC
│   │       ├── tools/                # 7 tool implementations
│   │       │   ├── get_context_pack.rs
│   │       │   ├── query_graph.rs
│   │       │   ├── record_decision.rs
│   │       │   ├── record_change.rs
│   │       │   ├── get_health.rs
│   │       │   ├── get_conventions.rs
│   │       │   └── sync_agent_state.rs
│   │       └── session.rs            # Session lifecycle
│   │
│   ├── core/                          # TypeScript business logic
│   │   ├── engine/
│   │   │   ├── graph.ts              # Graph CRUD
│   │   │   ├── health.ts             # Health scoring
│   │   │   └── conventions.ts        # Convention detection
│   │   ├── ai/
│   │   │   ├── router.ts             # Cost-aware routing
│   │   │   ├── context-pack.ts       # Pack generation
│   │   │   ├── cost-tracker.ts       # Usage analytics
│   │   │   └── token-estimator.ts    # Token counting
│   │   ├── search/
│   │   │   ├── bm25.ts               # Full-text search
│   │   │   └── graph-expand.ts       # 2-hop expansion
│   │   └── adapters/
│   │       ├── cursor.ts
│   │       ├── claude.ts
│   │       └── copilot.ts
│   │
│   ├── db/
│   │   ├── schema.ts                 # All table definitions
│   │   ├── migrations/
│   │   │   ├── 001_initial.sql
│   │   │   └── 002_decisions.sql
│   │   └── queries/
│   │       ├── entities.ts
│   │       ├── relationships.ts
│   │       └── search.ts
│   │
│   └── ui/                            # React frontend
│       ├── App.tsx
│       ├── components/
│       │   ├── dashboard/
│       │   │   ├── HealthScore.tsx
│       │   │   ├── Hotspots.tsx
│       │   │   └── RecentChanges.tsx
│       │   ├── graph/
│       │   │   ├── ModuleGraph.tsx     # D3.js force layout
│       │   │   ├── CallGraph.tsx
│       │   │   └── GraphControls.tsx
│       │   ├── agents/
│       │   │   ├── AgentPanel.tsx
│       │   │   ├── CostEstimator.tsx
│       │   │   └── TaskRouter.tsx
│       │   └── settings/
│       │       ├── ApiKeys.tsx
│       │       └── ProjectPicker.tsx
│       ├── hooks/
│       │   ├── useGraph.ts
│       │   ├── useHealth.ts
│       │   └── useScanProgress.ts
│       └── stores/
│           ├── projectStore.ts
│           ├── graphStore.ts
│           └── agentStore.ts
│
├── src-tauri/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── package.json
└── README.md
```

---

## 10. ACCEPTANCE CRITERIA BEFORE NEXT PHASE

### Phase 0 Gate:
- [ ] Scan 500-file JS/TS codebase in <30s on M1 Mac
- [ ] Resume interrupted scan without data loss
- [ ] Tree-sitter Rust parser extracts >95% of imports/exports correctly
- [ ] File watcher detects changes within 500ms

### Phase 1 Gate (MCP Proof-of-Life):
- [ ] Claude Code can call get_context_pack and receive valid pack
- [ ] Pack includes relevant entities for "refactor auth middleware" task
- [ ] Query graph returns 2-hop neighbors in <100ms
- [ ] Record decision persists and links to entities

### Phase 2 Gate (Router):
- [ ] Cost estimation within 20% of actual
- [ ] Simple tasks routed to free/cheap models
- [ ] Complex tasks routed to premium models
- [ ] Budget cap enforcement works

### Phase 3 Gate (Visualization):
- [ ] Graph renders 1000 nodes at 60fps
- [ ] Health score correlates with actual code quality (developer survey)
- [ ] Hotspot detection matches git blame data

---

## 11. COMPLETE DATABASE SCHEMA

```sql
-- Projects
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    tech_stack TEXT, -- JSON array
    created_at INTEGER,
    last_scanned_at INTEGER,
    health_score REAL DEFAULT 0
);

-- Scan jobs (incremental indexing)
CREATE TABLE scan_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
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

-- Core entities
CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
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
CREATE TABLE relationships (
    id TEXT PRIMARY KEY,
    source_id TEXT REFERENCES entities(id),
    target_id TEXT REFERENCES entities(id),
    type TEXT CHECK(type IN ('imports','extends','implements','calls','exports','depends_on','contains','references')),
    strength REAL DEFAULT 1.0,
    created_at INTEGER,
    metadata TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE entity_fts USING fts5(
    name,
    signature,
    file_path,
    content='entities',
    content_rowid='id'
);

-- Temporal events
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    entity_id TEXT REFERENCES entities(id),
    event_type TEXT CHECK(event_type IN ('created','modified','deleted','renamed','refactored')),
    commit_hash TEXT,
    author TEXT,
    diff_summary TEXT,
    decision_note TEXT,
    timestamp INTEGER,
    parent_event_id TEXT
);

-- Decisions (ADRs)
CREATE TABLE decisions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    status TEXT CHECK(status IN ('proposed','accepted','deprecated','superseded')),
    context TEXT NOT NULL,
    decision TEXT NOT NULL,
    consequences TEXT,
    alternatives_considered TEXT,
    linked_entities TEXT,
    linked_events TEXT,
    author TEXT,
    created_at INTEGER,
    updated_at INTEGER,
    superseded_by TEXT,
    valid_from INTEGER,
    valid_until INTEGER
);

-- Agent sessions
CREATE TABLE agent_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    agent_type TEXT,
    status TEXT CHECK(status IN ('active','idle','disconnected','error')),
    context_pack_version INTEGER,
    current_task TEXT,
    open_files TEXT, -- JSON array
    last_context_pack_id TEXT,
    created_at INTEGER,
    last_active_at INTEGER,
    resumed_at INTEGER,
    resume_count INTEGER DEFAULT 0
);

-- Context packs
CREATE TABLE context_packs (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES agent_sessions(id),
    pack_data TEXT, -- JSON: structured context
    entities_included TEXT, -- JSON array of entity IDs
    assumptions TEXT, -- JSON array
    estimated_tokens INTEGER,
    actual_tokens INTEGER,
    cost_usd REAL,
    created_at INTEGER
);

-- Cost tracking
CREATE TABLE cost_log (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    operation TEXT,
    provider TEXT, -- 'openai','anthropic','google','local'
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost_usd REAL,
    task_type TEXT, -- 'simple','complex','critical'
    timestamp INTEGER
);

-- Conventions & rules
CREATE TABLE conventions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    category TEXT, -- 'naming','error_handling','testing','architecture'
    rule TEXT,
    confidence REAL, -- 0-1, how consistently applied
    evidence TEXT, -- JSON: examples found
    is_auto_detected BOOLEAN DEFAULT 1,
    is_user_defined BOOLEAN DEFAULT 0,
    created_at INTEGER,
    updated_at INTEGER
);
```

---

## SUMMARY: THREE HARD DECISIONS RESOLVED

1. **Tree-sitter in Rust** — Native Rust crate, not WASM. Parser pipeline runs in Tauri backend, outputs JSON to TypeScript layer.

2. **Context pack JSON schema** — Fully specified for Claude Code first. Includes: system_prompt, project_memory, relevant_entities (with BM25+graph search), task_context, assumptions, token_budget. Semantic search uses BM25 + 2-hop graph expansion (no embeddings in V1).

3. **MCP 7-tool API** — get_context_pack, query_graph, record_decision, record_change, get_health, get_conventions, sync_agent_state. stdio transport, session resumption via session_id, SQLite-backed state.

This specification is now ready for implementation.
