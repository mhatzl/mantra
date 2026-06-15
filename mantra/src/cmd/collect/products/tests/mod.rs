use crate::{cmd::collect::test_setup::db_from_cfg_file, db::MantraPool};

#[sqlx::test]
async fn single_product(pool: MantraPool) {
    let db = db_from_cfg_file!(pool, "single_product.json5").expect("Failed to create mantra db");

    let records = sqlx::query!(
        "
        select id, name from Products
        "
    )
    .fetch_all(
        db.connection()
            .await
            .expect("Failed to get a connection")
            .as_mut(),
    )
    .await
    .unwrap();

    assert_eq!(records.len(), 1, "Expected exactly one product!");

    let record = records.first().unwrap();
    assert_eq!(record.id, "p1", "DB does not contain correct product ID.");
    assert_eq!(
        record.name, "test",
        "DB does not contain correct product name."
    );
}
