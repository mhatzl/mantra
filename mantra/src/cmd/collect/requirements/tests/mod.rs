use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

#[sqlx::test]
async fn detect_hierarchy_cycles(pool: MantraPool) {
    let dir = "hierarchy_cycle";
    let db_res = db_from_dir!(pool, dir);

    let Err(db_err) = db_res else {
        panic!("Failed to detect requirement cycle")
    };

    for err in db_err.chain() {
        if err.to_string() == "Requirement cycle detected!" {
            return;
        }
    }
    panic!("Failed to detect requirement cycle");
}

#[sqlx::test]
async fn detect_indirect_hierarchy_cycles(pool: MantraPool) {
    let dir = "indirect_hierarchy_cycle";
    let db_res = db_from_dir!(pool, dir);

    let Err(db_err) = db_res else {
        panic!("Failed to detect requirement cycle")
    };

    for err in db_err.chain() {
        if err.to_string() == "Requirement cycle detected!" {
            return;
        }
    }
    panic!("Failed to detect requirement cycle");
}

mod indirect_states {
    use mantra_schema::report::requirement::RequirementState;

    use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

    #[sqlx::test]
    async fn deprecated_indirect_req(pool: MantraPool) {
        let depr_state = RequirementState::Deprecated.as_nr();

        let db = db_from_dir!(pool, "indirect_states/deprecated_propagation").unwrap();

        let depr_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
            depr_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
            "Expected req-1.sub-1.sub-sub-1 to have deprecated state indirectly due to req-1."
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
    }

    #[sqlx::test]
    async fn excluded_indirect_req(pool: MantraPool) {
        let excluded_state = RequirementState::Excluded.as_nr();

        let db = db_from_dir!(pool, "indirect_states/excluded_propagation").unwrap();

        let excluded_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
            excluded_reqs.contains(&"req-1.sub-1.sub-sub-1".to_string()),
            "Expected req-1.sub-1.sub-sub-1 to have excluded state indirectly due to req-1."
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
    }

    #[sqlx::test]
    async fn optional_indirect_req(pool: MantraPool) {
        let unverified_state = RequirementState::Unverified.as_nr();
        let verified_state = RequirementState::Verified.as_nr();
        let failed_state = RequirementState::Failed.as_nr();

        let db = db_from_dir!(pool, "indirect_states/optional_propagation").unwrap();

        let unverified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
            where state = $1
            ",
            unverified_state
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
        .map(|r| r.id)
        .collect();

        let verified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let failed_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let optional_reqs: Vec<String> = sqlx::query!(
            "
            select id from OptionalRequirements
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
        .map(|r| r.id)
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
            unverified_reqs.contains(&"req-1.sub-2".to_string()),
            "Expected req-1.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-3.sub-2".to_string()),
            "Expected req-3.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-4.sub-2".to_string()),
            "Expected req-4.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-5.sub-2".to_string()),
            "Expected req-5.sub-2 to be unverified."
        );
        assert!(
            optional_reqs.contains(&"req-5.sub-2".to_string()),
            "Expected req-5.sub-2 to be optional."
        );
    }

    #[sqlx::test]
    async fn indirec_direct_req(pool: MantraPool) {
        let unverified_state = RequirementState::Unverified.as_nr();
        let verified_state = RequirementState::Verified.as_nr();
        let skipped_state = RequirementState::Skipped.as_nr();
        let failed_state = RequirementState::Failed.as_nr();

        let db = db_from_dir!(pool, "indirect_states/indirect_direct_mix").unwrap();

        let unverified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
            where state = $1
            ",
            unverified_state
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
        .map(|r| r.id)
        .collect();

        let verified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let skipped_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let failed_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let optional_reqs: Vec<String> = sqlx::query!(
            "
            select id from OptionalRequirements
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
        .map(|r| r.id)
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
            unverified_reqs.contains(&"req-1.sub-2".to_string()),
            "Expected req-1.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-3".to_string()),
            "Expected req-3 to be unverified."
        );
        assert!(
            unverified_reqs.contains(&"req-3.sub-1".to_string()),
            "Expected req-3.sub-1 to be unverified."
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
            unverified_reqs.contains(&"req-4.sub-2".to_string()),
            "Expected req-4.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
            "Expected req-5.sub-1.sub-sub-1 to be unverified."
        );
        assert!(
            optional_reqs.contains(&"req-5.sub-1.sub-sub-1".to_string()),
            "Expected req-5.sub-1.sub-sub-1 to be optional."
        );
        assert!(
            unverified_reqs.contains(&"req-5.sub-2".to_string()),
            "Expected req-5.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-6.sub-1.sub-sub-1".to_string()),
            "Expected req-6.sub-1.sub-sub-1 to be unverified."
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
            unverified_reqs.contains(&"req-7.sub-1.sub-sub-1".to_string()),
            "Expected req-7.sub-1.sub-sub-1 to be unverified."
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
        let unverified_state = RequirementState::Unverified.as_nr();
        let verified_state = RequirementState::Verified.as_nr();

        let db = db_from_dir!(pool, "indirect_states/manual_propagation").unwrap();

        let unverified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
            where state = $1
            ",
            unverified_state
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
        .map(|r| r.id)
        .collect();

        let verified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
        .collect();

        let optional_reqs: Vec<String> = sqlx::query!(
            "
            select id from OptionalRequirements
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
        .map(|r| r.id)
        .collect();

        let manual_reqs: Vec<String> = sqlx::query!(
            "
            select id from ManualRequirements
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
        .map(|r| r.id)
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
            unverified_reqs.contains(&"req-1.sub-2".to_string()),
            "Expected req-1.sub-2 to be unverified."
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
            unverified_reqs.contains(&"req-2.sub-1.sub-sub-1".to_string()),
            "Expected req-2.sub-1.sub-sub-1 to be unverified."
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
            unverified_reqs.contains(&"req-3.sub-2".to_string()),
            "Expected req-3.sub-2 to be unverified."
        );
        assert!(
            optional_reqs.contains(&"req-3.sub-2".to_string()),
            "Expected req-3.sub-2 to be optional."
        );
        assert!(
            unverified_reqs.contains(&"req-4".to_string()),
            "Expected req-4 to be unverified."
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
            unverified_reqs.contains(&"req-4.sub-2".to_string()),
            "Expected req-4.sub-2 to be unverified."
        );
        assert!(
            manual_reqs.contains(&"req-4.sub-2".to_string()),
            "Expected req-4.sub-2 to require manual verification."
        );
        assert!(
            unverified_reqs.contains(&"req-5".to_string()),
            "Expected req-5 to be indirectly unverified."
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
    }
}

mod states {
    use mantra_schema::report::requirement::RequirementState;

    use crate::{cmd::collect::test_setup::db_from_dir, db::MantraPool};

    #[sqlx::test]
    async fn deprecated_req(pool: MantraPool) {
        let depr_state = RequirementState::Deprecated.as_nr();

        let db = db_from_dir!(pool, "states/deprecated").unwrap();

        let depr_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
        let exclude_state = RequirementState::Excluded.as_nr();

        let db = db_from_dir!(pool, "states/excluded").unwrap();

        let excluded_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
    async fn failed_req(pool: MantraPool) {
        let failed_state = RequirementState::Failed.as_nr();

        let db = db_from_dir!(pool, "states/failed").unwrap();

        let failed_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
        let skipped_state = RequirementState::Skipped.as_nr();

        let db = db_from_dir!(pool, "states/skipped").unwrap();

        let skipped_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
    async fn unverified_req(pool: MantraPool) {
        let unverified_state = RequirementState::Unverified.as_nr();

        let db = db_from_dir!(pool, "states/unverified").unwrap();

        let unverified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
            where state = $1
            ",
            unverified_state
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
        .map(|r| r.id)
        .collect();

        assert!(
            unverified_reqs.contains(&"req-1".to_string()),
            "Expected req-1 to remain in unverified state, due to unverified child, even if directly verified."
        );
        assert!(
            unverified_reqs.contains(&"req-1.sub-1".to_string()),
            "Expected req-1.sub-1 to remain unverified, even if parent is verified."
        );
        assert!(
            unverified_reqs.contains(&"req-2".to_string()),
            "Expected req-2 to have unverified state."
        );
        assert!(
            unverified_reqs.contains(&"req-2.sub-1".to_string()),
            "Expected req-2.sub-1 to have unverified state."
        );
        assert!(
            unverified_reqs.contains(&"req-manual".to_string()),
            "Expected req-manual to have unverified state, even if req was verified by test, because manual review is missing."
        );
        assert!(
            unverified_reqs.contains(&"req-satisfy".to_string()),
            "Expected req-satisfy to have unverified state, because no test covers the trace."
        );
        assert!(
            unverified_reqs.contains(&"req-verify".to_string()),
            "Expected req-verify to have unverified state, because no test covers the trace."
        );
        assert!(
            unverified_reqs.contains(&"req-satisfy-verify".to_string()),
            "Expected req-satisfy-verify to have unverified state,
            because the test covering the verify trace does not cover the satisfy trace and vice versa."
        );
        assert!(
            unverified_reqs.contains(&"req-mult-satisfy".to_string()),
            "Expected req-mult-satisfy to have unverified state,
            because not all satisfy traces are covered by tests."
        );
    }

    #[sqlx::test]
    async fn verified_req(pool: MantraPool) {
        let verified_state = RequirementState::Verified.as_nr();

        let db = db_from_dir!(pool, "states/verified").unwrap();

        let verified_reqs: Vec<String> = sqlx::query!(
            "
            select id from RequirementVerificationStates
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
        .map(|r| r.id)
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
            verified_reqs.contains(&"req-satisfy".to_string()),
            "Expected req-satisfy to have verified state."
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
}
