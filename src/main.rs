use mini2react::build_file_tree;

fn main() {
    let file_tree = build_file_tree("/Users/wujun/MiniProjects/demo/components");

    println!("{:?}", file_tree);

    file_tree.traverse();

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
}
