#[mantra_rust_macros::req_satisfied("gs-req-1")]
fn foo() -> bool {
    // ...
    true
}

#[test]
fn test_foo_1() {
    mantra_rust_macros::assert_req!("gs-req-1" => foo(), "Verification using assert macro");
}

#[mantra_rust_macros::req_test("gs-req-1")]
#[test]
fn test_foo_2() {
    core::assert!(foo(), "Verification using attribute macro");
}
