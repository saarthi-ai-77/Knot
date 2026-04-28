use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub tech_stack: Option<String>,
    pub created_at: i64,
    pub last_scanned_at: Option<i64>,
    pub health_score: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub root_path: String,
    pub tech_stack: Option<Vec<String>>,
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    request: CreateProjectRequest,
) -> Result<Project, String> {
    let db = state.db.lock().await;
    
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().timestamp();
    let tech_stack_json = request.tech_stack.map(|ts| serde_json::to_string(&ts).unwrap_or_default());
    
    db.execute(
        "INSERT INTO projects (id, name, root_path, tech_stack, created_at, health_score) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        [
            &id,
            &request.name,
            &request.root_path,
            tech_stack_json.as_deref().unwrap_or_default(),
            &created_at.to_string(),
            "0",
        ],
    ).map_err(|e| format!("Failed to create project: {}", e))?;
    
    Ok(Project {
        id,
        name: request.name,
        root_path: request.root_path,
        tech_stack: tech_stack_json,
        created_at,
        last_scanned_at: None,
        health_score: 0.0,
    })
}

#[tauri::command]
pub async fn get_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT id, name, root_path, tech_stack, created_at, last_scanned_at, health_score 
         FROM projects ORDER BY created_at DESC"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let projects = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            root_path: row.get(2)?,
            tech_stack: row.get(3)?,
            created_at: row.get(4)?,
            last_scanned_at: row.get(5)?,
            health_score: row.get(6)?,
        })
    }).map_err(|e| format!("Failed to query projects: {}", e))?;
    
    projects.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect projects: {}", e))
}

#[tauri::command]
pub async fn load_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Project, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT id, name, root_path, tech_stack, created_at, last_scanned_at, health_score 
         FROM projects WHERE id = ?1"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let project = stmt.query_row([&project_id], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            root_path: row.get(2)?,
            tech_stack: row.get(3)?,
            created_at: row.get(4)?,
            last_scanned_at: row.get(5)?,
            health_score: row.get(6)?,
        })
    }).map_err(|e| format!("Failed to load project: {}", e))?;
    
    Ok(project)
}

// Dashboard stats commands
#[tauri::command]
pub async fn get_entity_count(state: State<'_, AppState>, project_id: String) -> Result<i64, String> {
    let db = state.db.lock().await;
    
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM entities WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get entity count: {}", e))?;
    
    Ok(count)
}

#[tauri::command]
pub async fn get_relationship_count(state: State<'_, AppState>, project_id: String) -> Result<i64, String> {
    let db = state.db.lock().await;
    
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM relationships r
         JOIN entities e ON r.source_id = e.id
         WHERE e.project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get relationship count: {}", e))?;
    
    Ok(count)
}

#[tauri::command]
pub async fn get_decision_count(state: State<'_, AppState>, project_id: String) -> Result<i64, String> {
    let db = state.db.lock().await;
    
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM decisions WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get decision count: {}", e))?;
    
    Ok(count)
}

#[tauri::command]
pub async fn get_session_count(state: State<'_, AppState>, project_id: String) -> Result<i64, String> {
    let db = state.db.lock().await;
    
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get session count: {}", e))?;
    
    Ok(count)
}

// Event row for activity feed
#[derive(Debug, Serialize)]
pub struct EventRow {
    pub id: String,
    pub event_type: String,
    pub file_path: String,
    pub author: Option<String>,
    pub timestamp: i64,
}

#[tauri::command]
pub async fn get_recent_events(
    state: State<'_, AppState>,
    project_id: String,
    limit: i64,
) -> Result<Vec<EventRow>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT e.id, e.event_type, COALESCE(en.file_path, e.entity_id), e.author, e.timestamp
         FROM events e
         LEFT JOIN entities en ON e.entity_id = en.id
         WHERE e.project_id = ?1
         ORDER BY e.timestamp DESC
         LIMIT ?2"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let events = stmt.query_map([&project_id, &limit.to_string()], |row| {
        Ok(EventRow {
            id: row.get(0)?,
            event_type: row.get(1)?,
            file_path: row.get(2)?,
            author: row.get(3)?,
            timestamp: row.get(4)?,
        })
    }).map_err(|e| format!("Failed to query events: {}", e))?;
    
    events.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect events: {}", e))
}

// Status bar info
#[derive(Debug, Serialize)]
pub struct StatusBarInfo {
    pub scan_status: String,
    pub scan_progress: Option<ScanProgress>,
    pub db_size_mb: f64,
    pub active_agents: i64,
}

#[derive(Debug, Serialize)]
pub struct ScanProgress {
    pub total: i32,
    pub completed: i32,
    pub failed: i32,
    pub current_file: Option<String>,
}

#[tauri::command]
pub async fn get_status_bar_info(state: State<'_, AppState>, project_id: String) -> Result<StatusBarInfo, String> {
    let db = state.db.lock().await;
    
    // Get scan progress
    let total: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let completed: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1 AND status = 'completed'",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let failed: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1 AND status = 'failed'",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let current_file: Option<String> = db.query_row(
        "SELECT file_path FROM scan_jobs WHERE project_id = ?1 AND status = 'scanning' LIMIT 1",
        [&project_id],
        |row| row.get(0),
    ).ok();
    
    let pending: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1 AND status = 'pending'",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let active_agents: i64 = db.query_row(
        "SELECT COUNT(*) FROM agent_sessions WHERE project_id = ?1 AND status = 'active'",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    // Calculate DB size (approximate via page count * page size)
    let page_count: i64 = db.query_row("PRAGMA page_count", [], |row| row.get(0)).unwrap_or(0);
    let page_size: i64 = db.query_row("PRAGMA page_size", [], |row| row.get(0)).unwrap_or(4096);
    let db_size_mb = (page_count * page_size) as f64 / (1024.0 * 1024.0);
    
    let scan_status = if pending > 0 || current_file.is_some() {
        "scanning".to_string()
    } else {
        "current".to_string()
    };
    
    let scan_progress = if total > 0 {
        Some(ScanProgress { total, completed, failed, current_file })
    } else {
        None
    };
    
    Ok(StatusBarInfo {
        scan_status,
        scan_progress,
        db_size_mb,
        active_agents,
    })
}

// Cost tracking
#[derive(Debug, Serialize)]
pub struct CostSummary {
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub session_count: i64,
}

#[tauri::command]
pub async fn get_cost_summary(state: State<'_, AppState>, project_id: String) -> Result<CostSummary, String> {
    let db = state.db.lock().await;
    
    let total_tokens: i64 = db.query_row(
        "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) FROM cost_log WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    let estimated_cost: f64 = db.query_row(
        "SELECT COALESCE(SUM(cost_usd), 0) FROM cost_log WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0.0);
    
    let session_count: i64 = db.query_row(
        "SELECT COUNT(*) FROM cost_log WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    Ok(CostSummary {
        total_tokens,
        estimated_cost_usd: estimated_cost,
        session_count,
    })
}

#[derive(Debug, Serialize)]
pub struct CostLogRow {
    pub id: String,
    pub operation: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f64,
    pub task_type: String,
    pub timestamp: i64,
}

#[tauri::command]
pub async fn get_cost_log(
    state: State<'_, AppState>,
    project_id: String,
    limit: i64,
) -> Result<Vec<CostLogRow>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT id, operation, provider, model, input_tokens, output_tokens, cost_usd, task_type, timestamp
         FROM cost_log
         WHERE project_id = ?1
         ORDER BY timestamp DESC
         LIMIT ?2"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let logs = stmt.query_map([&project_id, &limit.to_string()], |row| {
        Ok(CostLogRow {
            id: row.get(0)?,
            operation: row.get(1)?,
            provider: row.get(2)?,
            model: row.get(3)?,
            input_tokens: row.get(4)?,
            output_tokens: row.get(5)?,
            cost_usd: row.get(6)?,
            task_type: row.get(7)?,
            timestamp: row.get(8)?,
        })
    }).map_err(|e| format!("Failed to query cost log: {}", e))?;
    
    logs.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect cost logs: {}", e))
}

// Entity summary for both query and relationships
#[derive(Debug, Serialize)]
pub struct EntitySummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line_start: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[tauri::command]
pub async fn query_entities(
    state: State<'_, AppState>,
    project_id: String,
    query: String,
    kind_filter: Option<String>,
) -> Result<Vec<EntitySummary>, String> {
    let db = state.db.lock().await;
    
    let mut sql = String::from(
        "SELECT id, name, type as kind, file_path, line_start, signature FROM entities WHERE project_id = ?1 AND (name LIKE ?2 OR file_path LIKE ?2)"
    );
    
    if let Some(ref kind) = kind_filter {
        sql.push_str(&format!(" AND type = '{}'", kind));
    }
    
    sql.push_str(" ORDER BY name LIMIT 100");
    
    let pattern = if query.is_empty() {
        "%".to_string()
    } else {
        format!("%{}%", query)
    };
    
    let mut stmt = db.prepare(&sql).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let entities = stmt.query_map([&project_id, &pattern], |row| {
        Ok(EntitySummary {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            file_path: row.get(3)?,
            line_start: row.get(4)?,
            signature: row.get(5).ok(),
        })
    }).map_err(|e| format!("Failed to query entities: {}", e))?;
    
    entities.collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Failed to collect entities: {}", e))
}

// Event in entity detail
#[derive(Debug, Serialize)]
pub struct EntityEvent {
    pub event_type: String,
    pub timestamp: i64,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

// Decision in entity detail
#[derive(Debug, Serialize)]
pub struct EntityDecision {
    pub id: String,
    pub title: String,
    pub rationale: String,
}

// Full entity detail response
#[derive(Debug, Serialize)]
pub struct EntityDetail {
    pub entity: EntitySummary,
    pub imports: Vec<EntitySummary>,
    pub imported_by: Vec<EntitySummary>,
    pub calls: Vec<EntitySummary>,
    pub called_by: Vec<EntitySummary>,
    pub recent_events: Vec<EntityEvent>,
    pub decisions: Vec<EntityDecision>,
}

#[derive(Debug, Serialize)]
pub struct RelationshipInfo {
    pub id: String,
    pub target_name: String,
    pub target_id: Option<String>,
    pub kind: String,
    pub direction: String, // "outgoing" or "incoming"
}

#[derive(Debug, Serialize)]
pub struct DecisionInfo {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[tauri::command]
pub async fn get_entity_detail(
    state: State<'_, AppState>,
    entity_id: String,
) -> Result<EntityDetail, String> {
    let db = state.db.lock().await;
    
    // Get entity
    let mut stmt = db.prepare(
        "SELECT id, name, type, file_path, signature, line_start
         FROM entities WHERE id = ?1"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let (id, name, kind, file_path, signature, line_start): 
        (String, String, String, String, Option<String>, i64) = stmt
        .query_row([&entity_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        }).map_err(|e| format!("Failed to get entity: {}", e))?;
    
    let entity = EntitySummary {
        id: id.clone(),
        name: name.clone(),
        kind,
        file_path,
        signature,
        line_start,
    };
    
    // Get relationships - imports (this entity imports others)
    let mut imports: Vec<EntitySummary> = Vec::new();
    let mut imported_by: Vec<EntitySummary> = Vec::new();
    let mut calls: Vec<EntitySummary> = Vec::new();
    let mut called_by: Vec<EntitySummary> = Vec::new();
    
    {
        let mut stmt = db.prepare(
            "SELECT e.id, e.name, e.type, e.file_path, e.signature, e.line_start, r.type
             FROM relationships r
             JOIN entities e ON r.target_id = e.id
             WHERE r.source_id = ?1"
        ).map_err(|e| format!("Failed to prepare outgoing rel query: {}", e))?;
        
        let rows = stmt
            .query_map([&entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            }).map_err(|e| format!("Failed to query outgoing rels: {}", e))?;
        
        for row in rows {
            if let Ok((id, name, kind, file_path, signature, line_start, rel_type)) = row {
                let summary = EntitySummary {
                    id,
                    name,
                    kind,
                    file_path,
                    signature,
                    line_start,
                };
                match rel_type.as_str() {
                    "imports" => imports.push(summary),
                    "calls" => calls.push(summary),
                    _ => imports.push(summary), // default to imports
                }
            }
        }
    }
    
    // Get relationships - imported_by/called_by (other entities reference this)
    {
        let mut stmt = db.prepare(
            "SELECT e.id, e.name, e.type, e.file_path, e.signature, e.line_start, r.type
             FROM relationships r
             JOIN entities e ON r.source_id = e.id
             WHERE r.target_id = ?1"
        ).map_err(|e| format!("Failed to prepare incoming rel query: {}", e))?;
        
        let rows = stmt
            .query_map([&entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            }).map_err(|e| format!("Failed to query incoming rels: {}", e))?;
        
        for row in rows {
            if let Ok((id, name, kind, file_path, signature, line_start, rel_type)) = row {
                let summary = EntitySummary {
                    id,
                    name,
                    kind,
                    file_path,
                    signature,
                    line_start,
                };
                match rel_type.as_str() {
                    "imports" => imported_by.push(summary),
                    "calls" => called_by.push(summary),
                    _ => imported_by.push(summary), // default to imported_by
                }
            }
        }
    }
    
    // Get recent events for this entity
    let mut recent_events: Vec<EntityEvent> = Vec::new();
    {
        let mut stmt = db.prepare(
            "SELECT event_type, timestamp, parent_event_id, diff_summary
             FROM events WHERE entity_id = ?1 OR 
             (event_type IN ('entity_added', 'entity_removed', 'signature_changed') 
              AND diff_summary LIKE ?2)
             ORDER BY timestamp DESC LIMIT 5"
        ).map_err(|e| format!("Failed to prepare event query: {}", e))?;
        
        let rows = stmt
            .query_map([&entity_id, &format!("%{}%", &name)], |row| {
                Ok(EntityEvent {
                    event_type: row.get(0)?,
                    timestamp: row.get(1)?,
                    old_value: None,
                    new_value: row.get(3)?,
                })
            }).map_err(|e| format!("Failed to query events: {}", e))?;
        
        for row in rows {
            if let Ok(event) = row {
                recent_events.push(event);
            }
        }
    }
    
    // Get linked decisions
    let mut decisions: Vec<EntityDecision> = Vec::new();
    {
        let mut stmt = db.prepare(
            "SELECT id, title, context FROM decisions
             WHERE linked_entities LIKE ?1 OR linked_entities LIKE ?2
             ORDER BY created_at DESC LIMIT 5"
        ).map_err(|e| format!("Failed to prepare decision query: {}", e))?;
        
        let patterns = vec![format!("%{}%", &id), format!("%,{}%", &id)];
        let rows = stmt
            .query_map([&patterns[0], &patterns[1]], |row| {
                Ok(EntityDecision {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    rationale: row.get(2)?,
                })
            }).map_err(|e| format!("Failed to query decisions: {}", e))?;
        
        for row in rows {
            if let Ok(decision) = row {
                decisions.push(decision);
            }
        }
    }
    
    Ok(EntityDetail {
        entity,
        imports,
        imported_by,
        calls,
        called_by,
        recent_events,
        decisions,
    })
}

// Open project (creates new or returns existing)
#[tauri::command]
pub async fn open_project(
    path: String,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Project, String> {
    let db = state.db.lock().await;
    
    // Check if project already exists
    let existing: Option<Project> = db.query_row(
        "SELECT id, name, root_path, tech_stack, created_at, last_scanned_at, health_score 
         FROM projects WHERE root_path = ?1",
        [&path],
        |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                tech_stack: row.get(3)?,
                created_at: row.get(4)?,
                last_scanned_at: row.get(5)?,
                health_score: row.get(6)?,
            })
        },
    ).ok();
    
    let project = if let Some(project) = existing {
        // Update last_scanned_at (we use this as last_opened)
        let now = Utc::now().timestamp();
        db.execute(
            "UPDATE projects SET last_scanned_at = ?1 WHERE id = ?2",
            [&now.to_string(), &project.id],
        ).map_err(|e| format!("Failed to update project: {}", e))?;
        
        Project {
            last_scanned_at: Some(now),
            ..project
        }
    } else {
        // Create new project
        let id = Uuid::new_v4().to_string();
        let name = std::path::Path::new(&path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let created_at = Utc::now().timestamp();
        
        db.execute(
            "INSERT INTO projects (id, name, root_path, tech_stack, created_at, health_score) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &id,
                &name,
                &path,
                "",
                &created_at.to_string(),
                "0",
            ],
        ).map_err(|e| format!("Failed to create project: {}", e))?;
        
        Project {
            id,
            name,
            root_path: path.clone(),
            tech_stack: None,
            created_at,
            last_scanned_at: Some(created_at),
            health_score: 0.0,
        }
    };
    
    drop(db); // Release lock before async operations
    
    // Start the file watcher for this project
    let watch_path = project.root_path.clone();
    let _ = crate::watcher::start_watching(&watch_path).await;
    
    // Queue scan jobs
    let db_arc = state.db.clone();
    let file_count = crate::scanner::queue_scan_jobs(&project.id, &project.root_path, &db_arc).await
        .map_err(|e| format!("Failed to queue scan jobs: {}", e))?;
    
    // Emit scan started event
    let payload = serde_json::json!({
        "project_id": project.id,
        "total_files": file_count
    });
    
    if let Err(e) = app_handle.emit("knot://scan-started", payload) {
        tracing::error!("Failed to emit scan started: {}", e);
    }
    
    // Spawn background worker (non-blocking)
    let project_id = project.id.clone();
    let app_handle_clone = app_handle.clone();
    let db_arc_clone = state.db.clone();
    
    tauri::async_runtime::spawn(async move {
        crate::scanner::run_scan_worker(project_id, app_handle_clone, db_arc_clone).await;
    });
    
    tracing::info!("Scan started for project {}: {} files queued", project.id, file_count);
    
    Ok(project)
}

// Generate MCP configuration for Claude Code / OpenCode / Cursor
#[tauri::command]
pub async fn get_mcp_config(
    project_id: String,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    let binary_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();
    
    // On Windows, we need to handle the path differently for the config
    let binary_path = if cfg!(windows) {
        // Convert Windows path to forward slashes for JSON
        binary_path.replace('\\', "\\")
    } else {
        binary_path
    };
    
    let config = serde_json::json!({
        "mcpServers": {
            "knot": {
                "command": binary_path,
                "args": ["mcp-server", format!("--project-id={}", project_id)]
            }
        }
    });
    
    Ok(serde_json::to_string_pretty(&config).unwrap())
}

// Detect Ollama availability
#[tauri::command]
pub async fn detect_ollama() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    
    let response = client
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let models = body.get("models").and_then(|m| m.as_array()).map(|m| m.len()).unwrap_or(0);
                Ok(serde_json::json!({
                    "available": true,
                    "models": models
                }))
            } else {
                Ok(serde_json::json!({
                    "available": false,
                    "models": 0
                }))
            }
        }
        Err(_) => {
            Ok(serde_json::json!({
                "available": false,
                "models": 0
            }))
        }
    }
}

// Save AI settings
#[derive(Debug, Deserialize)]
pub struct AiSettings {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub ollama_url: String,
    pub enrichment_trigger: String,
}

#[tauri::command]
pub async fn save_ai_settings(
    settings: AiSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let now = Utc::now().timestamp();
    
    // Simple XOR "encryption" for API key (placeholder for real keychain in v2)
    let encrypted_key: String = settings.api_key.bytes().enumerate().map(|(i, b)| {
        let key_byte = (i % 256) as u8;
        (b ^ key_byte) as char
    }).collect();
    
    let settings_json = serde_json::json!({
        "provider": settings.provider,
        "model": settings.model,
        "api_key": encrypted_key,
        "ollama_url": settings.ollama_url,
        "enrichment_trigger": settings.enrichment_trigger,
    });
    
    db.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER
        )",
        [],
    ).map_err(|e| e.to_string())?;
    
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at)
         VALUES ('ai_settings', ?1, ?2)",
        [settings_json.to_string(), now.to_string()],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Test AI connection
#[tauri::command]
pub async fn test_ai_connection(
    provider: String,
    _model: String,
    api_key: String,
    ollama_url: String,
) -> Result<String, String> {
    match provider.as_str() {
        "anthropic" => {
            let client = reqwest::Client::new();
            let response = client
                .get("https://api.anthropic.com/v1/models")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            
            if response.status().is_success() {
                Ok("Successfully connected to Anthropic".to_string())
            } else {
                Err(format!("Connection failed: {}", response.status()))
            }
        }
        "openai" => {
            let client = reqwest::Client::new();
            let response = client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(api_key)
                .send()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            
            if response.status().is_success() {
                Ok("Successfully connected to OpenAI".to_string())
            } else {
                Err(format!("Connection failed: {}", response.status()))
            }
        }
        "ollama" => {
            let client = reqwest::Client::new();
            let response = client
                .get(format!("{}/api/tags", ollama_url))
                .send()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            
            if response.status().is_success() {
                Ok("Successfully connected to Ollama".to_string())
            } else {
                Err(format!("Connection failed: {}", response.status()))
            }
        }
        "google" => {
            // Gemini API test
            let client = reqwest::Client::new();
            let response = client
                .get(format!(
                    "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                    api_key
                ))
                .send()
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;
            
            if response.status().is_success() {
                Ok("Successfully connected to Google Gemini".to_string())
            } else {
                Err(format!("Connection failed: {}", response.status()))
            }
        }
        _ => Err("Unknown provider".to_string()),
    }
}
