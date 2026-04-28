#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod mcp;
mod parser;
mod scanner;
mod watcher;

use rusqlite::Connection;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

fn init_db(app_dir: &PathBuf) -> Result<Connection, String> {
    let db_path = app_dir.join("knot.db");
    
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;
    
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
    
    // Run migrations
    let migrations = include_str!("../migrations/001_initial.sql");
    conn.execute_batch(migrations)
        .map_err(|e| format!("Failed to run migrations: {}", e))?;
    
    // Resume logic: mark interrupted scans as pending
    conn.execute(
        "UPDATE scan_jobs SET status = 'pending', retry_count = retry_count + 1 WHERE status = 'scanning'",
        [],
    ).map_err(|e| format!("Failed to resume scans: {}", e))?;
    
    // Delete permanently failed jobs
    conn.execute(
        "DELETE FROM scan_jobs WHERE retry_count > 3",
        [],
    ).map_err(|e| format!("Failed to clean failed scans: {}", e))?;
    
    tracing::info!("Database initialized at {:?}", db_path);
    Ok(conn)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Handle MCP server subcommand first
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "mcp-server" {
        // Extract --project-id=xxx from args
        let project_id = args
            .iter()
            .find(|a| a.starts_with("--project-id="))
            .and_then(|a| a.split('=').nth(1))
            .unwrap_or("default")
            .to_string();
        
        // Run MCP server on stdio, block until stdin closes
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(mcp::run_mcp_server(project_id));
        return;
    }
    
    tracing_subscriber::fmt::init();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            
            std::fs::create_dir_all(&app_dir)
                .map_err(|e| format!("Failed to create app dir: {}", e))?;
            
            let conn = init_db(&app_dir)?;
            let state = AppState {
                db: Arc::new(Mutex::new(conn)),
            };
            
            app.manage(state);
            
            // Initialize file watcher
            tauri::async_runtime::block_on(async {
                if let Err(e) = watcher::init_watcher(app.handle().clone()).await {
                    tracing::error!("Failed to initialize file watcher: {}", e);
                }
            });
            
            tracing::info!("Knot application started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Project commands
            commands::projects::create_project,
            commands::projects::get_projects,
            commands::projects::load_project,
            commands::projects::open_project,
            commands::projects::get_mcp_config,
            commands::projects::detect_ollama,
            commands::projects::save_ai_settings,
            commands::projects::test_ai_connection,
            // Dashboard stats
            commands::projects::get_entity_count,
            commands::projects::get_relationship_count,
            commands::projects::get_decision_count,
            commands::projects::get_session_count,
            commands::projects::get_recent_events,
            commands::projects::get_status_bar_info,
            // Graph commands
            commands::graph::parse_file,
            commands::graph::parse_project,
            commands::graph::get_scan_progress,
            commands::projects::query_entities,
            commands::projects::get_entity_detail,
            // Cost tracking
            commands::projects::get_cost_summary,
            commands::projects::get_cost_log,
            // Agent commands
            commands::agents::get_active_sessions,
            commands::agents::get_session_files,
            commands::agents::log_task,
            commands::agents::get_task_log,
            commands::agents::generate_resume_prompt,
            commands::agents::create_agent_session,
            commands::agents::update_session_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
