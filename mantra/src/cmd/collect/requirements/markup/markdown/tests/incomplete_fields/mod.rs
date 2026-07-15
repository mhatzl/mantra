use mantra_schema::product::ProductId;

use crate::cmd::collect::requirements::markup::markdown;

#[test]
fn bad_parent() {
    let file = super::test_file!("bad_parents.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    for err_msg in schema_err.chain() {
        if err_msg.to_string()
            == "Parents field value must be a list of requirement references and end with ']'"
        {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_deprecated() {
    let file = super::test_file!("duplicate_deprecated.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    let mut dup_key_msg = false;
    let mut depr_key = false;

    for err in schema_err.chain() {
        let err_msg = err.to_string();

        if err_msg == "Duplicate entry! Key has been set more than once" {
            dup_key_msg = true;
        } else if err_msg == "Key: Deprecated" {
            depr_key = true;
        }

        if dup_key_msg && depr_key {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_exclude() {
    let file = super::test_file!("duplicate_exclude.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    let mut dup_key_msg = false;
    let mut exclude_key = false;

    for err in schema_err.chain() {
        let err_msg = err.to_string();

        if err_msg == "Duplicate entry! Key has been set more than once" {
            dup_key_msg = true;
        } else if err_msg == "Key: Exclude" {
            exclude_key = true;
        }

        if dup_key_msg && exclude_key {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_manual_verification() {
    let file = super::test_file!("duplicate_manual_verification.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    let mut dup_key_msg = false;
    let mut manual_key = false;

    for err in schema_err.chain() {
        let err_msg = err.to_string();

        if err_msg == "Duplicate entry! Key has been set more than once" {
            dup_key_msg = true;
        } else if err_msg == "Key: Manual Verification (or alias 'Manual')" {
            manual_key = true;
        }

        if dup_key_msg && manual_key {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_optional() {
    let file = super::test_file!("duplicate_optional.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    let mut dup_key_msg = false;
    let mut opt_key = false;

    for err in schema_err.chain() {
        let err_msg = err.to_string();

        if err_msg == "Duplicate entry! Key has been set more than once" {
            dup_key_msg = true;
        } else if err_msg == "Key: Optional" {
            opt_key = true;
        }

        if dup_key_msg && opt_key {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_parents() {
    let file = super::test_file!("duplicate_parents.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    for err_msg in schema_err.chain() {
        if err_msg.to_string() == "Duplicate entry! Key 'Parents' has been set more than once" {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}

#[test]
fn duplicate_replaces() {
    let file = super::test_file!("duplicate_replaces.md");
    let schema_err =
        markdown::collect_requirements(&ProductId::new(String::from("product-id")).unwrap(), &file)
            .expect_err("Parsing errorneous doc must result in error");

    for err_msg in schema_err.chain() {
        if err_msg.to_string() == "Duplicate entry! Key 'Replaces' has been set more than once" {
            return;
        }
    }

    panic!("Expected error message not found in '{schema_err:#?}'");
}
