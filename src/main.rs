use std::{collections::HashSet, path::Path};
use anyhow::Result;

use mini2react::dependencies;
use walkdir::WalkDir;

fn main() -> Result<()> {
    // 构建依赖树
    let root = Path::new("/Users/wujun/MiniProjects/blank/components");
    let mut all_files = HashSet::new();

    // 1. 收集所有 ts/tsx 文件
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            let path = e.path();
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s == "ts" || s == "tsx")
                    .unwrap_or(false)
        })
    {
        all_files.insert(entry.into_path());
    }

    // 2. 每个文件作为潜在入口构建依赖树
    let mut forest = serde_json::Map::new();
    for file in &all_files {
        let mut visited = HashSet::new();
        let tree = dependencies::build_tree(file, &all_files, &mut visited)?;
        forest.insert(file.to_string_lossy().into(), tree);
    }

    // 3. 输出为 JSON
    let output = serde_json::to_string_pretty(&serde_json::Value::Object(forest))?;
    println!("{}", output);

    Ok(())

    // let file_tree = build_file_tree("/Users/wujun/MiniProjects/blank/components");

    // println!("{:?}", file_tree);

    // file_tree.traverse();

    // create_or_replace_folder("widgets");

    // let file_path = "widgets/index.tsx";

    // let content = r#"
    // // Your TypeScript content here
    // const myVariable: string = "Hello, TypeScript!";
    // console.log(myVariable);
    // "#;

    // match generate_typescript_default_export(content, file_path) {
    //     Ok(_) => println!("TypeScript file '{}' generated successfully.", file_path),
    //     Err(err) => eprintln!("Error generating TypeScript file: {}", err),
    // }

	// let _ = generate_react_function_component("Carousel", "widgets/Carousel.tsx");

    // 转换小程序组件和react组件
    // let base_dir = Path::new("/Users/wujun/MiniProjects/blank/components");
    // let target_dir = Path::new("/Users/wujun/Github/iamwjun/mini2react/react/components");
    // let entries = scan_component_dirs(base_dir);

    // for entry in entries {
    //     let mut visited = HashSet::new();
    //     let mut deps = vec![];

    //     collect_all_dependencies(&entry, &mut visited, &mut deps);


    //     for d in &deps {
    //         match d {
    //             DependencyType::Component(p) |
    //             DependencyType::Style(p) |
    //             DependencyType::Script(p) |
    //             DependencyType::Asset(p) => {
    //                 println!("  → {:?}", p);
    //                 copy_dependency(p, base_dir, target_dir);
    //             }
    //         }
    //     }

    //     // 复制入口自身文件夹
    //     if let Some(comp_dir) = entry.parent() {
    //         for ext in ["json", "axml", "js", "ts", "less", "acss"] {
    //             let file = comp_dir.join(format!("index.{}", ext));
    //             if file.exists() {
    //                 copy_dependency(&file, base_dir, target_dir);
    //             }
    //         }
    //     }
    // }
}
