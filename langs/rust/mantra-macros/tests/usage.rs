use mantra_macros::{
    assert_eq_req, assert_ne_req, assert_req, clarify_req, debug_assert_eq_req,
    debug_assert_ne_req, debug_assert_req, impl_req, link_req, satisfy_req, verify_req,
};

#[test]
fn single_expr_macros() {
    let res = satisfy_req!("ID-1" => true);
    assert!(res);
    let res = satisfy_req!("ID-1", "ID-2" => 1 + 1 == 2);
    assert!(res);

    let res = impl_req!("ID-1" => true);
    assert!(res);
    let res = impl_req!("ID-1", "ID-2" => 1 + 1 == 2);
    assert!(res);

    let res = verify_req!("ID-1" => true);
    assert!(res);
    let res = verify_req!("ID-1", "ID-2" => 1 + 1 == 2);
    assert!(res);

    let res = clarify_req!("ID-1" => true);
    assert!(res);
    let res = clarify_req!("ID-1", "ID-2" => 1 + 1 == 2);
    assert!(res);

    let res = link_req!("ID-1" => true);
    assert!(res);
    let res = link_req!("ID-1", "ID-2" => 1 + 1 == 2);
    assert!(res);
}

#[test]
#[should_panic]
fn assert_macro() {
    assert_req!("ID-1" => true);
    assert_req!("ID-1" => false);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn assert_macro_with_static_msg() {
    assert_req!("ID-1" => true, "Passing");
    assert_req!("ID-1" => false, "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn assert_macro_with_param_msg() {
    assert_req!("ID-1" => true, "Passing {}: {}", "value", 1);
    assert_req!("ID-1" => false, "Failing assert {}: {}", "value", 0);
}

#[test]
#[should_panic]
fn debug_assert_macro() {
    debug_assert_req!("ID-1" => true);
    debug_assert_req!("ID-1" => false);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn debug_assert_macro_with_static_msg() {
    debug_assert_req!("ID-1" => true, "Passing");
    debug_assert_req!("ID-1" => false, "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn debug_assert_macro_with_param_msg() {
    debug_assert_req!("ID-1" => true, "Passing {}: {}", "value", 1);
    debug_assert_req!("ID-1" => false, "Failing assert {}: {}", "value", 0);
}

#[test]
#[should_panic]
fn assert_eq_macro() {
    assert_eq_req!("ID-1" => 1, 1);
    assert_eq_req!("ID-1" => 0, 1);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn assert_eq_macro_with_static_msg() {
    assert_eq_req!("ID-1" => 'a', 'a', "Passing");
    assert_eq_req!("ID-1" => 'a', 'b', "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn assert_eq_macro_with_param_msg() {
    assert_eq_req!("ID-1" => "hello", "hello", "Passing {}: {}", "value", 1);
    assert_eq_req!("ID-1" => "hello", "world", "Failing assert {}: {}", "value", 0);
}

#[test]
#[should_panic]
fn debug_assert_eq_macro() {
    debug_assert_eq_req!("ID-1" => 1, 1);
    debug_assert_eq_req!("ID-1" => 0, 1);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn debug_assert_eq_macro_with_static_msg() {
    debug_assert_eq_req!("ID-1" => 'a', 'a', "Passing");
    debug_assert_eq_req!("ID-1" => 'a', 'b', "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn debug_assert_eq_macro_with_param_msg() {
    debug_assert_eq_req!("ID-1" => "hello", "hello", "Passing {}: {}", "value", 1);
    debug_assert_eq_req!("ID-1" => "hello", "world", "Failing assert {}: {}", "value", 0);
}

#[test]
#[should_panic]
fn assert_ne_macro() {
    assert_ne_req!("ID-1" => 1, 0);
    assert_ne_req!("ID-1" => 1, 1);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn assert_ne_macro_with_static_msg() {
    assert_ne_req!("ID-1" => 'a', 'b', "Passing");
    assert_ne_req!("ID-1" => 'a', 'a', "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn assert_ne_macro_with_param_msg() {
    assert_ne_req!("ID-1" => "hello", "world", "Passing {}: {}", "value", 1);
    assert_ne_req!("ID-1" => "hello", "hello", "Failing assert {}: {}", "value", 0);
}

#[test]
#[should_panic]
fn debug_assert_ne_macro() {
    debug_assert_ne_req!("ID-1" => 1, 0);
    debug_assert_ne_req!("ID-1" => 1, 1);
}

#[test]
#[should_panic(expected = "Failing assert")]
fn debug_assert_ne_macro_with_static_msg() {
    debug_assert_ne_req!("ID-1" => 'a', 'b', "Passing");
    debug_assert_ne_req!("ID-1" => 'a', 'a', "Failing assert");
}

#[test]
#[should_panic(expected = "Failing assert value: 0")]
fn debug_assert_ne_macro_with_param_msg() {
    debug_assert_ne_req!("ID-1" => "hello", "world", "Passing {}: {}", "value", 1);
    debug_assert_ne_req!("ID-1" => "hello", "hello", "Failing assert {}: {}", "value", 0);
}
