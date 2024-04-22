#[req(main_id)]
fn traced_fn() {}

#[cfg(test)]
mod test {
    #[test]
    fn test_fn() {
        traced_fn();
    }
}
