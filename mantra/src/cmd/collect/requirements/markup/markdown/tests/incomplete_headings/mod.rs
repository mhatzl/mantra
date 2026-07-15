use mantra_schema::product::ProductId;

use crate::cmd::collect::requirements::markup::markdown;

#[test]
fn no_title() {
    let file = super::test_file!("no_title.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    for err_msg in schema_err.chain() {
        if err_msg.to_string() == "Missing title after ':' for the requirement definition" {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn no_separator() {
    let file = super::test_file!("no_separator.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    for err_msg in schema_err.chain() {
        if err_msg.to_string() == "Missing title separator ':' after requirement ID" {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}
