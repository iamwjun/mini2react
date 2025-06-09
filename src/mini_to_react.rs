use std::{collections::HashSet, fs, io::Cursor, path::{Path, PathBuf}};
use serde_json::Value;
use regex::Regex;
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::default::Default;
use std::fs::create_dir_all;
use std::fs::write;

#[derive(Debug)]
pub enum DependencyType {
    Component(PathBuf),
    Style(PathBuf),
    Script(PathBuf),
    Asset(PathBuf),
}

fn extract_json_components(json_str: &str, base: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    let parsed: Value = serde_json::from_str(json_str).unwrap_or(Value::Null);
    if let Some(components) = parsed.get("usingComponents") {
        if let Some(map) = components.as_object() {
            for (_k, v) in map.iter() {
                if let Some(rel_path) = v.as_str() {
                    let path = base.parent().unwrap().join(rel_path).join("index.json");
                    if path.exists() {
                        result.push(path);
                    }
                }
            }
        }
    }
    result
}

fn extract_style_imports(content: &str, base: &Path) -> Vec<PathBuf> {
    let mut deps = vec![];
    for line in content.lines() {
        if let Some(idx) = line.find("@import") {
            if let Some(start) = line[idx..].find('"') {
                if let Some(end) = line[idx + start + 1..].find('"') {
                    let path_str = &line[idx + start + 1..idx + start + 1 + end];
                    let dep = base.parent().unwrap().join(path_str).with_extension("less");
                    if dep.exists() {
                        deps.push(dep);
                    }
                }
            }
        }
    }
    deps
}

fn extract_script_imports(content: &str, base: &Path) -> Vec<PathBuf> {
    println!("extract_script_imports {:?}", base);
    let mut result = vec![];
    let re = Regex::new(r#"(import.*from\s+|require\()\s*[\"']([^\"']+)[\"']"#).unwrap();
    for cap in re.captures_iter(content) {
        let raw = cap[2].to_string();
        if raw.starts_with(".") {
            let p = base.parent().unwrap().join(&raw);
            for ext in ["js", "ts", "json"] {
                let candidate = p.with_extension(ext);
                if candidate.exists() {
                    result.push(candidate);
                    break;
                }
            }
        }
    }
    result
}

fn extract_import_sjs_paths(axml_path: &Path) -> Vec<PathBuf> {
    let content = fs::read_to_string(axml_path).unwrap_or_default();
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut content.as_bytes())
        .unwrap();

    let mut result = vec![];
    fn walk(node: &Handle, base: &Path, out: &mut Vec<PathBuf>) {
        if let NodeData::Element { ref name, ref attrs, .. } = node.data {
            if name.local.as_ref() == "import-sjs" {
                for attr in attrs.borrow().iter() {
                    if attr.name.local.as_ref() == "from" {
                        let sjs_path = base.parent().unwrap().join(attr.value.as_ref());
                        if sjs_path.exists() {
                            out.push(sjs_path);
                        }
                    }
                }
            }
        }
        for child in node.children.borrow().iter() {
            walk(child, base, out);
        }
    }

    walk(&dom.document, axml_path, &mut result);
    result
}

pub fn collect_all_dependencies(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    deps: &mut Vec<DependencyType>
) {
    if !visited.insert(path.to_path_buf()) {
        return;
    }
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let content = fs::read_to_string(path).unwrap_or_default();
        match ext {
            "json" => {
                for dep in extract_json_components(&content, path) {
                    collect_all_dependencies(&dep, visited, deps);
                    deps.push(DependencyType::Component(dep));
                }
            },
            "less" | "acss" => {
                for dep in extract_style_imports(&content, path) {
                    collect_all_dependencies(&dep, visited, deps);
                    deps.push(DependencyType::Style(dep));
                }
            },
            "js" | "ts" => {
                for dep in extract_script_imports(&content, path) {
                    collect_all_dependencies(&dep, visited, deps);
                    deps.push(DependencyType::Script(dep));
                }
            },
            "axml" => {
                for dep in extract_import_sjs_paths(&path) {
                    collect_all_dependencies(&dep, visited, deps);
                    deps.push(DependencyType::Script(dep));
                }
            },
            _ => {}
        }
    }
}

pub fn scan_component_dirs(base_dir: &Path) -> Vec<PathBuf> {
    let mut result = vec![];
    if let Ok(entries) = fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let json_path = path.join("index.json");
                if json_path.exists() {
                    result.push(json_path);
                }
            }
        }
    }
    result
}

pub fn copy_dependency(dep: &PathBuf, source_root: &Path, target_root: &Path) {
    if let Ok(rel_path) = dep.strip_prefix(source_root) {
        let target_path = target_root.join(rel_path);
        if let Some(parent) = target_path.parent() {
            let _ = create_dir_all(parent);
        }
        if let Err(e) = fs::copy(dep, &target_path) {
            eprintln!("Failed to copy {:?} to {:?}: {}", dep, target_path, e);
        } else {
            // println!("✔ Copied {:?} -> {:?}", dep, target_path);

            if let Some(ext) = dep.extension().and_then(|s| s.to_str()) {
                if ext == "axml" {
                    let jsx = convert_axml_to_jsx(dep);
                    let mut jsx_path = target_path.clone();
                    jsx_path.set_extension("tsx");
                    let _ = write(&jsx_path, jsx);
                    // println!("✔ Converted AXML to JSX: {:?}", jsx_path);
                }
            }
        }
    }
}

fn extract_methods_from_script(script_path: &Path) -> Vec<String> {
    let content = fs::read_to_string(script_path).unwrap_or_default();
    let mut methods = vec![];
    let re = Regex::new(r#"methods\s*:\s*\{\s*((.|\n)*?)\s*\}"#).unwrap();

    if let Some(cap) = re.captures(&content) {
        let body = &cap[1];
        let func_re = Regex::new(r#"(?m)^\s*(\w+)\s*\((.*?)\)\s*\{\s*((.|\n)*?)\n\s*\}"#).unwrap();
        for cap in func_re.captures_iter(body) {
            let name = &cap[1];
            let args = &cap[2];
            let body = &cap[3];
            let func = format!("function {}({}) {{\n{}\n}}", name, args, body.trim());
            methods.push(func);
        }
    }

    methods
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = true;
    for c in s.chars() {
        if c == '-' || c == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            result.push(c.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

fn convert_axml_to_jsx(axml_path: &Path) -> String {
    let axml_content = fs::read_to_string(axml_path).unwrap_or_default();
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut Cursor::new(axml_content.as_bytes()))
        .unwrap();

    let mut events = HashSet::new();

    fn convert_style(style: &str) -> String {
        let mut result = String::from("{{");
        for part in style.split(';') {
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim().replace("-", "");
                let value = value.trim();
                if !key.is_empty() && !value.is_empty() {
                    result.push_str(&format!(r#"{}: \"{}\", "#, key, value));
                }
            }
        }
        result.push_str("}}");
        result
    }

    fn convert_attr(name: &str, value: &str, events: &mut HashSet<String>) -> Option<(String, String)> {
        match name {
            "class" => Some(("className".to_string(), format!(r#"\"{}\""#, value))),
            "style" => Some(("style".to_string(), convert_style(value))),
            s if s.starts_with("on") => {
                let event = match &s[2..] {
                    "tap" => "onClick",
                    other => &format!("on{}", other),
                };
                events.insert(value.to_string());
                Some((event.to_string(), format!("{{{}}}", value)))
            }
            _ => Some((name.to_string(), format!(r#"\"{}\""#, value))),
        }
    }

    fn convert_tag(tag: &str) -> &str {
        match tag {
            "view" => "div",
            "text" => "span",
            "image" => "img",
            _ => tag,
        }
    }

    fn walk(node: &Handle, indent: usize, out: &mut String, events: &mut HashSet<String>) {
        match &node.data {
            NodeData::Text { contents } => {
                let text = contents.borrow();
                let text = text.trim();
                if !text.is_empty() {
                    out.push_str(&format!("{}{}\n", " ".repeat(indent), text));
                }
            }
            NodeData::Element { name, attrs, .. } => {
                let tag_name = name.local.as_ref();
                if tag_name == "import-sjs" {
                    return;
                }
                let jsx_tag = convert_tag(tag_name);
                let mut props = vec![];
                for attr in attrs.borrow().iter() {
                    if let Some((k, v)) = convert_attr(attr.name.local.as_ref(), &attr.value, events) {
                        props.push(format!("{}={} ", k, v));
                    }
                }
                let children = &node.children.borrow();
                let has_children = children.iter().any(|c| matches!(c.data, NodeData::Text { .. } | NodeData::Element { .. }));
                let indent_str = " ".repeat(indent);
                if has_children {
                    out.push_str(&format!("{}<{} {}>\n", indent_str, jsx_tag, props.concat()));
                    for child in children.iter() {
                        walk(child, indent + 2, out, events);
                    }
                    out.push_str(&format!("{}</{}>\n", indent_str, jsx_tag));
                } else {
                    out.push_str(&format!("{}<{} {}/>\n", indent_str, jsx_tag, props.concat()));
                }
            }
            _ => {}
        }
    }

    let mut jsx = String::new();
    for child in dom.document.children.borrow().iter() {
        walk(child, 2, &mut jsx, &mut events);
    }

    let component_name = axml_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Component");

    let script_path = axml_path.with_file_name("index.js");
    let ts_script_path = axml_path.with_file_name("index.ts");
    let method_functions = if script_path.exists() {
        extract_methods_from_script(&script_path)
    } else if ts_script_path.exists() {
        extract_methods_from_script(&ts_script_path)
    } else {
        vec![]
    };

    let stub_funcs = events.iter()
        .filter(|e| !method_functions.iter().any(|m| m.contains(&format!("function {}", e))))
        .map(|e| format!("function {}(e) {{\n  // TODO: implement {}\n}}", e, e))
        .collect::<Vec<_>>();

    let all_functions = [method_functions, stub_funcs].concat().join("\n\n");

    format!(
        "import React from \"react\";

{}

export default function {}() {{
  return (
    <>\n{}    </>
  );
}}",
        all_functions,
        to_camel_case(component_name),
        jsx
    )
}
