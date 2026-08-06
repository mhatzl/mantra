use crate::collect::{collector::AnnotationCollector, rust::RustCodeCollector};

#[test]
fn simple_attrb() {
    let content = r#"
        #[req("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn mult_ids() {
    let content = r#"
        #[req("ID1", "ID2")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn satisfying_attrb() {
    let content = r#"
        #[req_satisfied("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn clarifying_attrb() {
    let content = r#"
        #[req_note("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn verifying_attrb() {
    let content = r#"
        #[req_verified("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn test_attrb() {
    let content = r#"
        #[req_test("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
    {
        info => &content,
        omit_expression => true
    }, {
        insta::assert_ron_snapshot!(annotations);
    }
    );
}

#[test]
fn linking_attrb() {
    let content = r#"
        #[req_link("ID1")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn two_attrbs() {
    let content = r#"
        #[req_verified("ID1")]
        #[req_verified("ID2")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn simple_cfg_attrb() {
    let content = r#"
        #[cfg_attr(feature = "some-feature", req_link("ID1"))]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn cfg_mult_attrbs() {
    let content = r#"
        #[cfg_attr(feature = "some-feature", derive(Copy), req("ID2"), req_link("ID1"))]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn nested_cfg_attrb() {
    let content = r#"
        #[cfg_attr(feature = "some-feature", cfg_attr(target_os = "some-os", req_link("ID1")))]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn simple_fn_like() {
    let content = r#"
        fn foo() {
            satisfy_req!("ID1" => {
                // some code...
            })
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn clarifying_fn_like_mult_ids() {
    let content = r#"
        fn foo() {
            clarify_req!("ID1", "ID2" => {
                // some code...
            })
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn verifying_fn_like() {
    let content = r#"
        fn foo() {
            verify_req!("ID1" => {
                // some code...
            })
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn link_fn_like() {
    let content = r#"
        fn foo() {
            link_req!("ID1.link" => {
                // some code...
            })
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn impl_fn_like() {
    let content = r#"
        fn foo() {
            impl_req!("ID1.impl" => {
                // some code...
            })
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn assert_fn_like() {
    let content = r#"
        fn foo() {
            assert_req!("ID1.assert" => true || false, "Optional comment")
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn debug_assert_fn_like() {
    let content = r#"
        fn foo() {
            debug_assert_req!("ID1.debug_assert" => 1 + 1 == 2)
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn assert_eq_fn_like() {
    let content = r#"
        fn foo() {
            assert_eq_req!("ID1.assert" => true, false, "Optional comment")
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn debug_assert_eq_fn_like() {
    let content = r#"
        fn foo() {
            debug_assert_eq_req!("ID1.debug_assert" => 1, 1)
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn assert_ne_fn_like() {
    let content = r#"
        fn foo() {
            assert_ne_req!("ID1.assert" => true, false, "Optional comment")
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn debug_assert_ne_fn_like() {
    let content = r#"
        fn foo() {
            debug_assert_ne_req!("ID1.debug_assert" => 1, 2)
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_product_id() {
    let content = r#"
        #[req({ id: "ID1", product_id: "product-id"})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_product_id_quoted_idents() {
    let content = r#"
        #[req({ "id": "ID1", "product_id": "product-id"})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_product_id_trailing_comma() {
    let content = r#"
        #[req({ id: "ID1", product_id: "product-id",})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_object_no_product_id() {
    let content = r#"
        #[req({ id: "ID1"})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_object_no_product_id_trailing_comma() {
    let content = r#"
        #[req({ id: "ID1",})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_explicit_product_id_followed_by_literal_id() {
    let content = r#"
        #[req({ id: "ID1", product_id: "product-id"}, "ID2")]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn attrb_with_literal_id_followed_by_explicit_product_id() {
    let content = r#"
        #[req("ID1", { id: "ID2", product_id: "product-b"})]
        fn foo() {}
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}

#[test]
fn nested_fn_like() {
    let content = r#"
        fn foo() {
            satisfy_req!("ID1" => {
                // some code...
                clarify_req!("ID2" => {
                    // nested code ...
                });
            });
        }
        "#;
    let annotations = RustCodeCollector::collect(content).unwrap();

    insta::with_settings!(
        {
            info => &content,
            omit_expression => true
        }, {
            insta::assert_ron_snapshot!(annotations);
        }
    );
}
