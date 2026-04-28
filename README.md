# Knot

A local-first, AI-agnostic context management desktop app for agentic coding workflows.

## Architecture

Knot is built with:
- **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS
- **Backend**: Tauri v2 (Rust) with native Tree-sitter parsers
- **Database**: SQLite with FTS5 for full-text search
- **File Watching**: Native Rust with 300ms debounce

## Phase 0 Deliverables

### ✅ Tauri + React Shell
- Tauri v2 + React 19 + TypeScript + Vite
- Tailwind CSS wired up
- Zustand stores (projectStore, graphStore, agentStore)
- TanStack Query configured
- Basic App.tsx with tabs: Dashboard | Graph | Agents | Cost | Settings
- Window title: "Knot"

### ✅ SQLite Schema
- All 11 tables created on first launch
- FTS5 with triggers for entity sync (no content= parameter)
- session_files table (separate from agent_sessions)
- All indexes from spec created
- Resume logic on app start

### ✅ File Watcher (Rust)
- notify crate with 300ms debounce
- Recursive watching with .gitignore respect
- Emits "knot_file_changed" event
- Watcher starts when project is loaded

### ✅ Tree-sitter Parser (Rust)
- Native Rust bindings (not WASM)
- Supports JS, TS, TSX, Python
- Per-language extractors
- Tauri command: parse_file
- Tauri command: parse_project

### ✅ Scan Job System
- Priority queue (git-recent: 100, src/: 50, other: 10)
- Chunked processing (50 files per transaction)
- Resume logic for interrupted scans
- Progress tracking

## File Structure

```
knot/
├── src/
│   └── ui/                    # React frontend
│       ├── src/
│       │   ├── main.tsx
│       │   ├── App.tsx
│       │   ├── stores/
│       │   │   ├── projectStore.ts
│       │   │   ├── graphStore.ts
│       │   │   └── agentStore.ts
│       │   └── hooks/
│       ├── package.json
│       └── ...
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # App entry point
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── projects.rs    # Project CRUD
│   │   │   └── graph.rs       # Graph queries & scan jobs
│   │   ├── parser/
│   │   │   ├── mod.rs         # Language detection
│   │   │   ├── languages.rs   # Tree-sitter language setup
│   │   │   └── extractors/
│   │   │       ├── mod.rs
│   │   │       ├── javascript.rs
│   │   │       ├── typescript.rs
│   │   │       └── python.rs
│   │   └── watcher/
│   │       ├── mod.rs         # File watcher
│   │       └── debouncer.rs   # 300ms debounce logic
│   ├── migrations/
│   │   └── 001_initial.sql    # Complete schema
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
└── README.md
```

## Database Schema

### Tables
1. **projects** - Project metadata
2. **scan_jobs** - Incremental file scanning queue
3. **entities** - Code entities (functions, classes, etc.)
4. **relationships** - Entity relationships
5. **entity_fts** - FTS5 virtual table for search
6. **events** - Temporal events (file changes)
7. **decisions** - ADR (Architecture Decision Records)
8. **agent_sessions** - AI agent session state
9. **session_files** - Files open in agent sessions
10. **context_packs** - Generated context for agents
11. **cost_log** - AI usage tracking
12. **conventions** - Code conventions & rules

### FTS5 Triggers
- entity_fts_insert: Sync on INSERT
- entity_fts_update: Sync on UPDATE
- entity_fts_delete: Sync on DELETE

## Tauri Commands

### Projects
- `create_project(request: CreateProjectRequest) -> Project`
- `get_projects() -> Vec<Project>`
- `load_project(project_id: String) -> Project`

### Graph
- `parse_file(file_path: String) -> ParsedFile`
- `parse_project(project_id: String) -> String`
- `get_scan_progress(project_id: String) -> ScanProgress`

## Development

### Prerequisites
- Rust toolchain
- Node.js 20+
- npm or pnpm

### Setup

```bash
# Install UI dependencies
cd src/ui
npm install

# Install Rust dependencies (in root)
cd ../..
cd src-tauri
cargo build

# Run dev server
npm run tauri dev
```

### Building

```bash
npm run tauri build
```

## Critical Rules

1. **Tree-sitter runs in Rust only** - Never in TypeScript/WebView
2. **SQLite access via Rust only** - TypeScript reads via Tauri commands
3. **No WASM bindings for parsing** - Native Rust only
4. **All Tauri commands return Result<T, String>** - Explicit error handling
5. **No unwrap() in production paths** - Use ? operator

## License

MIT
