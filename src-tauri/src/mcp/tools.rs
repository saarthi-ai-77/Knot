use rusqlite::Connection;
use serde_json::json;
use uuid::Uuid;

pub async fn handle_tool_call(
    name: &str,
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    match name {
        "get_context_pack" => get_context_pack(arguments, project_id, db).await,
        "query_graph" => query_graph(arguments, project_id, db).await,
        "get_entity" => get_entity(arguments, project_id, db).await,
        "get_callers" => get_callers(arguments, project_id, db).await,
        "get_recent_changes" => get_recent_changes(arguments, project_id, db).await,
        "get_resume_context" => get_resume_context(project_id, db).await,
        "record_decision" => record_decision(arguments, project_id, db).await,
        "record_progress" => record_progress(arguments, project_id, db).await,
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

async fn get_context_pack(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let task = arguments
        .get("task")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let max_entities = arguments
        .get("max_entities")
        .and_then(|m| m.as_i64())
        .unwrap_or(20) as usize;

    if task.is_empty() {
        return Err("Task parameter is required".to_string());
    }

    // Extract keywords from task
    let keywords: Vec<&str> = task
        .split_whitespace()
        .filter(|w| !is_stopword(w))
        .collect();

    // Build FTS5 query
    let fts_query = if keywords.len() == 1 {
        format!("{}*", keywords[0])
    } else {
        keywords.join(" OR ")
    };

    // Search entities
    let mut entities: Vec<(String, String, String, i64, String, String)> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT e.id, e.name, e.type, e.line_start, e.signature, e.file_path
                 FROM entity_fts ef
                 JOIN entities e ON ef.rowid = e.rowid
                 WHERE ef.entity_fts MATCH ?1
                   AND e.project_id = ?2
                 ORDER BY rank
                 LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(
                rusqlite::params![fts_query, project_id, max_entities as i64],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                entities.push(r);
            }
        }
    }

    // Get relationships for each entity
    let mut relationships_info: Vec<String> = Vec::new();
    for (id, name, kind, _, _, _) in &entities {
        let mut rels: Vec<String> = Vec::new();
        {
            let mut stmt = db
                .prepare(
                    "SELECT DISTINCT e2.name, r.type
                     FROM relationships r
                     JOIN entities e2 ON r.target_id = e2.id
                     WHERE r.source_id = ?1
                     LIMIT 5",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(rusqlite::params![id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;

            for row in rows {
                if let Ok((target_name, rel_kind)) = row {
                    rels.push(format!("{} {}", rel_kind, target_name));
                }
            }
        }

        if !rels.is_empty() {
            relationships_info.push(format!("- **{}** ({}): {}", name, kind, rels.join(", ")));
        }
    }

    // Get recent changes
    let mut recent_changes: Vec<String> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT DISTINCT e.name, e.type, ev.event_type, ev.timestamp
                 FROM events ev
                 JOIN entities e ON ev.entity_id = e.id
                 WHERE ev.project_id = ?1
                   AND ev.timestamp > unixepoch() - 7200
                 ORDER BY ev.timestamp DESC
                 LIMIT 10",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![project_id], |row| {
                let name: String = row.get(0)?;
                let kind: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                let timestamp: i64 = row.get(3)?;
                let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_default();
                Ok(format!("- [{}] {} {} {}", time_str, event_type, kind, name))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                recent_changes.push(r);
            }
        }
    }

    // Get recent decisions
    let mut decisions: Vec<String> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT title, context FROM decisions
                 WHERE project_id = ?1
                 ORDER BY created_at DESC
                 LIMIT 5",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![project_id], |row| {
                let title: String = row.get(0)?;
                let context: String = row.get(1)?;
                Ok(format!("- **{}**: {}", title, &context[..context.len().min(100)]))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                decisions.push(r);
            }
        }
    }

    // Get total entity count
    let total_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE project_id = ?1",
            [project_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();

    let mut output = format!(
        "## Context Pack: {}\nGenerated by Knot at {}\n\n",
        task, timestamp
    );

    output.push_str(&format!("### Relevant Entities ({})\n\n", entities.len()));
    for (_id, name, kind, line_num, signature, file_path) in &entities {
        output.push_str(&format!(
            "**{}** ({}) — {}:{}\n```\n{}\n```\n\n",
            name, kind, file_path, line_num, signature.lines().next().unwrap_or(signature)
        ));
    }

    if !relationships_info.is_empty() {
        output.push_str("### Relationships\n");
        for rel in relationships_info {
            output.push_str(&rel);
            output.push('\n');
        }
        output.push('\n');
    }

    if !recent_changes.is_empty() {
        output.push_str("### Recent Changes (last 2 hours)\n");
        for change in &recent_changes {
            output.push_str(change);
            output.push('\n');
        }
        output.push('\n');
    }

    if !decisions.is_empty() {
        output.push_str("### Active Decisions\n");
        for decision in &decisions {
            output.push_str(decision);
            output.push('\n');
        }
        output.push('\n');
    }

    output.push_str("### Quick Reference\n");
    output.push_str(&format!("- Total entities indexed: {}\n", total_count));
    output.push_str("- Use `get_entity` for full details on any entity\n");
    output.push_str("- Use `get_callers` to find dependencies\n");

    Ok(output)
}

async fn query_graph(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let query = arguments.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let kind = arguments.get("kind").and_then(|k| k.as_str());

    if query.is_empty() {
        return Err("Query parameter is required".to_string());
    }

    let kind_clause = match kind {
        Some("all") | None => String::new(),
        Some(k) => format!("AND e.type = '{}'", k),
    };

    let sql = format!(
        "SELECT e.name, e.type, e.file_path, e.signature
         FROM entity_fts ef
         JOIN entities e ON ef.rowid = e.rowid
         WHERE ef.entity_fts MATCH ?1
           AND e.project_id = ?2
           {}
         ORDER BY rank
         LIMIT 30",
        kind_clause
    );

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;

    let fts_query = if query.contains(' ') {
        query.to_string()
    } else {
        format!("{}*", query)
    };

    let rows = stmt
        .query_map(rusqlite::params![fts_query, project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut output = format!("## Search Results for \"{}\"\n\n", query);

    let mut count = 0;
    for row in rows {
        if let Ok((name, kind, file_path, signature)) = row {
            output.push_str(&format!(
                "**{}** ({}) in {}\n```\n{}\n```\n\n---\n\n",
                name,
                kind,
                file_path.split('/').last().unwrap_or(&file_path),
                signature.lines().next().unwrap_or(&signature)
            ));
            count += 1;
        }
    }

    if count == 0 {
        output.push_str("No matching entities found.\n");
    } else {
        output.push_str(&format!("\nFound {} entities.\n", count));
    }

    Ok(output)
}

async fn get_entity(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let name = arguments.get("name").and_then(|n| n.as_str()).unwrap_or("");

    if name.is_empty() {
        return Err("Name parameter is required".to_string());
    }

    // Try exact match first
    let entity: Option<(String, String, String, i64, String, String)> = db
        .query_row(
            "SELECT id, name, type, line_start, signature, file_path
             FROM entities
             WHERE project_id = ?1 AND name = ?2
             LIMIT 1",
            rusqlite::params![project_id, name],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .ok();

    let (id, name, kind, line, signature, file_path) = match entity {
        Some(e) => e,
        None => {
            // Try LIKE match
            let pattern = format!("%{}%", name);
            let entity: Option<(String, String, String, i64, String, String)> = db
                .query_row(
                    "SELECT id, name, type, line_start, signature, file_path
                     FROM entities
                     WHERE project_id = ?1 AND name LIKE ?2
                     LIMIT 1",
                    rusqlite::params![project_id, pattern],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .ok();

            match entity {
                Some(e) => e,
                None => return Err(format!("Entity '{}' not found", name)),
            }
        }
    };

    // Get relationships
    let mut relationships: Vec<(String, String, String)> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT e2.name, r.type, CASE WHEN r.source_id = ?1 THEN 'outgoing' ELSE 'incoming' END as direction
                 FROM relationships r
                 JOIN entities e2 ON (CASE WHEN r.source_id = ?1 THEN r.target_id ELSE r.source_id END = e2.id)
                 WHERE r.source_id = ?1 OR r.target_id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![id, id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                relationships.push(r);
            }
        }
    }

    let mut output = format!(
        "## {} ({})\n\n**Location:** {}:{}\n\n**Signature:**\n```\n{}\n```\n\n",
        name, kind, file_path, line, signature
    );

    if !relationships.is_empty() {
        output.push_str("**Relationships:**\n");
        for (target, rel_kind, direction) in relationships {
            output.push_str(&format!("- {} {} {}\n", direction, rel_kind, target));
        }
    }

    Ok(output)
}

async fn get_callers(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let name = arguments.get("name").and_then(|n| n.as_str()).unwrap_or("");

    if name.is_empty() {
        return Err("Name parameter is required".to_string());
    }

    let mut stmt = db
        .prepare(
            "SELECT DISTINCT e.name, e.file_path, e.type
             FROM relationships r
             JOIN entities e ON r.source_id = e.id
             JOIN entities target ON r.target_id = target.id
             WHERE target.name = ?1
               AND r.project_id = ?2
               AND r.type IN ('calls', 'imports')",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![name, project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut output = format!("## Entities calling/importing \"{}\"\n\n", name);

    let mut count = 0;
    for row in rows {
        if let Ok((caller_name, file_path, kind)) = row {
            output.push_str(&format!(
                "- **{}** ({}) in {}\n",
                caller_name,
                kind,
                file_path.split('/').last().unwrap_or(&file_path)
            ));
            count += 1;
        }
    }

    if count == 0 {
        output.push_str("No callers found.\n");
    } else {
        output.push_str(&format!("\nFound {} callers.\n", count));
    }

    Ok(output)
}

async fn get_recent_changes(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let since_minutes = arguments
        .get("since_minutes")
        .and_then(|s| s.as_i64())
        .unwrap_or(60);

    let mut stmt = db
        .prepare(
            "SELECT DISTINCT e.name, e.type, e.file_path, ev.event_type, ev.timestamp
             FROM events ev
             JOIN entities e ON ev.entity_id = e.id
             WHERE ev.project_id = ?1
               AND ev.timestamp > unixepoch() - (?2 * 60)
             ORDER BY ev.timestamp DESC
             LIMIT 20",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![project_id, since_minutes], |row| {
            let name: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let file_path: String = row.get(2)?;
            let event_type: String = row.get(3)?;
            let timestamp: i64 = row.get(4)?;
            let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.format("%H:%M").to_string())
                .unwrap_or_default();
            Ok(format!(
                "- [{}] {} {} {} in {}",
                time_str,
                event_type,
                kind,
                name,
                file_path.split('/').last().unwrap_or(&file_path)
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut output = format!(
        "## Recent Changes (last {} minutes)\n\n",
        since_minutes
    );

    let mut count = 0;
    for row in rows {
        if let Ok(change) = row {
            output.push_str(&change);
            output.push('\n');
            count += 1;
        }
    }

    if count == 0 {
        output.push_str("No changes in the specified time period.\n");
    } else {
        output.push_str(&format!("\nFound {} changes.\n", count));
    }

    Ok(output)
}

async fn get_resume_context(project_id: &str, db: &Connection) -> Result<String, String> {
    // Get current task from most recent session
    let current_task: Option<String> = db
        .query_row(
            "SELECT current_task FROM agent_sessions
             WHERE project_id = ?1
             ORDER BY last_active_at DESC LIMIT 1",
            [project_id],
            |row| row.get(0),
        )
        .ok();

    // Get recent events
    let mut recent_changes: Vec<String> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT event_type, diff_summary, timestamp FROM events
                 WHERE project_id = ?1
                 ORDER BY timestamp DESC LIMIT 10",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![project_id], |row| {
                let event_type: String = row.get(0)?;
                let diff: String = row.get(1)?;
                let timestamp: i64 = row.get(2)?;
                let time_str = chrono::DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default();
                Ok(format!("- [{}] {}: {}", time_str, event_type, diff))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                recent_changes.push(r);
            }
        }
    }

    // Get top entities
    let mut top_entities: Vec<String> = Vec::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT e.name, e.type, COUNT(r.id) as connection_count
                 FROM entities e
                 LEFT JOIN relationships r ON e.id = r.source_id OR e.id = r.target_id
                 WHERE e.project_id = ?1
                 GROUP BY e.id
                 ORDER BY connection_count DESC
                 LIMIT 10",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(rusqlite::params![project_id], |row| {
                let name: String = row.get(0)?;
                let kind: String = row.get(1)?;
                let count: i64 = row.get(2)?;
                Ok(format!("- {} ({}): {} connections", name, kind, count))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row {
                top_entities.push(r);
            }
        }
    }

    let mut output = String::from("# Resume Context Prompt\n\n");

    if let Some(task) = current_task {
        output.push_str(&format!("## Current Task\n{}\n\n", task));
    }

    if !recent_changes.is_empty() {
        output.push_str("## Recent Changes\n");
        for change in recent_changes {
            output.push_str(&change);
            output.push('\n');
        }
        output.push('\n');
    }

    if !top_entities.is_empty() {
        output.push_str("## Key Entities\n");
        for entity in top_entities {
            output.push_str(&entity);
            output.push('\n');
        }
    }

    output.push_str("\n## Instructions\n");
    output.push_str(
        "1. Review the current task and recent changes\n",
    );
    output.push_str("2. Understand the key entities involved\n");
    output.push_str("3. Continue from where the previous session left off\n");

    Ok(output)
}

async fn record_decision(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let title = arguments
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    let rationale = arguments
        .get("rationale")
        .and_then(|r| r.as_str())
        .unwrap_or("");
    let alternatives = arguments
        .get("alternatives")
        .and_then(|a| a.as_str())
        .unwrap_or("");

    if title.is_empty() || rationale.is_empty() {
        return Err("Title and rationale are required".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    db.execute(
        "INSERT INTO decisions (id, project_id, title, context, alternatives_considered, created_at, status)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'proposed')",
        rusqlite::params![id, project_id, title, rationale, alternatives, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(format!("Decision recorded: {}", title))
}

async fn record_progress(
    arguments: serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> Result<String, String> {
    let description = arguments
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("");

    if description.is_empty() {
        return Err("Description is required".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Insert as event
    db.execute(
        "INSERT INTO events (id, project_id, event_type, diff_summary, timestamp)
         VALUES (?1, ?2, 'task_logged', ?3, ?4)",
        rusqlite::params![id, project_id, description, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(format!("Progress logged: {}", description))
}

fn is_stopword(word: &str) -> bool {
    let stopwords = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "shall", "can", "need", "dare",
        "ought", "used", "to", "of", "in", "for", "on", "with", "at", "by",
        "from", "as", "into", "through", "during", "before", "after", "above",
        "below", "between", "under", "again", "further", "then", "once",
        "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not",
        "only", "own", "same", "so", "than", "too", "very", "just", "and",
        "but", "if", "or", "because", "until", "while", "this", "that",
        "these", "those", "i", "me", "my", "myself", "we", "our", "ours",
        "you", "your", "yours", "he", "him", "his", "she", "her", "hers",
        "it", "its", "they", "them", "their", "what", "which", "who", "whom",
    ];
    stopwords.contains(&word.to_lowercase().as_str())
}
