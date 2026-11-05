use mantra_schema::report::requirement::RequirementState;

use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

#[sqlx::test]
async fn deprecated_indirect_req(pool: MantraPool) {
    let depr_state = RequirementState::Deprecated.as_number();

    let db = db_from_dir!(pool, "deprecated_propagation").unwrap();

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
        !depr_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to **not** have deprecated state due to explicit 'false'."
    );
    assert!(
        depr_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to have deprecated state indirectly due to req-1."
    );
    assert!(
        !depr_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to **not** have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have deprecated state."
    );
    assert!(
        depr_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to have deprecated state indirectly due to req-1.sub-1."
    );
    assert!(
        !depr_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to **not** have deprecated state."
    );
    assert!(
        !depr_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to **not** have deprecated state due to only one of two parents being deprecated."
    );
}

#[sqlx::test]
async fn excluded_indirect_req(pool: MantraPool) {
    let excluded_state = RequirementState::Excluded.as_number();

    let db = db_from_dir!(pool, "excluded_propagation").unwrap();

    let excluded_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        excluded_state
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
        !excluded_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to **not** have excluded state due to explicit 'false'."
    );
    assert!(
        !excluded_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to **not** have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to have excluded state indirectly due to req-1."
    );
    assert!(
        !excluded_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to **not** have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have excluded state."
    );
    assert!(
        excluded_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to have excluded state indirectly due to req-1.sub-1."
    );
    assert!(
        !excluded_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to **not** have excluded state."
    );
    assert!(
        !excluded_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to **not** have excluded state."
    );
}

#[sqlx::test]
async fn draft_indirect_req(pool: MantraPool) {
    let draft_state = RequirementState::Draft.as_number();

    let db = db_from_dir!(pool, "draft_propagation").unwrap();

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
        !draft_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to **not** have draft state due to explicit 'false'."
    );
    assert!(
        draft_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to have draft state indirectly due to req-1."
    );
    assert!(
        !draft_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to **not** have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to have draft state."
    );
    assert!(
        draft_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to have draft state indirectly due to req-1.sub-1."
    );
    assert!(
        !draft_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to **not** have draft state."
    );
    assert!(
        !draft_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to **not** have draft state due to only one of two parents being draft."
    );
}

#[sqlx::test]
async fn optional_indirect_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let verified_state = RequirementState::Verified.as_number();
    let failed_state = RequirementState::Failed.as_number();

    let db = db_from_dir!(pool, "optional_propagation").unwrap();

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

    let optional_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from OptionalRequirements
        "
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
        "Expected req-1 to be indirectly verified."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to be indirectly verified."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to be directly verified."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be optional."
    );
    assert!(
        failed_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be in failed state due to indirect fail propagation."
    );
    assert!(
        failed_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be in failed state due to indirect fail propagation."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be optional."
    );
    assert!(
        failed_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be failed."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be indirectly optional."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to be verified."
    );
    assert!(
        verified_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to be indirectly verified."
    );
    assert!(
        verified_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be directly verified."
    );
    assert!(
        unknown_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to be indirectly verified."
    );
    assert!(
        optional_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-4.sub-1".to_string()),
        "Expected req-4.sub-1 to be directly verified."
    );
    assert!(
        optional_reqs.contains(&"req-4.sub-1".to_string()),
        "Expected req-4.sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to be indirectly verified."
    );
    assert!(
        optional_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to be indirectly verified."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be directly verified."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to be verified."
    );
    assert!(
        !optional_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to **not** be optional."
    );
    assert!(
        verified_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7 to be verified."
    );
    assert!(
        !optional_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7.sub-1 to **not** be optional."
    );
}

#[sqlx::test]
async fn verified_indirect_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let verified_state = RequirementState::Verified.as_number();
    let partially_verified_state = RequirementState::PartiallyVerified.as_number();

    let db = db_from_dir!(pool, "verified_propagation").unwrap();

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

    let direct_verified_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from DirectRequirementVerificationStates
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

    let optional_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from OptionalRequirements
        "
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

    let manual_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from ManualRequirements
        "
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
        "Expected req-1 to be indirectly verified."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to be indirectly verified."
    );
    assert!(
        direct_verified_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be directly verified."
    );
    assert!(
        partially_verified_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be partially verified."
    );
    assert!(
        manual_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be verified."
    );
    assert!(
        !manual_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to **not** require manual verification."
    );
    assert!(
        unknown_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to be unknown."
    );
    assert!(
        manual_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to be verified."
    );
    assert!(
        partially_verified_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be partially verified."
    );
    assert!(
        direct_verified_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be directly verified."
    );
    assert!(
        optional_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-3.sub-1.sub-sub-1".to_string()),
        "Expected req-3.sub-1.sub-sub-1 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-3.sub-1.sub-sub-1".to_string()),
        "Expected req-3.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-3.sub-1.sub-sub-2".to_string()),
        "Expected req-3.sub-1.sub-sub-2 to be unknown."
    );
    assert!(
        !optional_reqs.contains(&"req-3.sub-1.sub-sub-2".to_string()),
        "Expected req-3.sub-1.sub-sub-2 to **not** be optional."
    );
    assert!(
        verified_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be verified."
    );
}

#[sqlx::test]
async fn indirec_direct_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let verified_state = RequirementState::Verified.as_number();
    let skipped_state = RequirementState::Skipped.as_number();
    let failed_state = RequirementState::Failed.as_number();
    let partially_verified_state = RequirementState::PartiallyVerified.as_number();

    let db = db_from_dir!(pool, "indirect_direct_mix").unwrap();

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

    let optional_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from OptionalRequirements
        "
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
        "Expected req-1 to be failed."
    );
    assert!(
        failed_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to be failed."
    );
    assert!(
        failed_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to be failed."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be optional."
    );
    assert!(
        failed_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be failed."
    );
    assert!(
        failed_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be failed."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be optional."
    );
    assert!(
        failed_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be failed."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be indirectly optional."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to be verified."
    );
    assert!(
        partially_verified_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to be partially verified."
    );
    assert!(
        unknown_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be unknown."
    );
    assert!(
        verified_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be optional."
    );
    assert!(
        failed_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to be failed."
    );
    assert!(
        verified_reqs.contains(&"req-4.sub-1".to_string()),
        "Expected req-4.sub-1 to be directly verified."
    );
    assert!(
        unknown_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to be indirectly verified."
    );
    assert!(
        optional_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to be directly verified."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to be directly verified."
    );
    assert!(
        optional_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to be optional."
    );
    assert!(
        skipped_reqs.contains(&"req-6.sub-1".to_string()),
        "Expected req-6.sub-1 to be directly skipped."
    );
    assert!(
        optional_reqs.contains(&"req-6.sub-1".to_string()),
        "Expected req-6.sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-6.sub-1.sub-sub-1".to_string()),
        "Expected req-6.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-6.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-6.sub-2".to_string()),
        "Expected req-5.sub-2 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-6.sub-2".to_string()),
        "Expected req-6.sub-2 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be optional."
    );
    assert!(
        skipped_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7.sub-1 to be directly skipped."
    );
    assert!(
        optional_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7.sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-7.sub-1.sub-sub-1".to_string()),
        "Expected req-7.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-7.sub-1.sub-sub-1".to_string()),
        "Expected req-7.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        verified_reqs.contains(&"req-7.sub-2".to_string()),
        "Expected req-7.sub-2 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-7.sub-2".to_string()),
        "Expected req-7.sub-2 to be optional."
    );
}

#[sqlx::test]
async fn manual_indirect_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let verified_state = RequirementState::Verified.as_number();
    let partially_verified_state = RequirementState::PartiallyVerified.as_number();

    let db = db_from_dir!(pool, "manual_propagation").unwrap();

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

    let optional_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from OptionalRequirements
        "
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

    let manual_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from ManualRequirements
        "
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
        "Expected req-1 to be verified."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to be verified."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to be verified."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be optional."
    );
    assert!(
        manual_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be verified."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be verified."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be optional."
    );
    assert!(
        manual_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to require manual verification."
    );
    assert!(
        unknown_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be indirectly optional."
    );
    assert!(
        manual_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to be verified."
    );
    assert!(
        verified_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to be verified."
    );
    assert!(
        manual_reqs.contains(&"req-3".to_string()),
        "Expected req-3 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-3.sub-1".to_string()),
        "Expected req-3.sub-1 to be verified."
    );
    assert!(
        unknown_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-3.sub-2".to_string()),
        "Expected req-3.sub-2 to be optional."
    );
    assert!(
        partially_verified_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to be partially verified."
    );
    assert!(
        manual_reqs.contains(&"req-4".to_string()),
        "Expected req-4 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-4.sub-1".to_string()),
        "Expected req-4.sub-1 to be directly verified."
    );
    assert!(
        manual_reqs.contains(&"req-4.sub-1".to_string()),
        "Expected req-4.sub-1 to require manual verification."
    );
    assert!(
        unknown_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to be unknown."
    );
    assert!(
        manual_reqs.contains(&"req-4.sub-2".to_string()),
        "Expected req-4.sub-2 to require manual verification."
    );
    assert!(
        partially_verified_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to be indirectly partially verified."
    );
    assert!(
        manual_reqs.contains(&"req-5".to_string()),
        "Expected req-5 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to be directly verified."
    );
    assert!(
        manual_reqs.contains(&"req-5.sub-1".to_string()),
        "Expected req-5.sub-1 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to be directly verified."
    );
    assert!(
        manual_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
        "Expected req-5.sub-1.sub-sub-1 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to be verified."
    );
    assert!(
        manual_reqs.contains(&"req-5.sub-2".to_string()),
        "Expected req-5.sub-2 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to be verified."
    );
    assert!(
        !manual_reqs.contains(&"req-6".to_string()),
        "Expected req-6 to **not** require manual verification."
    );
    assert!(
        partially_verified_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to be partially verified."
    );
    assert!(
        !manual_reqs.contains(&"req-7".to_string()),
        "Expected req-7 to **not** require manual verification."
    );
    assert!(
        partially_verified_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7.sub-1 to be partially verified."
    );
    assert!(
        manual_reqs.contains(&"req-7.sub-1".to_string()),
        "Expected req-7.sub-1 to require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-7.sub-1.sub-sub-1".to_string()),
        "Expected req-7.sub-1.sub-sub-1 to be verified."
    );
    assert!(
        !manual_reqs.contains(&"req-7.sub-1.sub-sub-1".to_string()),
        "Expected req-7.sub-1.sub-sub-1 to **not** require manual verification."
    );
    assert!(
        verified_reqs.contains(&"req-7.sub-2".to_string()),
        "Expected req-7.sub-2 to be verified."
    );
    assert!(
        !manual_reqs.contains(&"req-7.sub-2".to_string()),
        "Expected req-7.sub-2 to **not** require manual verification."
    );
}

#[sqlx::test]
async fn nested_req(pool: MantraPool) {
    let unknown_state = RequirementState::Unknown.as_number();
    let verified_state = RequirementState::Verified.as_number();
    let skipped_state = RequirementState::Skipped.as_number();
    let excluded_state = RequirementState::Excluded.as_number();
    let deprecated_state = RequirementState::Deprecated.as_number();
    let partially_verified_state = RequirementState::PartiallyVerified.as_number();

    let db = db_from_dir!(pool, "nested_propagation").unwrap();

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

    let excluded_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from RequirementVerificationStates
        where state = $1
        ",
        excluded_state
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

    let optional_reqs: Vec<String> = sqlx::query!(
        "
        select req_id from OptionalRequirements
        "
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
        "Expected req-1 to be partially verified."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-1".to_string()),
        "Expected req-1.sub-1 to be unknown."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
        "Expected req-1.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-1.sub-1.sub-sub-2".to_string()),
        "Expected req-1.sub-1.sub-sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-1.sub-1.sub-sub-2".to_string()),
        "Expected req-1.sub-1.sub-sub-2 to be optional."
    );
    assert!(
        excluded_reqs.contains(&"req-1.sub-1.sub-sub-3".to_string()),
        "Expected req-1.sub-1.sub-sub-3 to be excluded."
    );
    assert!(
        verified_reqs.contains(&"req-1.sub-2".to_string()),
        "Expected req-1.sub-2 to be verified."
    );
    assert!(
        excluded_reqs.contains(&"req-1.sub-3".to_string()),
        "Expected req-1.sub-3 to be excluded."
    );
    assert!(
        skipped_reqs.contains(&"req-2".to_string()),
        "Expected req-2 to be skipped."
    );
    assert!(
        skipped_reqs.contains(&"req-2.sub-1".to_string()),
        "Expected req-2.sub-1 to be skipped."
    );
    assert!(
        unknown_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
        "Expected req-2.sub-1.sub-sub-1 to be optional."
    );
    assert!(
        unknown_reqs.contains(&"req-2.sub-1.sub-sub-2".to_string()),
        "Expected req-2.sub-1.sub-sub-2 to be unknown."
    );
    assert!(
        optional_reqs.contains(&"req-2.sub-1.sub-sub-2".to_string()),
        "Expected req-2.sub-1.sub-sub-2 to be optional."
    );
    assert!(
        deprecated_reqs.contains(&"req-2.sub-1.sub-sub-3".to_string()),
        "Expected req-2.sub-1.sub-sub-3 to be deprecated."
    );
    assert!(
        verified_reqs.contains(&"req-2.sub-2".to_string()),
        "Expected req-2.sub-2 to be verified."
    );
    assert!(
        excluded_reqs.contains(&"req-2.sub-3".to_string()),
        "Expected req-2.sub-3 to be excluded."
    );
}
