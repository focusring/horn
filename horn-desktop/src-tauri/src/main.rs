// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

/// Open a native file picker, validate all selected PDFs, return results.
#[tauri::command]
async fn pick_and_validate(app: tauri::AppHandle) -> serde_json::Value {
    eprintln!("[horn] pick_and_validate called");

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .file()
        .add_filter("PDF", &["pdf"])
        .set_title("Select PDF files")
        .pick_files(move |files| {
            eprintln!("[horn] dialog callback fired, files: {:?}", files.as_ref().map(|f| f.len()));
            let _ = tx.send(files);
        });

    eprintln!("[horn] waiting for dialog...");

    let Ok(Some(files)) = rx.await else {
        eprintln!("[horn] no files selected");
        return serde_json::json!([]);
    };

    eprintln!("[horn] got {} files", files.len());

    let validate_start = std::time::Instant::now();

    let mut reports = Vec::new();
    for file in &files {
        let Some(path) = file.as_path() else { continue };
        let path_str = path.to_string_lossy().to_string();
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                reports.push(serde_json::json!({
                    "path": path_str,
                    "standard": "unknown",
                    "results": [],
                    "error": format!("Failed to read file: {e}")
                }));
                continue;
            }
        };
        let name = path
            .file_name()
            .map_or("unknown".to_string(), |n| n.to_string_lossy().into_owned());
        let report = horn::validate_bytes(&name, data);
        reports.push(serde_json::to_value(&report).unwrap_or(serde_json::Value::Null));
    }

    eprintln!("[horn] validation took {:?}", validate_start.elapsed());
    serde_json::json!(reports)
}

/// Validate PDFs by file paths (used for drag & drop).
#[tauri::command]
fn validate_paths(paths: Vec<String>) -> serde_json::Value {
    let mut reports = Vec::new();
    for path_str in &paths {
        let path = std::path::Path::new(path_str);
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                reports.push(serde_json::json!({
                    "path": path_str,
                    "standard": "unknown",
                    "results": [],
                    "error": format!("Failed to read file: {e}")
                }));
                continue;
            }
        };
        let name = path
            .file_name()
            .map_or("unknown".to_string(), |n| n.to_string_lossy().into_owned());
        let report = horn::validate_bytes(&name, data);
        reports.push(serde_json::to_value(&report).unwrap_or(serde_json::Value::Null));
    }
    serde_json::json!(reports)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![pick_and_validate, validate_paths])
        .run(tauri::generate_context!())
        .expect("error while running Horn desktop app");
}
