use mantra_lang_tracing::collect::{collector::AnnotationCollector, rust::RustCodeCollector};
use tree_sitter::Parser;

pub fn main() {
    // let content = r#"
    //     ["ID-1", "ID-2";231];
    //     "#;

    let content = r#"
        #[req_note("ID-1", "ID-2")]
        fn foo() {
            let i = 0;
        }

        /// some comment
        #[req_satisfied("ID-2")]
        trait MyTrait {}
        "#;

    // let mut parser = Parser::new();
    // parser
    //     .set_language(&tree_sitter_rust::LANGUAGE.into())
    //     .unwrap();
    // let tree = parser
    //     .parse(content, None)
    //     .ok_or(anyhow::anyhow!("Failed to parse Rust code"))
    //     .unwrap();
    // let mut cursor = tree.walk();

    // cursor.goto_first_child();
    // let node = cursor.node();
    // println!("kind='{}' node='{:?}'", node.kind(), node);

    // let ident = node.child_by_field_name("name").unwrap();
    // println!("kind='{}' node='{:?}'", ident.kind(), ident);

    // // cursor.goto_next_sibling();
    // // let node = cursor.node();
    // // println!("kind='{}' node='{:?}'", node.kind(), node);

    // // for child in node.children(&mut cursor) {
    // //     println!("kind='{}' node='{:?}'", child.kind(), child);
    // // }

    let annotations = RustCodeCollector::collect(content).unwrap();

    println!("{annotations:?}");
}
