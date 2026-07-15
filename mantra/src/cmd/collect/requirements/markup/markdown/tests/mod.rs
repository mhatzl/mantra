use mantra_schema::{path::RelativePathBuf, product::ProductId};

use crate::cmd::collect::requirements::markup::markdown;

mod incomplete_fields;
mod incomplete_headings;
mod multiple_products;

macro_rules! test_file {
    ($f:literal) => {{
        let content = include_str!($f);

        $crate::cmd::collect::collector::CollectableFile {
            filepath: mantra_schema::path::RelativePathBuf::from($f),
            file_hash: mantra_schema::FmtHash::new(content),
            content,
        }
    }};
}
use test_file;

macro_rules! expect_schema {
    ($f:literal) => {
        expect_schema!($f, "product-id")
    };
    ($f:literal, $pid:literal) => {{
        let file = test_file!($f);
        let schema =
            markdown::collect_requirements(&ProductId::new(String::from($pid)).unwrap(), &file)
                .unwrap()
                .unwrap();

        schema
    }};
}

#[test]
fn diff_product_frontmatter() {
    let file = test_file!("diff_product_frontmatter.md");
    let no_schema =
        markdown::collect_requirements(&ProductId::new(String::from("other-pid")).unwrap(), &file)
            .unwrap();

    assert_eq!(
        no_schema, None,
        "File is set for other product, so no requirement is collected"
    );
}

#[test]
fn dot_nested_requirements() {
    let schema = expect_schema!("dot_nested_requirements.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn known_and_custom_fields() {
    let schema = expect_schema!("known_and_custom_fields.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn mantra_known_fields() {
    let schema = expect_schema!("mantra_known_fields.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn matching_frontmatter() {
    let schema = expect_schema!("matching_frontmatter.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn multiple_requirements() {
    let schema = expect_schema!("multiple_requirements.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn parents_and_dot_combination() {
    let schema = expect_schema!("parents_and_dot_combination.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
}

#[test]
fn single_requirement() {
    let schema = expect_schema!("single_requirement.md");

    insta::with_settings!(
        {
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(schema);
        }
    );
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
