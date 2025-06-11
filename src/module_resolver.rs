use std::{
    collections::{HashMap, HashSet},
    env,
    fs,
    path::{Component, Path, PathBuf}
};
use walkdir::WalkDir;
use anyhow::Result;

/// collect
pub fn collect_ts_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter(|entry| {
            matches!(
                entry.path().extension().and_then(|s| s.to_str()),
                Some("ts") | Some("tsx")
            )
        })
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// extract import
pub fn extract_imports(source: &str) -> Vec<String> {
    let import_re = regex::Regex::new(r#"(?m)^\s*import\s.*?['"]([^'"]+)['"]"#).unwrap();
    import_re
        .captures_iter(source)
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

/// resolve import path
pub fn resolve_path(current: &Path, import: &str) -> Option<PathBuf> {
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

/// build
pub fn build_from_root(
    file: &Path,
    graph: &mut HashMap<PathBuf, Vec<PathBuf>>,
    visited: &mut HashSet<PathBuf>,
) {
    let file = match file.canonicalize() {
        Ok(f) => f,
        Err(_) => return,
    };

    if visited.contains(&file) {
        return;
    }
    visited.insert(file.clone());

    graph.entry(file.clone()).or_insert_with(Vec::new);

    let code = fs::read_to_string(&file).unwrap_or_default();
    let imports = extract_imports(&code);

    for import in imports {
        if let Some(resolved) = resolve_path(&file, &import) {
            graph.entry(file.clone()).or_default().push(resolved.clone());
            build_from_root(&resolved, graph, visited);
        }
    }
}

pub fn print_dep_graph(root_dir: &Path) {
    let mut graph = HashMap::new();
    let mut visited = HashSet::new();

    let all_files = collect_ts_files(root_dir);

    println!("all files {:?}", all_files);

    for file in &all_files {
        build_from_root(file, &mut graph, &mut visited);
    }

     println!("graph {:?}", graph);

    for (file, deps) in graph {
        println!("{}:", file.display());
        for dep in deps {
            println!("  → {}", dep.display());
        }
    }
}

pub fn copy_graph_files(
    root_dir: &Path,
    to: &Path,
) -> std::io::Result<()> {
    let mut graph = HashMap::new();
    let mut visited = HashSet::new();
    let entry = root_dir.join("components");

    println!("{:?}", entry);

    let all_files = collect_ts_files(&entry);

    for file in &all_files {
        build_from_root(file, &mut graph, &mut visited);
    }

    let mut all = HashSet::new();

    for (key, deps) in graph {
        all.insert(key.clone());
        for dep in deps {
            all.insert(dep.clone());
        }
    }

    println!("{:?}", all);

    for file in all {
        // compute file resolve from path
        let rel_path = file.strip_prefix(root_dir).unwrap_or(&file);
        let target_path = to.join(rel_path);

        // create paraent dir
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // copy file
        fs::copy(&file, &target_path)?;
        // println!("Copied: {} → {}", file.display(), target_path.display());
    }

    Ok(())
}
