use crate::db::MantraDb;

pub struct TestDb {
    db: MantraDb,
    tmp_dir: tempfile::TempDir,
}

const TEST_DB: &str = "test.db";

impl TestDb {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let tmp_dir = tempfile::tempdir()?;
        let db_file = tmp_dir.path().join(TEST_DB);
        let db = MantraDb::new(Some(&format!("sqlite://{}?mode=rwc", db_file.display()))).await?;

        Ok(Self { db, tmp_dir })
    }

    pub fn db(&self) -> &MantraDb {
        &self.db
    }

    pub async fn connection(&self) -> Result<TestConnection, anyhow::Error> {
        Ok(TestConnection {
            connection: self.db.pool.acquire().await?,
        })
    }

    pub fn db_file(&self) -> std::path::PathBuf {
        self.tmp_dir.path().join(TEST_DB)
    }
}

pub struct TestConnection {
    connection: sqlx::pool::PoolConnection<sqlx::Sqlite>,
}

impl TestConnection {
    pub fn as_mut(&mut self) -> &mut crate::db::MantraConnection {
        self.connection.as_mut()
    }
}
