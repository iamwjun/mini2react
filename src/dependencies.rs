use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use regex::Regex;
use anyhow::{Result, Context};

fn extract_imports(code: &str) -> Vec<String> {
    let re = Regex::new(r#"(?m)^\s*(?:import|export).*?from\s+['"](.+?)['"]"#).unwrap();
    re.captures_iter(code)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn resolve_path(current: &Path, import: &str) -> Option<PathBuf> {
    if !import.starts_with('.') {
        return None; // 非本地路径跳过
    }

    let base = current.parent()?;
    let mut full = base.join(import);

    if full.is_file() {
        return Some(full);
    }

    for ext in ["ts", "tsx"] {
        let try_path = full.with_extension(ext);
        if try_path.is_file() {
            return Some(try_path);
        }

        let index_path = full.join(format!("index.{}", ext));
        if index_path.is_file() {
            return Some(index_path);
        }
    }

    None
}

pub fn build_tree(
    file: &Path,
    all_files: &HashSet<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) -> Result<serde_json::Value> {
    if visited.contains(file) {
        return Ok(serde_json::json!("(cycle)"));
    }
    visited.insert(file.to_path_buf());

    let code = fs::read_to_string(file)
        .with_context(|| format!("Failed to read {:?}", file))?;
    let imports = extract_imports(&code);

    let mut map = serde_json::Map::new();
    for import in imports {
        if let Some(resolved) = resolve_path(file, &import) {
            if all_files.contains(&resolved) {
                let child = build_tree(&resolved, all_files, visited)?;
                map.insert(resolved.to_string_lossy().into(), child);
            }
        }
    }
    Ok(serde_json::Value::Object(map))
}
