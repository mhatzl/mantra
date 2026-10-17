use crate::{
    cmd::collect::{
        collection::Collection,
        test_setup::{self, testdata_dir},
    },
    db::{MantraDb, MantraPool},
};

#[sqlx::test]
async fn detect_hierarchy_cycles(pool: MantraPool) {
    let db = MantraDb::new_with_pool(pool);
    let testdata_dir = testdata_dir!("base_test_data");
    let cfg_path = testdata_dir.join("mantra.json5");
    let collect_cfg = test_setup::collect_test_cfg(&cfg_path)
        .await
        .expect("Failed to read mantra cfg");

    assert_eq!(
        collect_cfg.product_cfgs.len(),
        1,
        "Only one product must be defined to attach db during collection"
    );

    let product_id = collect_cfg.product_cfgs[0].product.id.clone();

    let mut collection = Collection::new(&db, &collect_cfg)
        .await
        .expect("Failed to collect data");
    // collect products, but don't aggregate yet
    collection
        .collect_products(&collect_cfg.args, collect_cfg.product_cfgs)
        .await
        .expect("Collecting test data must not fail");

    let collect_nr = collection.collect_nr();
    let test_run_date = "2026-03-28T13:00:00Z";

    // now we need to manually insert a test run, because the schema does not allow cyclical test runs
    // by construction
    sqlx::query!(
        "
        insert into TestRunHierarchies (collect_nr, product_id, parent_name, parent_date, child_name, child_date)
        values ($1, $2, $3, $4, $5, $6)
        ",
        collect_nr,
        product_id,
        "tr-1.sub",
        test_run_date,
        "tr-1",
        test_run_date
    ).execute(collection.connection_mut()).await.expect("Failed to insert intentionally bad hierarchy cycle");

    let tr_res = collection.aggregate().await;

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
