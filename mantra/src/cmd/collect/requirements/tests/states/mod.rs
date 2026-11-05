use mantra_schema::report::requirement::RequirementState;

use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

#[sqlx::test]
async fn deprecated_req(pool: MantraPool) {
    let depr_state = RequirementState::Deprecated.as_number();

    let db = db_from_dir!(pool, "deprecated").unwrap();

    let depr_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        depr_state
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
        depr_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to have deprecated state indirectly due to req-1."
    );
    assert!(
        !depr_reqs.contains(&"req-2".to_string()),
        "Req-2 must not be marked as deprecated."
    );
    assert!(
        depr_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to have deprecated state, even if covering test failed."
    );
    assert!(
        depr_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to have deprecated state, even if covered test was skipped."
    );
    assert!(
        depr_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to have deprecated state, even if covered test passed."
    );
    assert!(
        depr_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to have deprecated state, even if req was manually verified."
    );
    assert!(
        depr_reqs.contains(&"req-8".to_string()),
        "Expected req-8 to have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-9".to_string()),
        "Expected req-9 to have deprecated state."
    );
}

#[sqlx::test]
async fn excluded_req(pool: MantraPool) {
    let exclude_state = RequirementState::Excluded.as_number();

    let db = db_from_dir!(pool, "excluded").unwrap();

    let excluded_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        exclude_state
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
        excluded_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to have excluded state indirectly due to req-1."
    );
    assert!(
        !excluded_reqs.contains(&"req-2".to_string()),
        "Req-2 must not be marked as excluded."
    );
    assert!(
        excluded_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to have excluded state, even if covering test failed."
    );
    assert!(
        excluded_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to have excluded state, even if covered test was skipped."
    );
    assert!(
        excluded_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to have excluded state, even if covered test passed."
    );
    assert!(
        excluded_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to have excluded state, even if req was manually verified."
    );
    assert!(
        excluded_reqs.contains(&"req-8".to_string()),
        "Expected req-8 to have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-9".to_string()),
        "Expected req-9 to have excluded state."
    );
}

#[sqlx::test]
async fn draft_req(pool: MantraPool) {
    let draft_state = RequirementState::Draft.as_number();

    let db = db_from_dir!(pool, "draft").unwrap();

    let draft_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        draft_state
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
        draft_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to have draft state indirectly due to req-1."
    );
    assert!(
        !draft_reqs.contains(&"req-2".to_string()),
        "Req-2 must not be marked as draft."
    );
    assert!(
        draft_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to have draft state, even if covering test failed."
    );
    assert!(
        draft_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to have draft state, even if covered test was skipped."
    );
    assert!(
        draft_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to have draft state, even if covered test passed."
    );
    assert!(
        draft_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to have draft state, even if req was manually verified."
    );
    assert!(
        draft_reqs.contains(&"req-8".to_string()),
        "Expected req-8 to have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-9".to_string()),
        "Expected req-9 to have draft state."
    );
}

#[sqlx::test]
async fn failed_req(pool: MantraPool) {
    let failed_state = RequirementState::Failed.as_number();

    let db = db_from_dir!(pool, "failed").unwrap();

    let failed_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        failed_state
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
        failed_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to have failed state."
    );
    assert!(
        !failed_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to not be failed."
    );
    assert!(
        failed_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be failed indirectly due to req-2.sub-1."
    );
    assert!(
        failed_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have failed state."
    );
    assert!(
        failed_reqs.contains(&"req-skipped".to_string()),
        "Expected req-skipped to have failed state, even if other covered test was skipped."
    );
    assert!(
        failed_reqs.contains(&"req-verified".to_string()),
        "Expected req-verified to have failed state, even if other covered test passed."
    );
    assert!(
        failed_reqs.contains(&"req-manual".to_string()),
        "Expected req-manual to have failed state, even if req was manually verified."
    );
}

#[sqlx::test]
async fn skipped_req(pool: MantraPool) {
    let skipped_state = RequirementState::Skipped.as_number();

    let db = db_from_dir!(pool, "skipped").unwrap();

    let skipped_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        skipped_state
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
        skipped_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to have skipped state."
    );
    assert!(
        !skipped_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to not be skipped."
    );
    assert!(
        skipped_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be skipped indirectly due to req-2.sub-1."
    );
    assert!(
        skipped_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have skipped state."
    );
    assert!(
        skipped_reqs.contains(&"req-verified".to_string()),
        "Expected req-verified to have skipped state, even if other covered test passed."
    );
    assert!(
        skipped_reqs.contains(&"req-manual".to_string()),
        "Expected req-manual to have skipped state, even if req was manually verified."
    );
}

#[sqlx::test]
async fn unknown_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let satisfied_state = RequirementState::Satisfied.as_number();
    let partially_verified_state = RequirementState::PartiallyVerified.as_number();

    let db = db_from_dir!(pool, "unknown").unwrap();

    let partially_verified_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        partially_verified_state
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

    let unknown_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        unknown_state
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

    let satisfied_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        satisfied_state
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
        partially_verified_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to be partially verified, due to unknown child and direct verified."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to have unknown state."
    );
    assert!(
        unknown_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to have unknown state."
    );
    assert!(
        unknown_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have unknown state."
    );
    assert!(
        partially_verified_reqs.contains(&"req-manual".to_string()),
        "Expected req-manual to be partially verified due to verified by test, but only partial due to missing review."
    );
    assert!(
        satisfied_reqs.contains(&"req-satisfy".to_string()),
        "Expected req-satisfy to have satisfied state, because no test verifies the req, but satisfy trace exists."
    );
    assert!(
        unknown_reqs.contains(&"req-verify".to_string()),
        "Expected req-verify to have unknown state, because no test covers the verify trace."
    );
    assert!(
        satisfied_reqs.contains(&"req-satisfy-verify".to_string()),
        "Expected req-satisfy-verify to have satisfied state,
        because the test covering the verify trace does not cover the satisfy trace and vice versa."
    );
    assert!(
        satisfied_reqs.contains(&"req-mult-satisfy".to_string()),
        "Expected req-mult-satisfy to have satisfied state, because it has no (covered) verify traces or direct test."
    );
}

#[sqlx::test]
async fn verified_req(pool: MantraPool) {
    let verified_state = RequirementState::Verified.as_number();

    let db = db_from_dir!(pool, "verified").unwrap();

    let verified_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        verified_state
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
        verified_reqs.contains(&"req-1".to_string()),
        "Expected req-1 to be indirectly verified through req-1.sub-1."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to have verified state."
    );
    assert!(
        verified_reqs.contains(&"req-manual".to_string()),
        "Expected req-manual to have verified state."
    );
    assert!(
        verified_reqs.contains(&"req-verify".to_string()),
        "Expected req-verify to have verified state."
    );
    assert!(
        verified_reqs.contains(&"req-satisfy-verify".to_string()),
        "Expected req-satisfy-verify to have verified state"
    );
}
