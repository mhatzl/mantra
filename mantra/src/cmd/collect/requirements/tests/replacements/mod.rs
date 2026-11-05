use mantra_schema::report::requirement::RequirementState;

use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

#[sqlx::test]
async fn valid_req_replacements(pool: MantraPool) {
    let deprecated_state = RequirementState::Deprecated.as_number();

    let db = db_from_dir!(pool, "valid_replaces").unwrap();

    let deprecated_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        deprecated_state
    )
    .fetch_all(
        db.connection()
            .await
            .expect("Failed to get a connection")
            .as_mut(),
    )
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.req_id)
    .collect();

    assert!(
        deprecated_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to be indirectly deprecated through req-2."
    );
    assert!(
        deprecated_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to be indirectly deprecated through req-4."
    );
    assert!(
        deprecated_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to be indirectly deprecated through req-5."
    );
    assert!(
        deprecated_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to be indirectly deprecated through req-7."
    );
    assert!(
        deprecated_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be directly deprecated."
    );
}

#[sqlx::test]
async fn bad_req_replacements(pool: MantraPool) {
    let db_res = db_from_dir!(pool, "bad_replaces");

    let Err(db_err) = db_res else {
        panic!("Failed to detect bad replacement relations")
    };

    for err in db_err.chain() {
        if err.to_string() == "Requirement tried to replace its ancestor" {
            return;
        }
    }
    panic!("Failed to detect bad replacement relations in errors: {db_err:#}");
}
