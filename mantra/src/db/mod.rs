use std::str::FromStr;

use mantra_schema::path::RelativePathBuf;

#[cfg(test)]
pub mod test_stub;

pub type MantraPool = sqlx::SqlitePool;
pub type MantraConnection = sqlx::sqlite::SqliteConnection;
pub type MantraTransaction<'db> = sqlx::Transaction<'db, sqlx::sqlite::Sqlite>;

#[derive(Debug)]
pub struct MantraDb {
    pool: MantraPool,
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "db")]
pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located in the current directory.
    #[arg(long, alias = "db-url", env = "MANTRA_DB")]
    pub url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Could not get connection to database. Cause: {}", .0)]
    Connect(anyhow::Error),
    #[error("Could not run migration on database. Cause: {}", .0)]
    Migrate(anyhow::Error),
    #[error("Failed to execute a SQL statement against the database. Cause: {}", .0)]
    Execute(anyhow::Error),
    #[error("The database contains invalid data. Cause: {}", .0)]
    Validate(anyhow::Error),
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

impl MantraDb {
    pub async fn new(url: Option<&str>) -> Result<Self, DbError> {
        let db_url = url.unwrap_or("sqlite://mantra.db?mode=rwc");
        let pool = MantraPool::connect(db_url)
            .await
            .map_err(|err| DbError::Connect(err.into()))?;

        let db = MantraDb { pool };

        MIGRATOR
            .run(&db.pool)
            .await
            .map_err(|err| DbError::Migrate(err.into()))?;

        Ok(db)
    }

    pub(crate) async fn start_transaction(&self) -> Result<MantraTransaction<'_>, DbError> {
        match self.pool.try_begin().await {
            Ok(Some(t)) => Ok(t),
            Ok(None) => Err(DbError::Execute(anyhow::anyhow!(
                "Failed to start a transaction."
            ))),
            Err(err) => Err(DbError::Execute(err.into())),
        }
    }

    pub(crate) async fn close(self) {
        self.pool.close().await
    }
}

pub(crate) type Filepath = sqlx::types::Text<SqlFilepath>;

#[derive(Debug, Clone)]
pub(crate) struct SqlFilepath(RelativePathBuf);

pub(crate) trait FilepathExt {
    fn to_filepath(self) -> Filepath;
    fn from_filepath(filepath: Filepath) -> Self;
}

impl FilepathExt for RelativePathBuf {
    fn to_filepath(self) -> Filepath {
        sqlx::types::Text(SqlFilepath(self))
    }

    fn from_filepath(filepath: Filepath) -> Self {
        filepath.0.0
    }
}

impl FromStr for SqlFilepath {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SqlFilepath(RelativePathBuf::from_path(s)?))
    }
}

impl std::fmt::Display for SqlFilepath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
