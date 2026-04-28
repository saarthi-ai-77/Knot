use crate::AppState;
use crate::parser::{self, ParsedFile};
use rusqlite::{params, Connection};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

pub async fn queue_scan_jobs(
    project_id: &str,
    root_path: &str,
    state: &Arc<Mutex<Connection>>,
) -> Result<usize, String> {
    let conn = state.lock().await;
    let mut file_count = 0;
    
    // Walk the project directory
    let mut walker = ignore::WalkBuilder::new(root_path);
    walker.add_custom_ignore_filename(".gitignore");
    walker.hidden(false);
    walker.filter_entry(|entry| {
        let path = entry.path();
        let path_str = path.to_string_lossy();
        
        // Skip common non-source directories
        if path_str.contains("/node_modules/") || path_str.contains("\\node_modules\\") {
            return false;
        }
        if path_str.contains("/.git/") || path_str.contains("\\.git\\") {
            return false;
        }
        if path_str.contains("/dist/") || path_str.contains("\\dist\\") {
            return false;
        }
        if path_str.contains("/build/") || path_str.contains("\\build\\") {
            return false;
        }
        if path_str.contains("/target/") || path_str.contains("\\target\\") {
            return false;
        }
        if path_str.contains("/.next/") || path_str.contains("\\.next\\") {
            return false;
        }
        true
    });
    
    for entry in walker.build() {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        // Only queue supported file types
        if !["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx", "py", "rs", "go", "java"].contains(&ext) {
            continue;
        }
        
        let file_path = path.to_string_lossy().to_string();
        
        // Compute content hash
        let content_hash = match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => compute_hash(&content),
            Err(_) => continue, // Skip files we can't read
        };
        
        // Calculate priority
        let priority = calculate_priority(&file_path, root_path);
        
        // Upsert scan job
        let job_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        conn.execute(
            "INSERT INTO scan_jobs (id, project_id, file_path, status, priority, content_hash, retry_count, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, 0, ?6)
             ON CONFLICT(project_id, file_path) 
             DO UPDATE SET 
                status = CASE WHEN content_hash != excluded.content_hash THEN 'pending' ELSE status END,
                content_hash = excluded.content_hash,
                retry_count = CASE WHEN content_hash != excluded.content_hash THEN 0 ELSE retry_count END
             WHERE status != 'pending'",
            params![&job_id, &project_id, &file_path, &priority, &content_hash, &now],
        ).map_err(|e| format!("Failed to upsert scan job: {}", e))?;
        
        file_count += 1;
    }
    
    Ok(file_count)
}

fn compute_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn calculate_priority(file_path: &str, root_path: &str) -> i32 {
    let relative_path = file_path.strip_prefix(root_path).unwrap_or(file_path);
    
    // Priority 50: Root level, src/, lib/, app/
    if relative_path.starts_with("/src/") || relative_path.starts_with("\\src\\") ||
       relative_path.starts_with("/lib/") || relative_path.starts_with("\\lib\\") ||
       relative_path.starts_with("/app/") || relative_path.starts_with("\\app\\") ||
       !relative_path.contains('/') && !relative_path.contains('\\') {
        return 50;
    }
    
    // Priority 10: Everything else
    10
}

pub async fn run_scan_worker(
    project_id: String,
    app_handle: AppHandle,
    state: Arc<Mutex<Connection>>,
) {
    let mut completed_count = 0;
    let mut failed_count = 0;
    
    // Get total count for progress
    let total_count: i64 = {
        let conn = state.lock().await;
        conn.query_row(
            "SELECT COUNT(*) FROM scan_jobs WHERE project_id = ?1",
            params![&project_id],
            |row| row.get(0),
        ).unwrap_or(0)
    };
    
    if total_count == 0 {
        return;
    }
    
    loop {
        // Claim next job atomically
        let job_result: Option<(String, String)> = {
            let conn = state.lock().await;
            conn.query_row(
                "UPDATE scan_jobs 
                 SET status = 'scanning', started_at = unixepoch()
                 WHERE id = (
                     SELECT id FROM scan_jobs 
                     WHERE project_id = ?1 AND status = 'pending'
                     ORDER BY priority DESC, created_at ASC
                     LIMIT 1
                 )
                 RETURNING id, file_path",
                params![&project_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            ).ok()
        };
        
        let (job_id, file_path) = match job_result {
            Some((id, path)) => (id, path),
            None => break, // No more jobs
        };
        
        // Parse the file
        match parser::parse_file(&file_path).await {
            Ok(parsed) => {
                // Write results to DB
                if let Err(e) = write_parsed_results(&project_id, &file_path, &parsed, &state, &job_id).await {
                    tracing::error!("Failed to write parsed results: {}", e);
                    failed_count += 1;
                } else {
                    completed_count += 1;
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse file {}: {}", file_path, e);
                // Mark as failed
                let conn = state.lock().await;
                let _ = conn.execute(
                    "UPDATE scan_jobs SET status = 'failed', error_message = ?1, retry_count = retry_count + 1 WHERE id = ?2",
                    params![e, &job_id],
                );
                failed_count += 1;
            }
        }
        
        // Emit progress every 25 files or on completion
        if completed_count % 25 == 0 || completed_count + failed_count == total_count as usize {
            let percentage = ((completed_count + failed_count) * 100) / total_count as usize;
            let payload = serde_json::json!({
                "project_id": project_id,
                "completed": completed_count,
                "total": total_count,
                "failed": failed_count,
                "current_file": file_path,
                "percentage": percentage
            });
            
            if let Err(e) = app_handle.emit("knot://scan-progress", payload) {
                tracing::error!("Failed to emit scan progress: {}", e);
            }
        }
    }
    
    // Emit completion event
    let payload = serde_json::json!({
        "project_id": project_id,
        "total": completed_count,
        "failed": failed_count
    });
    
    if let Err(e) = app_handle.emit("knot://scan-complete", payload) {
        tracing::error!("Failed to emit scan complete: {}", e);
    }
    
    tracing::info!("Scan worker completed: {} files processed, {} failed", completed_count, failed_count);
}

async fn write_parsed_results(
    project_id: &str,
    file_path: &str,
    parsed: &ParsedFile,
    state: &Arc<Mutex<Connection>>,
    job_id: &str,
) -> Result<(), String> {
    let conn = state.lock().await;
    
    // Start transaction
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    
    // Remove old entities for this file
    tx.execute(
        "DELETE FROM entities WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path],
    ).map_err(|e| e.to_string())?;
    
    // Insert new entities
    let mut entity_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for entity in &parsed.entities {
        let entity_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        tx.execute(
            "INSERT INTO entities (id, project_id, type, name, file_path, line_start, line_end, signature, is_public, metadata, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                &entity_id,
                &project_id,
                &entity.kind,
                &entity.name,
                &file_path,
                &entity.line_start,
                &entity.line_end,
                &entity.signature,
                &entity.is_public,
                &format!("{{\"language\": \"{}\"}}", parsed.language),
                &now,
                &now
            ],
        ).map_err(|e| e.to_string())?;
        
        // Store entity ID for relationship lookup
        let key = format!("{}:{}", entity.name, entity.kind);
        entity_ids.insert(key, entity_id);
    }
    
    // Insert relationships
    for rel in &parsed.relationships {
        let rel_id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
    // Try to find source and target entities by name within this project
        
        // Look up actual entity IDs from what we just inserted or existing entities
        let source_id = find_entity_id(&tx, project_id, &rel.source_name)?;
        let target_id = find_entity_id(&tx, project_id, &rel.target_name)?;
        
        // Only insert relationship if we found at least one entity
        let source_id = source_id.unwrap_or_else(|| file_path.to_string());
        let target_id = target_id.unwrap_or_else(|| rel.target_name.clone());
        
        tx.execute(
            "INSERT INTO relationships (id, source_id, target_id, type, strength, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, 1.0, ?5, NULL)",
            params![&rel_id, &source_id, &target_id, &rel.kind, &now],
        ).map_err(|e| format!("Failed to insert relationship: {}", e))?;
    }
    
    // Mark job as completed
    tx.execute(
        "UPDATE scan_jobs SET status = 'completed', completed_at = unixepoch() WHERE id = ?1",
        params![job_id],
    ).map_err(|e| e.to_string())?;
    
    // Commit transaction
    tx.commit().map_err(|e| e.to_string())?;
    
    Ok(())
}

fn find_entity_id(
    conn: &Connection,
    project_id: &str,
    name: &str,
) -> Result<Option<String>, String> {
    // First try exact match
    let result: Option<String> = conn.query_row(
        "SELECT id FROM entities WHERE project_id = ?1 AND name = ?2 LIMIT 1",
        params![project_id, name],
        |row| row.get(0),
    ).ok();
    
    if result.is_some() {
        return Ok(result);
    }
    
    // Try with file extension stripped
    let name_without_ext = name.split('.').next().unwrap_or(name);
    if name_without_ext != name {
        let result: Option<String> = conn.query_row(
            "SELECT id FROM entities WHERE project_id = ?1 AND name = ?2 LIMIT 1",
            params![project_id, name_without_ext],
            |row| row.get(0),
        ).ok();
        
        return Ok(result);
    }
    
    Ok(None)
}

pub async fn process_file_change(
    project_id: &str,
    file_path: &str,
    state: &Arc<Mutex<Connection>>,
    app_handle: &AppHandle,
) -> Result<(), String> {
    // Check if file is parseable
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    if !["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx", "py", "rs", "go", "java"].contains(&ext) {
        return Ok(()); // Skip non-parseable files
    }
    
    // Read file and compute hash
    let content = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let current_hash = compute_hash(&content);
    
    // Check if hash differs from stored
    let conn = state.lock().await;
    let stored_hash: Option<String> = conn.query_row(
        "SELECT content_hash FROM scan_jobs WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path],
        |row| row.get(0),
    ).ok();
    
    if stored_hash.as_ref() == Some(&current_hash) {
        return Ok(()); // No change, skip
    }
    
    drop(conn); // Release lock before parsing
    
    // Get existing entities for this file
    let old_entities: Vec<(String, String, String)> = {
        let conn = state.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, type FROM entities WHERE project_id = ?1 AND file_path = ?2"
        ).map_err(|e| e.to_string())?;
        
        let rows: Vec<(String, String, String)> = stmt
            .query_map(params![project_id, file_path], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            }).map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        
        rows
    };
    
    // Parse the file
    let parsed = parser::parse_file(file_path).await?;
    
    // Compute changes
    let mut added = 0;
    let mut removed = 0;
    let mut changed = 0;
    
    // Build map of old entities
    let old_map: std::collections::HashMap<(String, String), String> = old_entities
        .into_iter()
        .map(|(id, name, kind)| ((name, kind), id))
        .collect();
    
    // Build map of new entities
    let mut new_map: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
    
    // Write changes to DB
    {
        let conn = state.lock().await;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        
        // Remove old entities
        tx.execute(
            "DELETE FROM entities WHERE project_id = ?1 AND file_path = ?2",
            params![project_id, file_path],
        ).map_err(|e| e.to_string())?;
        
        // Insert new entities and track changes
        for entity in &parsed.entities {
            let entity_id = Uuid::new_v4().to_string();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            
            let key = (entity.name.clone(), entity.kind.clone());
            
            if old_map.contains_key(&key) {
                // Entity exists, check if signature changed
                let old_id = old_map.get(&key).unwrap();
                let old_sig: Option<String> = conn.query_row(
                    "SELECT signature FROM entities WHERE id = ?1",
                    params![old_id],
                    |row| row.get(0),
                ).ok();
                
                if old_sig.as_ref() != Some(&entity.signature) {
                    // Signature changed
                    let event_id = Uuid::new_v4().to_string();
                    tx.execute(
                        "INSERT INTO events (id, project_id, entity_id, event_type, diff_summary, timestamp)
                         VALUES (?1, ?2, ?3, 'signature_changed', ?4, ?5)",
                        params![&event_id, &project_id, &entity_id, &format!("Signature changed for {}", entity.name), &now],
                    ).map_err(|e| e.to_string())?;
                    changed += 1;
                }
            } else {
                // Entity added
                let event_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO events (id, project_id, event_type, diff_summary, timestamp)
                     VALUES (?1, ?2, 'entity_added', ?3, ?4)",
                    params![&event_id, &project_id, &format!("Added {} {}", entity.kind, entity.name), &now],
                ).map_err(|e| e.to_string())?;
                added += 1;
            }
            
            new_map.insert(key, entity_id.clone());
            
            // Insert the entity
            tx.execute(
                "INSERT INTO entities (id, project_id, type, name, file_path, line_start, line_end, signature, is_public, metadata, created_at, modified_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    &entity_id,
                    &project_id,
                    &entity.kind,
                    &entity.name,
                    &file_path,
                    &entity.line_start,
                    &entity.line_end,
                    &entity.signature,
                    &entity.is_public,
                    &format!("{{\"language\": \"{}\"}}", parsed.language),
                    &now,
                    &now
                ],
            ).map_err(|e| e.to_string())?;
        }
        
    // Check for removed entities
    for ((name, kind), _entity_id) in &old_map {
            if !new_map.contains_key(&(name.clone(), kind.clone())) {
                // Entity removed
                let event_id = Uuid::new_v4().to_string();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                tx.execute(
                    "INSERT INTO events (id, project_id, event_type, diff_summary, timestamp)
                     VALUES (?1, ?2, 'entity_removed', ?3, ?4)",
                    params![&event_id, &project_id, &format!("Removed {} {}", kind, name), &now],
                ).map_err(|e| e.to_string())?;
                removed += 1;
            }
        }
        
        // Update content_hash in scan_jobs
        tx.execute(
            "INSERT INTO scan_jobs (id, project_id, file_path, status, content_hash, retry_count, created_at)
             VALUES (?1, ?2, ?3, 'completed', ?4, 0, unixepoch())
             ON CONFLICT(project_id, file_path) 
             DO UPDATE SET content_hash = excluded.content_hash",
            params![&Uuid::new_v4().to_string(), &project_id, &file_path, &current_hash],
        ).map_err(|e| e.to_string())?;
        
        tx.commit().map_err(|e| e.to_string())?;
    }
    
    // Emit graph updated event
    let payload = serde_json::json!({
        "project_id": project_id,
        "file_path": file_path,
        "added": added,
        "removed": removed,
        "changed": changed
    });
    
    if let Err(e) = app_handle.emit("knot://graph-updated", payload) {
        tracing::error!("Failed to emit graph updated: {}", e);
    }
    
    Ok(())
}
