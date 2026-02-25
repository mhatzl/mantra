#[test]
fn first_test() {
    assert_eq!(1 + 1, 2, "trivial add");
}

#[test]
#[should_panic(expected = "failing add")]
fn second_test() {
    assert_eq!(1 + 1, 1, "failing add");
}
