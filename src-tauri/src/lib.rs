pub mod converter;

use std::fs;
use log::{debug, info, warn, error};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn convert_ohh_content(content: String) -> Result<String, String> {
    debug!("convert_ohh_content called with {} bytes", content.len());

    match converter::convert_ohh_file(&content) {
        Ok(result) => Ok(result),
        Err(e) => {
            error!("conversion failed: {}", e);
            Err(format!("conversion failed: {}", e))
        }
    }
}

#[tauri::command]
fn convert_ohh_file_path(file_path: String) -> Result<String, String> {
    use std::path::Path;

    debug!("convert_ohh_file_path called with: {}", file_path);

    // Validate file path
    let path = Path::new(&file_path);

    // Ensure the path is absolute and doesn't contain directory traversal
    let canonical = path
        .canonicalize()
        .map_err(|e| {
            error!("Failed to canonicalize path: {}", e);
            "Invalid file path or file does not exist".to_string()
        })?;

    debug!("Canonical path: {:?}", canonical);

    // Verify file extension
    if let Some(ext) = canonical.extension() {
        let ext_str = ext.to_str().unwrap_or("");
        debug!("File extension: {}", ext_str);
        if !matches!(ext_str, "ohh" | "txt" | "json") {
            let err = "Invalid file type. Only .ohh, .txt, or .json files are supported";
            error!("{}", err);
            return Err(err.to_string());
        }
    } else {
        let err = "File must have an extension";
        error!("{}", err);
        return Err(err.to_string());
    }

    // Check file size before reading (prevent DoS)
    let metadata =
        fs::metadata(&canonical).map_err(|e| {
            error!("Failed to get file metadata: {}", e);
            "Cannot access file".to_string()
        })?;

    debug!("File size: {} bytes", metadata.len());

    const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB
    if metadata.len() > MAX_FILE_SIZE {
        let err = format!(
            "File too large: {} MB (maximum 100 MB)",
            metadata.len() / 1024 / 1024
        );
        warn!("{}", err);
        return Err(err);
    }

    debug!("Reading file content");
    let content =
        fs::read_to_string(&canonical).map_err(|e| {
            error!("Failed to read file: {}", e);
            "Failed to read file".to_string()
        })?;

    debug!("Read {} bytes, starting conversion", content.len());
    match converter::convert_ohh_file(&content) {
        Ok(result) => {
            info!("File conversion successful, output size: {} bytes", result.len());
            Ok(result)
        }
        Err(e) => {
            error!("File conversion failed: {}", e);
            Err(e)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging to file
    use std::fs;
    use std::io::Write;

    let logs_dir = dirs::config_dir()
        .map(|d| d.join("pab-converter").join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));

    // Create logs directory if it doesn't exist
    let _ = fs::create_dir_all(&logs_dir);

    let log_file_path = logs_dir.join(format!(
        "pab-converter-{}.log",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));

    let file = match fs::File::create(&log_file_path) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("Failed to create log file: {}", e);
            None
        }
    };

    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let mut builder = env_logger::builder();
    builder
        .filter_level(level)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        });

    if let Some(f) = file {
        builder.target(env_logger::Target::Pipe(Box::new(f)));
    }

    builder.try_init().ok();

    info!("Starting PAB Converter v{}", env!("CARGO_PKG_VERSION"));
    info!("Debug mode: {}", cfg!(debug_assertions));
    info!("Log directory: {:?}", logs_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            convert_ohh_content,
            convert_ohh_file_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
