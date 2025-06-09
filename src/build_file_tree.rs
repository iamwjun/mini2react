use std::collections::HashMap;

#[derive(Debug)]
/// 定义文件树结构
pub struct FileNode {
    name: String,
    is_file: bool,
    path: String,
    children: HashMap<String, FileNode>,
}

/// 继承文件树结构
/// 定义遍历文件树方法
/// 生成目标文件
impl FileNode {
    fn new(name: &str, is_file: bool, path: &str) -> Self {
        FileNode {
            name: name.to_string(),
            is_file,
            path: path.to_string(),
            children: HashMap::new(),
        }
    }

    fn add_child(&mut self, child: FileNode) {
        self.children.insert(child.name.clone(), child);
    }

    pub fn traverse(&self) {
        self.traverse_recursive(0);
    }

    pub fn traverse_recursive(&self, depth: usize) {
        if !self.is_file {
            if depth == 0 {
                utils::create_or_replace_folder("widgets/");
            } else {
                utils::create_or_replace_folder(&format!("widgets/{}", &utils::capitalize_first_letter(&self.name)));
            }
        }

        // 递归遍历子节点
        for child in self.children.values() {
            child.traverse_recursive(depth + 1);
        }
    }
}

pub mod utils {
    use crate::build_file_tree::FileNode;
    use std::{
        env, fs::{self, File}, io::Write, path::PathBuf
    };

    /// &str 首字母大写
    pub fn capitalize_first_letter(s: &str) -> String {
        if let Some(ch) = s.chars().next() {
            let first_letter = ch.to_uppercase().collect::<String>();
            let rest_of_string = s.chars().skip(1).collect::<String>();
            format!("{}{}", first_letter, rest_of_string)
        } else {
            s.to_string()
        }
    }

    /// 遍历文件夹目录结构
    /// 构建文件树
    pub fn build_file_tree(path: &str) -> FileNode {
        let path_buf = PathBuf::from(path);
        let name = path_buf.file_name().unwrap().to_str().unwrap();
        let mut root = FileNode::new(name, false, path);

        if path_buf.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let entry_path = entry.path();
                        let entry_name = entry_path.file_name().unwrap().to_str().unwrap();
                        if entry_path.is_dir() {
                            let child = build_file_tree(entry_path.to_str().unwrap());
                            root.add_child(child);
                        } else {
                            let child =
                                FileNode::new(entry_name, true, entry_path.to_str().unwrap());
                            root.add_child(child);
                        }
                    }
                }
            }
        }

        root
    }

    /// 创建或替换文件
    pub fn create_or_replace_folder(folder_name: &str) {
        // 尝试删除已有文件夹
        if let Err(err) = fs::remove_dir_all(folder_name) {
            // 如果该文件夹不存在，则返回 Err，但我们可以忽略该错误
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Error removing folder '{}': {}", folder_name, err);
                return;
            }
        }

        // 使用create_dir创建文件夹
        match fs::create_dir(folder_name) {
            Ok(_) => println!("Folder '{}' created successfully.", folder_name),
            Err(err) => eprintln!("Error creating folder '{}': {}", folder_name, err),
        }
    }

    pub fn get_executable_path() -> Option<String> {
        if let Ok(exe_path) = env::current_exe() {
            exe_path.to_str().map(String::from)
        } else {
            None
        }
    }

    /// 生成 TypeScript 默认导出文件
    pub fn generate_typescript_default_export(
        content: &str,
        file_path: &str,
    ) -> std::io::Result<()> {
        let mut file = File::create(file_path)?;
        writeln!(file, "export {};", content)?;

        Ok(())
    }

    /// 生成 React 函数组件示例代码
    pub fn generate_react_function_component(
        component_name: &str,
        file_path: &str,
    ) -> std::io::Result<()> {
        let component_code = format!(
            r#"
import React from 'react';
        
// Example React function component
const {} = () => {{
    return (
        <div>
        <h1>{}</h1>
        <p>This is an example React function component.</p>
        </div>
    );
}};
        
export default {};
        "#,
            component_name, component_name, component_name
        );

        let mut file = File::create(file_path)?;
        writeln!(file, "{}", component_code)?;

        Ok(())
    }
}
