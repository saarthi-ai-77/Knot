pub mod debouncer;

use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::AppState;
use crate::scanner;

pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, FileIdMap>,
    app_handle: AppHandle,
    project_id: String,
}

impl FileWatcher {
    pub async fn new(app_handle: AppHandle, project_id: String) -> Result<Self, String> {
        let app_handle_clone = app_handle.clone();
        let project_id_clone = project_id.clone();
        
        let debouncer = new_debouncer(
            Duration::from_millis(300),
            None,
            move |result: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            handle_file_event(&app_handle_clone, &event, &project_id_clone);
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            tracing::error!("File watcher error: {:?}", error);
                        }
                    }
                }
            },
        )
        .map_err(|e| format!("Failed to create debouncer: {}", e))?;
        
        Ok(FileWatcher {
            _debouncer: debouncer,
            app_handle,
            project_id,
        })
    }
    
    pub async fn watch(&mut self, path: &Path) -> Result<(), String> {
        self._debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;
        
        tracing::info!("Started watching path: {:?}", path);
        Ok(())
    }
}

fn handle_file_event(app_handle: &AppHandle, event: &DebouncedEvent, project_id: &str) {
    let paths: Vec<_> = event
        .paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    
    let change_type = match event.kind {
        notify::EventKind::Create(_) => "created",
        notify::EventKind::Modify(_) => "modified",
        notify::EventKind::Remove(_) => "deleted",
        _ => return, // Skip other events
    };
    
    for path in paths {
        // Skip .git and node_modules
        if path.contains("/.git/") || path.contains("/node_modules/") || 
           path.contains("\\.git\\") || path.contains("\\node_modules\\") {
            continue;
        }
        
        let payload = serde_json::json!({
            "file_path": path,
            "change_type": change_type,
        });
        
        // Use a valid event name (no colons or slashes)
        if let Err(e) = app_handle.emit("knot_file_changed", payload) {
            tracing::error!("Failed to emit file changed event: {}", e);
        } else {
            tracing::debug!("Emitted file changed event: {} ({})", path, change_type);
        }
        
        // Process file changes for parseable files
        if change_type == "modified" || change_type == "created" {
            let ext = Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            
            if !["js", "mjs", "cjs", "ts", "mts", "cts", "tsx", "jsx", "py", "rs", "go", "java"].contains(&ext) {
                continue;
            }
            
            // Get the app state and process the file change
            let app_handle_clone = app_handle.clone();
            let project_id = project_id.to_string();
            let path_clone = path.clone();
            
        tauri::async_runtime::spawn(async move {
            // Get state from the app handle
            let state = app_handle_clone.state::<AppState>();
            if let Err(e) = crate::scanner::process_file_change(
                &project_id,
                &path_clone,
                &state.db,
                &app_handle_clone,
            ).await {
                tracing::error!("Failed to process file change: {}", e);
            }
        });
        }
    }
}

// Global watchers storage
static WATCHERS: tokio::sync::OnceCell<Arc<Mutex<HashMap<String, FileWatcher>>>> = tokio::sync::OnceCell::const_new();

pub async fn init_watcher(_app_handle: AppHandle) -> Result<(), String> {
    WATCHERS
        .set(Arc::new(Mutex::new(HashMap::new())))
        .map_err(|_| "Watchers already initialized")?;
    
    Ok(())
}

pub async fn start_watching(project_path: &str) -> Result<(), String> {
    // Get or create project ID from path
    // For now, we'll use a hash of the path as the ID
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    project_path.hash(&mut hasher);
    let project_id = format!("{:x}", hasher.finish());
    
    if let Some(watchers) = WATCHERS.get() {
        let mut watchers = watchers.lock().await;
        
        // Check if already watching this project
        if watchers.contains_key(&project_id) {
            tracing::info!("Already watching project: {}", project_id);
            return Ok(());
        }
        
        // Get the app handle from any existing watcher or create new one
        let app_handle = watchers.values().next()
            .map(|w| w.app_handle.clone())
            .unwrap_or_else(|| {
                // This is a bit of a hack - we need the app handle
                // In practice, the first watcher should be created with the app handle
                panic!("Watcher not initialized properly")
            });
        
        let mut watcher = FileWatcher::new(app_handle, project_id.clone()).await?;
        watcher.watch(Path::new(project_path)).await?;
        
        watchers.insert(project_id, watcher);
    } else {
        return Err("Watcher not initialized".to_string());
    }
    
    Ok(())
}

pub async fn start_watching_with_handle(
    project_path: &str,
    project_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    if let Some(watchers) = WATCHERS.get() {
        let mut watchers = watchers.lock().await;
        
        // Check if already watching this project
        if watchers.contains_key(&project_id) {
            tracing::info!("Already watching project: {}", project_id);
            return Ok(());
        }
        
        let mut watcher = FileWatcher::new(app_handle, project_id.clone()).await?;
        watcher.watch(Path::new(project_path)).await?;
        
        watchers.insert(project_id, watcher);
    } else {
        return Err("Watcher not initialized".to_string());
    }
    
    Ok(())
}
