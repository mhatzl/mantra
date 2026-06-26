use test_case::test_case;

#[mantra_macros::req_satisfied("calc.add")]
fn add(a: usize, b: usize) -> usize {
    a + b
}

#[mantra_macros::req_satisfied("calc.sub")]
fn sub(a: usize, b: usize) -> usize {
    a - b
}

#[mantra_macros::req_satisfied("calc.mult")]
fn mult(a: usize, b: usize) -> usize {
    a * b
}

fn div(a: usize, b: usize) -> usize {
    a / b
}

#[mantra_macros::req_satisfied("calc.div")]
fn other_div(a: usize, b: usize) -> usize {
    a / b
}

#[test]
fn basic_add() {
    mantra_macros::assert_eq_req!("calc.add" => add(1, 1), 2, "Simple addition failed");
    mantra_macros::assert_eq_req!("calc.add" => add(1, 2), 3, "Simple addition failed");
}

#[mantra_macros::req_verified("calc.sub")]
#[test]
fn basic_sub() {
    core::assert_eq!(sub(1, 1), 0, "Simple subtraction failed");
    core::assert_eq!(sub(2, 1), 1, "Simple subtraction failed");
}

#[test]
fn basic_mult() {
    core::assert_eq!(mult(1, 1), 1, "Simple mult failed");
    core::assert_eq!(mult(1, 2), 2, "Simple mult failed");
}

#[mantra_macros::req_verified("calc.div")]
#[test_case(1, 1, 1; "One is One")]
#[test_case(2, 1, 2; "One has no effect (1)")]
#[test_case(3, 1, 3; "One has no effect (2)")]
fn basic_div(a: usize, b: usize, exp: usize) {
    core::assert_eq!(div(a, b), exp);
}
