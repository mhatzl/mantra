impl super::MantraDb {
    pub(crate) fn new_with_pool(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    pub(crate) async fn connection(&self) -> Result<TestConnection, anyhow::Error> {
        Ok(TestConnection {
            connection: self.pool.acquire().await?,
        })
    }
}

pub(crate) struct TestConnection {
    connection: sqlx::pool::PoolConnection<sqlx::Sqlite>,
}

impl TestConnection {
    pub(crate) fn as_mut(&mut self) -> &mut crate::db::MantraConnection {
        self.connection.as_mut()
    }
}
