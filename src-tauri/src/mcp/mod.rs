pub mod tools;

use rusqlite::Connection;
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;

fn get_db_path() -> PathBuf {
    // Use platform-appropriate app data directory
    let app_dir = if cfg!(target_os = "windows") {
        dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
    } else {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
    };
    
    let knot_dir = app_dir.join("com.knot.app");
    std::fs::create_dir_all(&knot_dir).ok();
    knot_dir.join("knot.db")
}

pub async fn run_mcp_server(project_id: String) {
    // Open DB connection
    let db_path = get_db_path();
    let db = Connection::open(&db_path).unwrap_or_else(|e| {
        eprintln!("Failed to open database: {}", e);
        std::process::exit(1);
    });

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = std::io::BufReader::new(stdin);
    let mut writer = std::io::BufWriter::new(stdout);

    use std::io::BufRead;
    let mut lines = reader.lines();

    loop {
        match lines.next() {
            Some(Ok(line)) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let response = match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(request) => {
                        handle_request(&request, &project_id, &db).await
                    }
                    Err(e) => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32700,
                                "message": format!("Parse error: {}", e)
                            }
                        })
                    }
                };

                let mut response_str = serde_json::to_string(&response).unwrap();
                response_str.push('\n');
                if writer.write_all(response_str.as_bytes()).is_err() {
                    break;
                }
                if writer.flush().is_err() {
                    break;
                }
            }
            Some(Err(_)) => break,
            None => break, // EOF
        }
    }
}

async fn handle_request(
    request: &serde_json::Value,
    project_id: &str,
    db: &Connection,
) -> serde_json::Value {
    let method = request.get("method").and_then(|m| m.as_str());
    let id = request.get("id").cloned().unwrap_or(json!(null));

    match method {
        Some("initialize") => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "knot",
                        "version": "0.1.0"
                    }
                }
            })
        }
        Some("tools/list") => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "get_context_pack",
                            "description": "Get relevant codebase context for a specific task. Returns entities, relationships, and decisions most relevant to the task description.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task": {
                                        "type": "string",
                                        "description": "Description of what you are trying to implement or fix"
                                    },
                                    "max_entities": {
                                        "type": "number",
                                        "description": "Maximum entities to return (default 20)"
                                    }
                                },
                                "required": ["task"]
                            }
                        },
                        {
                            "name": "query_graph",
                            "description": "Search the codebase knowledge graph for entities matching a query.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "kind": {
                                        "type": "string",
                                        "enum": ["function", "class", "interface", "import", "all"],
                                        "description": "Filter by entity kind"
                                    }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "get_entity",
                            "description": "Get full details for a specific entity including all relationships.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string",
                                        "description": "Entity name to look up"
                                    }
                                },
                                "required": ["name"]
                            }
                        },
                        {
                            "name": "get_callers",
                            "description": "Find all entities that call or import a specific function or module.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string" }
                                },
                                "required": ["name"]
                            }
                        },
                        {
                            "name": "get_recent_changes",
                            "description": "Get entities that changed recently, useful for understanding what has been modified.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "since_minutes": {
                                        "type": "number",
                                        "description": "How many minutes back to look (default 60)"
                                    }
                                }
                            }
                        },
                        {
                            "name": "get_resume_context",
                            "description": "Get full resume context for handing off work to another agent. Use this when starting a session to understand where work left off.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "record_decision",
                            "description": "Record an architectural decision for permanent storage in the project knowledge base.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "rationale": { "type": "string" },
                                    "alternatives": { "type": "string" }
                                },
                                "required": ["title", "rationale"]
                            }
                        },
                        {
                            "name": "record_progress",
                            "description": "Update the current task progress log. Call this when completing a significant step.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "description": {
                                        "type": "string",
                                        "description": "What was just completed or what is currently in progress"
                                    }
                                },
                                "required": ["description"]
                            }
                        }
                    ]
                }
            })
        }
        Some("tools/call") => {
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            match tools::handle_tool_call(name, arguments, project_id, db).await {
                Ok(result) => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": result
                                }
                            ]
                        }
                    })
                }
                Err(e) => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": e
                        }
                    })
                }
            }
        }
        Some(method) => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })
        }
        None => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32600,
                    "message": "Invalid request: missing method"
                }
            })
        }
    }
}
