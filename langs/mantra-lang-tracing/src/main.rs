use mantra_lang_tracing::collect::{collector::AnnotationCollector, rust::RustCodeCollector};

/// Note: Used for experimentations only
pub fn main() {
    let content = r#"
        #[cfg_attr(
       	    feature = "some", req("ID-1"),
            cfg_attr(any(feature = "other", target = "none"),
            submod::req_note("ID-2"))
        )]
        fn foo() {}
        "#;

    let annotations = RustCodeCollector::collect(content).unwrap();

    println!("{annotations:?}");
}
