#[req(main_id)]
fn traced_fn() {}

#[cfg(test)]
mod test {
    #[test]
    #[req(other_id, bad_id)]
    fn test_fn() {
        traced_fn();
    }
}
