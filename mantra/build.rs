fn main() {
    let crate_path = env!("CARGO_MANIFEST_DIR");
    std::env::set_var(
        "SQLX_OFFLINE_DIR",
        std::path::PathBuf::from(crate_path)
            .join(".sqlx")
            .to_str()
            .expect("Local path to this repository must be valid UTF8"),
    );
}
