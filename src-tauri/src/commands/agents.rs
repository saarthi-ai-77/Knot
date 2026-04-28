use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct AgentSession {
    pub id: String,
    pub project_id: String,
    pub agent_type: String,
    pub status: String,
    pub current_task: Option<String>,
    pub last_context_pack_id: Option<String>,
    pub created_at: i64,
    pub last_active_at: i64,
    pub resumed_at: Option<i64>,
    pub resume_count: i64,
    pub open_files_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionFile {
    pub file_path: String,
    pub opened_at: i64,
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<AgentSession>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT s.id, s.project_id, s.agent_type, s.status, s.current_task, s.last_context_pack_id,
         s.created_at, s.last_active_at, s.resumed_at, s.resume_count,
         (SELECT COUNT(*) FROM session_files WHERE session_id = s.id) as open_files_count
         FROM agent_sessions s
         WHERE s.project_id = ?1 AND s.status IN ('active', 'idle')
         ORDER BY s.last_active_at DESC"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let sessions = stmt
        .query_map([&project_id], |row| {
            Ok(AgentSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                agent_type: row.get(2)?,
                status: row.get(3)?,
                current_task: row.get(4)?,
                last_context_pack_id: row.get(5)?,
                created_at: row.get(6)?,
                last_active_at: row.get(7)?,
                resumed_at: row.get(8)?,
                resume_count: row.get(9)?,
                open_files_count: row.get(10)?,
            })
        }).map_err(|e| format!("Failed to query sessions: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect sessions: {}", e))?;
    
    Ok(sessions)
}

#[tauri::command]
pub async fn get_session_files(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<SessionFile>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT file_path, opened_at FROM session_files
         WHERE session_id = ?1 ORDER BY opened_at DESC"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let files = stmt
        .query_map([&session_id], |row| {
            Ok(SessionFile {
                file_path: row.get(0)?,
                opened_at: row.get(1)?,
            })
        }).map_err(|e| format!("Failed to query session files: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect session files: {}", e))?;
    
    Ok(files)
}

#[derive(Debug, Deserialize)]
pub struct LogTaskRequest {
    pub project_id: String,
    pub session_id: Option<String>,
    pub description: String,
}

#[tauri::command]
pub async fn log_task(
    state: State<'_, AppState>,
    request: LogTaskRequest,
) -> Result<String, String> {
    let db = state.db.lock().await;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    
    // Insert into events table with type 'task_logged'
    db.execute(
        "INSERT INTO events (id, project_id, entity_id, event_type, author, diff_summary, timestamp)
         VALUES (?1, ?2, ?3, 'task_logged', 'user', ?4, ?5)",
        [
            &id,
            &request.project_id,
            request.session_id.as_deref().unwrap_or(""),
            &request.description,
            &now.to_string(),
        ],
    ).map_err(|e| format!("Failed to log task: {}", e))?;
    
    // Update session if provided
    if let Some(session_id) = request.session_id {
        db.execute(
            "UPDATE agent_sessions SET current_task = ?1, last_active_at = ?2 WHERE id = ?3",
            [&request.description, &now.to_string(), &session_id],
        ).map_err(|e| format!("Failed to update session: {}", e))?;
    }
    
    Ok(id)
}

#[derive(Debug, Serialize)]
pub struct TaskLog {
    pub id: String,
    pub description: String,
    pub timestamp: i64,
}

#[tauri::command]
pub async fn get_task_log(
    state: State<'_, AppState>,
    project_id: String,
    limit: i64,
) -> Result<Vec<TaskLog>, String> {
    let db = state.db.lock().await;
    
    let mut stmt = db.prepare(
        "SELECT id, diff_summary, timestamp FROM events
         WHERE project_id = ?1 AND event_type = 'task_logged'
         ORDER BY timestamp DESC LIMIT ?2"
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let tasks = stmt
        .query_map([&project_id, &limit.to_string()], |row| {
            Ok(TaskLog {
                id: row.get(0)?,
                description: row.get(1)?,
                timestamp: row.get(2)?,
            })
        }).map_err(|e| format!("Failed to query task log: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect task log: {}", e))?;
    
    Ok(tasks)
}

#[tauri::command]
pub async fn generate_resume_prompt(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    let db = state.db.lock().await;
    
    // 1. Get project info
    let (project_name, project_root): (String, String) = db.query_row(
        "SELECT name, root_path FROM projects WHERE id = ?1",
        [&project_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| format!("Failed to get project info: {}", e))?;
    
    // 2. Get current task (most recent task log entry)
    let current_task: String = db.query_row(
        "SELECT diff_summary FROM events 
         WHERE project_id = ?1 AND event_type = 'task_logged' 
         ORDER BY timestamp DESC LIMIT 1",
        [&project_id],
        |row| row.get(0),
    ).unwrap_or_else(|_| "No task logged. Describe what you were working on.".to_string());
    
    // 3. Get recent events (last 15)
    let mut event_stmt = db.prepare(
        "SELECT event_type, diff_summary, timestamp FROM events 
         WHERE project_id = ?1 
         ORDER BY timestamp DESC LIMIT 15"
    ).map_err(|e| format!("Failed to prepare event query: {}", e))?;
    
    let events: Vec<String> = event_stmt
        .query_map([&project_id], |row| {
            let event_type: String = row.get(0)?;
            let diff: String = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            Ok(format!("[{}] {}: {}", time_str, event_type, diff))
        }).map_err(|e| format!("Failed to query events: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    
    // 4. Get recently modified entities (last 10 unique)
    let mut entity_stmt = db.prepare(
        "SELECT DISTINCT e.name, e.type, e.file_path, e.signature 
         FROM events ev 
         JOIN entities e ON ev.entity_id = e.id 
         WHERE ev.project_id = ?1 
         ORDER BY ev.timestamp DESC LIMIT 10"
    ).map_err(|e| format!("Failed to prepare entity query: {}", e))?;
    
    let entities: Vec<String> = entity_stmt
        .query_map([&project_id], |row| {
            let name: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let file_path: String = row.get(2)?;
            Ok(format!("- {} ({}) in {}", name, kind, file_path.split('/').last().unwrap_or(&file_path)))
        }).map_err(|e| format!("Failed to query entities: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    
    // 5. Get git diff stat
    let git_diff = std::process::Command::new("git")
        .args(["diff", "--stat", "HEAD"])
        .current_dir(&project_root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    
    // 6. Get recent decisions
    let mut stmt = db.prepare(
        "SELECT title, context FROM decisions 
         WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 5"
    ).map_err(|e| format!("Failed to prepare decision query: {}", e))?;
    
    let decisions: Vec<String> = stmt
        .query_map([&project_id], |row| {
            let title: String = row.get(0)?;
            let context: String = row.get(1)?;
            Ok(format!("- {}: {}", title, context.chars().take(100).collect::<String>()))
        }).map_err(|e| format!("Failed to query decisions: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    
    // 7. Compose the resume prompt
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    
    let events_str = if events.is_empty() { 
        "No recent activity recorded.".to_string() 
    } else { 
        events.join("\n") 
    };
    
    let entities_str = if entities.is_empty() { 
        "No entities modified yet.".to_string() 
    } else { 
        entities.join("\n") 
    };
    
    let decisions_str = if decisions.is_empty() { 
        "No decisions recorded yet.".to_string() 
    } else { 
        decisions.join("\n") 
    };
    
    let git_diff_str = if git_diff.is_empty() { 
        "No git changes detected or git not available.".to_string() 
    } else { 
        git_diff 
    };
    
    let prompt = format!(
        r##"## Knot Resume Context
Generated: {timestamp}
Project: {project_name}

---

### Current Task
{current_task}

---

### Git Changes Since Last Commit
{git_diff_str}

---

### Recent Activity (last 15 events)
{events_str}

---

### Entities Being Modified
{entities_str}

---

### Active Decisions
{decisions_str}

---

### Instructions
You are resuming development on this project. The above context was captured automatically by Knot (local codebase intelligence).

- The current task is clearly stated above
- Review git changes to understand what is already done
- The entities listed are what was being actively modified
- Do not re-read files already covered by the entities above
- Follow the active decisions as architectural constraints
- Continue from where the previous agent left off

Begin by briefly acknowledging what you understand about the current state, then proceed with the next logical step.
"##,
        timestamp = timestamp,
        project_name = project_name,
        current_task = current_task,
        git_diff_str = git_diff_str,
        events_str = events_str,
        entities_str = entities_str,
        decisions_str = decisions_str
    );
    
    Ok(prompt)
}

// Create a new agent session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub project_id: String,
    pub agent_type: String,
}

#[tauri::command]
pub async fn create_agent_session(
    state: State<'_, AppState>,
    request: CreateSessionRequest,
) -> Result<AgentSession, String> {
    let db = state.db.lock().await;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    
    db.execute(
        "INSERT INTO agent_sessions (id, project_id, agent_type, status, created_at, last_active_at, resume_count)
         VALUES (?1, ?2, ?3, 'active', ?4, ?5, 0)",
        [
            &id,
            &request.project_id,
            &request.agent_type,
            &now.to_string(),
            &now.to_string(),
        ],
    ).map_err(|e| format!("Failed to create session: {}", e))?;
    
    Ok(AgentSession {
        id,
        project_id: request.project_id,
        agent_type: request.agent_type,
        status: "active".to_string(),
        current_task: None,
        last_context_pack_id: None,
        created_at: now,
        last_active_at: now,
        resumed_at: None,
        resume_count: 0,
        open_files_count: 0,
    })
}

// Update session status
#[tauri::command]
pub async fn update_session_status(
    state: State<'_, AppState>,
    session_id: String,
    status: String,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let now = Utc::now().timestamp();
    
    db.execute(
        "UPDATE agent_sessions SET status = ?1, last_active_at = ?2 WHERE id = ?3",
        [&status, &now.to_string(), &session_id],
    ).map_err(|e| format!("Failed to update session status: {}", e))?;
    
    Ok(())
}
