use crate::db::MantraDb;

pub(crate) async fn get_test_db() -> MantraDb {
    MantraDb::new(Some("sqlite::memory:"))
        .await
        .expect("Failed to create in-memory SQLite db")
}
