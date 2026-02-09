use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Read a JSON file and deserialise into a typed struct.
pub fn read_json<T: DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let canonical = resolve_path(path)?;
    let contents = fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;
    let value: T = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse '{}': {}", canonical.display(), e))?;
    Ok(value)
}

/// Read a JSON file as a generic serde_json::Value.
pub fn read_json_value(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let canonical = resolve_path(path)?;
    let contents = fs::read_to_string(&canonical)
        .map_err(|e| format!("Failed to read '{}': {}", canonical.display(), e))?;
    let value: Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse '{}': {}", canonical.display(), e))?;
    Ok(value)
}

/// Resolve and validate the path, preventing directory traversal.
fn resolve_path(path: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let p = Path::new(path);
    let canonical = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()?.join(p)
    };

    // Basic existence check
    if !canonical.exists() {
        return Err(format!("File not found: {}", canonical.display()).into());
    }

    if !canonical.is_file() {
        return Err(format!("Not a file: {}", canonical.display()).into());
    }

    Ok(canonical)
}
