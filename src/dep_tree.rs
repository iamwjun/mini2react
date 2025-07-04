use std::collections::{HashMap, HashSet};
use std::{env, fs};
use std::path::{Component, Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;
use anyhow::Result;

#[derive(Debug)]
pub struct DepGraph {
    pub graph: HashMap<PathBuf, HashSet<PathBuf>>,
    pub reverse_graph: HashMap<PathBuf, HashSet<PathBuf>>,
    pub all_files: HashSet<PathBuf>,
}

impl DepGraph {
    pub fn build_from_root(root: &Path) -> Result<Self> {
        let mut all_files = HashSet::new();
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                let path = e.path();
                path.is_file()
                    && path.extension().map(|ext| ext == "ts" || ext == "tsx").unwrap_or(false)
            })
        {
            all_files.insert(entry.path().canonicalize()?);
        }

        let mut graph: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
        let mut reverse_graph: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

        for file in &all_files {
            let code = fs::read_to_string(file)?;
            let imports = extract_imports(&code);
            for import in imports {
                if let Some(resolved) = resolve_path(file, &import) {
                    if !all_files.contains(&resolved) {
                        graph.entry(file.clone()).or_default().insert(resolved.clone());
                        reverse_graph.entry(resolved).or_default().insert(file.clone());
                    }
                }
            }
        }

        Ok(Self {
            graph,
            reverse_graph,
            all_files,
        })
    }

    pub fn find_roots(&self) -> Vec<PathBuf> {
        self.all_files
            .iter()
            .filter(|file| !self.reverse_graph.contains_key(*file))
            .cloned()
            .collect()
    }

    pub fn build_tree(&self, file: &PathBuf, visited: &mut HashSet<PathBuf>) -> serde_json::Value {
        if visited.contains(file) {
            return serde_json::json!("(cycle)");
        }
        visited.insert(file.clone());

        let mut map = serde_json::Map::new();
        if let Some(children) = self.graph.get(file) {
            for child in children {
                let sub_tree = self.build_tree(child, visited);
                map.insert(child.to_string_lossy().into(), sub_tree);
            }
        }

        serde_json::Value::Object(map)
    }
}

fn extract_imports(code: &str) -> Vec<String> {
    let re = Regex::new(r#"(?m)^\s*(?:import|export).*?from\s+['"](.+?)['"]"#).unwrap();
    re.captures_iter(code)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut result = PathBuf::new();

    let full_path = if path.as_ref().is_absolute() {
        path.as_ref().to_path_buf()
    } else {
        env::current_dir().unwrap().join(path)
    };

    for component in full_path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {
                // skep
            }
            other => result.push(other),
        }
    }

    result
}

fn resolve_path(current: &Path, import: &str) -> Option<PathBuf> {
    if !import.starts_with('.') {
        return None;
    }

    let base = current.parent()?;
    let raw = base.join(import);

    let candidates = [
        raw.clone(),
        raw.with_extension("ts"),
        raw.with_extension("tsx"),
        raw.join("index.ts"),
        raw.join("index.tsx"),
    ];

    for path in candidates {
        let resolved = normalize_path(path);
        if let Ok(canon) = resolved.canonicalize() {
            if canon.is_file() {
                return Some(canon);
            }
        }
    }

    None
}
