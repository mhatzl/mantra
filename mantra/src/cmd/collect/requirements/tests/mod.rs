use crate::cmd::collect::test_setup::db_from_dir;

#[test_case::test_case("hierarchy_cycle")]
#[test_case::test_case("indirect_hierarchy_cycle")]
#[tokio::test]
async fn detect_hierarchy_cycles(dir: &str) {
    let db_res = db_from_dir!(dir);

    let Err(db_err) = db_res else {
        panic!("Failed to detect requirement cycle")
    };
    assert_eq!(db_err.to_string(), "Requirement cycle detected!");
}
