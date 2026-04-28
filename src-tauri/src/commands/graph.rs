use crate::AppState;
use crate::parser::{self, ParsedFile};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total: i32,
    pub completed: i32,
    pub failed: i32,
    pub current_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileScanJob {
    pub id: String,
    pub project_id: String,
    pub file_path: String,
    pub priority: i32,
    pub status: String,
}

#[tauri::command]
pub async fn parse_file(file_path: String) -> Result<ParsedFile, String> {
    parser::parse_file(&file_path).await
}

#[tauri::command]
pub async fn parse_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<String, String> {
    // Get project info
    let db = state.db.lock().await;
    
    let root_path: String = db.query_row(
        "SELECT root_path FROM projects WHERE id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get project: {}", e))?;
    
    drop(db);
    
    // Queue file scan jobs
    let jobs = queue_scan_jobs(&state, &project_id, &root_path).await?;
    
    // Start scan workers
    let job_count = jobs.len();
    let app_state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_scan_workers(app_state, project_id, jobs).await {
            tracing::error!("Scan failed: {}", e);
        }
    });
    
    Ok(format!("Queued {} files for scanning", job_count))
}

async fn queue_scan_jobs(
    state: &State<'_, AppState>,
    project_id: &str,
    root_path: &str,
) -> Result<Vec<FileScanJob>, String> {
    let mut jobs = Vec::new();
    let mut walker = ignore::WalkBuilder::new(root_path);
    walker.add_custom_ignore_filename(".gitignore");
    walker.hidden(false);
    
    let walker = walker.build();
    
    // Get git recently modified files for priority
    let git_recent = get_git_recent_files(root_path).await.unwrap_or_default();
    let git_recent_set: std::collections::HashSet<_> = git_recent.iter().cloned().collect();
    
    for entry in walker {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        // Only queue supported files
        if !["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "py", "rs", "go", "java"].contains(&ext) {
            continue;
        }
        
        let file_path = path.to_string_lossy().to_string();
        let priority = calculate_priority(&file_path, &git_recent_set, root_path);
        
        let job = FileScanJob {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            file_path,
            priority,
            status: "pending".to_string(),
        };
        
        jobs.push(job);
    }
    
    // Insert jobs into database
    let db = state.db.lock().await;
    
    for job in &jobs {
        db.execute(
            "INSERT OR IGNORE INTO scan_jobs 
             (id, project_id, file_path, status, priority, retry_count)
             VALUES (?1, ?2, ?3, 'pending', ?4, 0)",
            [
                &job.id,
                &job.project_id,
                &job.file_path,
                &job.priority.to_string(),
            ],
        ).map_err(|e| format!("Failed to insert scan job: {}", e))?;
    }
    
    tracing::info!("Queued {} scan jobs for project {}", jobs.len(), project_id);
    Ok(jobs)
}

fn calculate_priority(file_path: &str, git_recent: &std::collections::HashSet<String>, root_path: &str) -> i32 {
    let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);
    
    // Priority 100: Recently modified in git
    if git_recent.contains(file_path) {
        return 100;
    }
    
    // Priority 50: Root or src directory
    if relative_path.starts_with("/src/") || relative_path.starts_with("src/") {
        return 50;
    }
    
    if !relative_path.contains('/') {
        return 50;
    }
    
    // Priority 10: Everything else
    10
}

async fn get_git_recent_files(root_path: &str) -> Result<Vec<String>, String> {
    let output = tokio::process::Command::new("git")
        .args(["-C", root_path, "diff", "--name-only", "HEAD~10..HEAD"])
        .output()
        .await
        .map_err(|e| format!("Git command failed: {}", e))?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout
        .lines()
        .map(|f| format!("{}/{}", root_path, f))
        .collect();
    
    Ok(files)
}

async fn run_scan_workers(
    state: AppState,
    project_id: String,
    _jobs: Vec<FileScanJob>,
) -> Result<(), String> {
    // Get pending jobs sorted by priority
    let pending_jobs: Vec<(String, String)> = {
        let db = state.db.lock().await;
        
        let mut stmt = db.prepare(
            "SELECT id, file_path FROM scan_jobs 
             WHERE project_id = ?1 AND status = 'pending'
             ORDER BY priority DESC"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;
        
        let jobs = stmt.query_map([&project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("Failed to query pending jobs: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect jobs: {}", e))?;
        jobs
    };
    
    let total = pending_jobs.len();
    let mut completed = 0;
    let mut failed = 0;
    
    // Process in chunks of 50
    for chunk in pending_jobs.chunks(50) {
        let chunk_results: Vec<Result<(), String>> = futures::future::join_all(
            chunk.iter().map(|(job_id, file_path)| {
                let job_id = job_id.clone();
                let file_path = file_path.clone();
                let project_id = project_id.to_string();
                
                let state = state.clone();
                async move {
                    process_single_file(state, job_id, project_id, file_path).await
                }
            })
        ).await;
        
        for result in chunk_results {
            match result {
                Ok(_) => completed += 1,
                Err(_) => failed += 1,
            }
        }
        
        // Emit progress event
        // This would emit: knot://scan-progress
    }
    
    tracing::info!("Scan completed: {} total, {} completed, {} failed", total, completed, failed);
    Ok(())
}

async fn process_single_file(
    state: AppState,
    job_id: String,
    project_id: String,
    file_path: String,
) -> Result<(), String> {
    // Update status to scanning
    {
        let db = state.db.lock().await;
        let started_at = Utc::now().timestamp();
        db.execute(
            "UPDATE scan_jobs SET status = 'scanning', started_at = ?1 WHERE id = ?2",
            [&started_at.to_string(), &job_id],
        ).map_err(|e| format!("Failed to update job status: {}", e))?;
    }
    
    // Parse the file
    match parser::parse_file(&file_path).await {
        Ok(parsed) => {
            // Insert entities and relationships
            let db = state.db.lock().await;
            
            // Insert file entity
            let file_entity_id = Uuid::new_v4().to_string();
            let now = Utc::now().timestamp();
            
            db.execute(
                "INSERT INTO entities (id, project_id, type, name, file_path, created_at, modified_at)
                 VALUES (?1, ?2, 'file', ?3, ?4, ?5, ?6)",
                [
                    &file_entity_id,
                    &project_id,
                    &Path::new(&file_path).file_name().unwrap_or_default().to_string_lossy().to_string(),
                    &file_path,
                    &now.to_string(),
                    &now.to_string(),
                ],
            ).map_err(|e| format!("Failed to insert file entity: {}", e))?;
            
            // Insert parsed entities
            for entity in &parsed.entities {
                let entity_id = Uuid::new_v4().to_string();
                
                db.execute(
                    "INSERT INTO entities (id, project_id, type, name, file_path, line_start, line_end, signature, is_public, created_at, modified_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    [
                        &entity_id,
                        &project_id,
                        &entity.kind,
                        &entity.name,
                        &file_path,
                        &entity.line_start.to_string(),
                        &entity.line_end.to_string(),
                        &entity.signature,
                        &entity.is_public.to_string(),
                        &now.to_string(),
                        &now.to_string(),
                    ],
                ).map_err(|e| format!("Failed to insert entity: {}", e))?;
            }
            
            // Update job status to completed
            let completed_at = Utc::now().timestamp();
            db.execute(
                "UPDATE scan_jobs SET status = 'completed', completed_at = ?1 WHERE id = ?2",
                [&completed_at.to_string(), &job_id],
            ).map_err(|e| format!("Failed to update job status: {}", e))?;
            
            tracing::debug!("Processed file: {}", file_path);
            Ok(())
        }
        Err(e) => {
            // Update job status to failed
            let db = state.db.lock().await;
            let completed_at = Utc::now().timestamp();
            db.execute(
                "UPDATE scan_jobs SET status = 'failed', completed_at = ?1, error_message = ?2 WHERE id = ?3",
                [&completed_at.to_string(), &e, &job_id],
            ).map_err(|e| format!("Failed to update job status: {}", e))?;
            
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_scan_progress(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ScanProgress, String> {
    let db = state.db.lock().await;
    
    let total: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get total: {}", e))?;
    
    let completed: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1 AND status IN ('completed', 'parsed', 'indexed')",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get completed: {}", e))?;
    
    let failed: i32 = db.query_row(
        "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1 AND status = 'failed'",
        [&project_id],
        |row| row.get(0),
    ).map_err(|e| format!("Failed to get failed: {}", e))?;
    
    let current_file: Option<String> = db.query_row(
        "SELECT file_path FROM scan_jobs WHERE project_id = ?1 AND status = 'scanning' LIMIT 1",
        [&project_id],
        |row| row.get(0),
    ).ok();
    
    Ok(ScanProgress {
        total,
        completed,
        failed,
        current_file,
    })
}
