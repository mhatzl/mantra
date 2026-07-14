use comrak::{Arena, nodes::NodeValue};
use mantra_schema::{FmtHash, path::RelativePathBuf, product::ProductId};

use crate::cmd::collect::{collector::CollectableFile, requirements::markup::markdown};

macro_rules! test_file {
    ($f:literal) => {{
        let content = include_str!($f);

        CollectableFile {
            filepath: RelativePathBuf::from($f),
            file_hash: FmtHash::new(content),
            content,
        }
    }};
}

macro_rules! expect_schema {
    ($f:literal) => {
        expect_schema!($f, "product-id")
    };
    ($f:literal, $pid:literal) => {{
        let schema = markdown::collect_requirements(
            &ProductId::new(String::from($pid)).unwrap(),
            &test_file!($f),
        )
        .unwrap()
        .unwrap();

        schema
    }};
}

#[test]
fn markdown_single_files() {
    let arena = Arena::default();
    let mut options = comrak::Options::default();
    options.extension.front_matter_delimiter = Some("---".to_owned());

    let root_node =
        comrak::parse_document(&arena, include_str!("mantra_known_fields.md"), &options);

    if root_node.data().value != NodeValue::Document {
        panic!("Expected Markdown root to be a document");
    }

    let top_nodes = root_node.children().peekable();

    for child in top_nodes {
        eprintln!("{child:#?}");
    }
}

#[test]
fn markdown_single_requirement() {
    let schema = expect_schema!("mantra_known_fields.md");

    eprintln!("{schema:#?}");
}

#[test]
fn markdown_extension_detection() {
    let filepath = RelativePathBuf::from("file.md");
    let media_type = mime_guess::from_ext(filepath.extension().unwrap_or_default()).first_raw();

    assert_eq!(media_type, Some("text/markdown"));

    let filepath = RelativePathBuf::from("file.markdown");
    let media_type = mime_guess::from_ext(filepath.extension().unwrap_or_default()).first_raw();

    assert_eq!(media_type, Some("text/markdown"));
}
