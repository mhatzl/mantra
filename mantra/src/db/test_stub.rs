use crate::db::{MantraConnection, MantraDb};

pub async fn test_db() -> MantraDb {
    MantraDb::new(Some("sqlite::memory:"))
        .await
        .expect("Failed to create in-memory SQLite db")
}

impl MantraDb {
    pub async fn test_connection(&self) -> Result<TestConnection, anyhow::Error> {
        Ok(TestConnection(self.pool.acquire().await?))
    }
}

pub struct TestConnection(sqlx::pool::PoolConnection<sqlx::Sqlite>);

impl TestConnection {
    pub(crate) fn connection_mut(&mut self) -> &mut MantraConnection {
        self.0.as_mut()
    }
}
