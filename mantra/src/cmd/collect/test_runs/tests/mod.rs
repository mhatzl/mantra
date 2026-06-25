use crate::{
    cmd::collect::test_setup::{self, testdata_dir},
    db::{MantraDb, MantraPool},
};

#[sqlx::test]
async fn detect_hierarchy_cycles(pool: MantraPool) {
    let db = MantraDb::new_with_pool(pool);
    let testdata_dir = testdata_dir!("base_test_data");
    let cfg_path = testdata_dir.join("mantra.json5");
    let collect_cfgs = test_setup::test_collect_cfgs(&cfg_path)
        .await
        .expect("Failed to read mantra cfg");

    assert_eq!(
        collect_cfgs.len(),
        1,
        "Only one product must be defined to attach db during collection"
    );

    let cfg = collect_cfgs
        .into_iter()
        .next()
        .expect("Checked that one cfg exists");

    let mut collection = crate::cmd::collect::collect_data(&db, cfg)
        .await
        .expect("Failed to collect data");
    let last_collect_nr = collection.collect_nr();
    let product_id = collection.product_id();
    let test_run_date = "2026-03-28T13:00:00Z";

    // now we need to manually insert a test run, because the schema does not allow cyclical test runs
    // by construction
    sqlx::query!(
        "
        insert into TestRunHierarchies (last_collect_nr, product_id, parent_name, parent_date, child_name, child_date)
        values ($1, $2, $3, $4, $5, $6)
        ",
        last_collect_nr,
        product_id,
        "tr-1.sub",
        test_run_date,
        "tr-1",
        test_run_date
    ).execute(collection.connection_mut()).await.expect("Failed to insert intentionally bad hierarchy cycle");

    collection
        .aggregate_requirements_data()
        .await
        .expect("Requirement data must be valid");
    collection
        .aggregate_annotations_data()
        .await
        .expect("Annotation data must be valid");

    let tr_res = collection.aggregate_test_run_data().await;

    let Err(db_err) = tr_res else {
        panic!("Failed to detect test run cycle")
    };

    for err in db_err.chain() {
        if err.to_string() == "Test run cycle detected!" {
            return;
        }
    }

    panic!("Failed to detect the test run cycle!");
}
