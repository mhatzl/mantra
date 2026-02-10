use mantra_schema::path::RelativePathBuf;
use sqlx::SqlitePool;

// pub(crate) mod add;

#[cfg(test)]
mod test_setup;

pub type MantraConnection = sqlx::sqlite::SqliteConnection;
pub type MantraTransaction<'db> = sqlx::Transaction<'db, sqlx::sqlite::Sqlite>;

#[derive(Debug)]
pub struct MantraDb {
    pool: SqlitePool,
}

#[derive(Debug, Clone, clap::Args)]
#[group(id = "db")]
pub struct Config {
    /// URL to connect to a SQL database.
    /// Default is a SQLite file named `mantra.db` that is located in the current directory.
    #[arg(long, alias = "db-url", env = "MANTRA_DB")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum DbError {
    #[error("Could not get connection to database. Cause: {}", .0)]
    Connect(String),
    #[error("Could not run migration on database. Cause: {}", .0)]
    Migrate(String),
    #[error("Could query database. Cause: {}", .0)]
    Query(String),
    #[error("Could not insert data into database. Cause: {}", .0)]
    Insert(String),
    #[error("Failed to delete table content. Cause: {}", .0)]
    Delete(String),
    #[error("Failed to update table content. Cause: {}", .0)]
    Update(String),
    #[error("The database contains invalid data. Cause: {}", .0)]
    Validate(String),
    // #[error("{}", .0)]
    // ForeignKeyViolation(String),
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

impl MantraDb {
    pub async fn new(url: Option<&str>) -> Result<Self, DbError> {
        let db_url = url.unwrap_or("sqlite://mantra.db?mode=rwc");
        let db = match sqlx::sqlite::SqlitePool::connect(db_url).await {
            Ok(pool) => MantraDb { pool },
            Err(err) => {
                panic!(
                    "Faild to connect to SQLite database. Note: only SQLite is currently supported. Error: {err}"
                );
            }
        };

        MIGRATOR
            .run(&db.pool)
            .await
            .map_err(|err| DbError::Migrate(err.to_string()))?;

        Ok(db)
    }

    pub(crate) async fn start_transaction(&self) -> Result<MantraTransaction<'_>, DbError> {
        match self.pool.try_begin().await {
            Ok(Some(t)) => Ok(t),
            Ok(None) => Err(DbError::Connect(
                "Failed to start a transaction.".to_string(),
            )),
            Err(err) => Err(DbError::Connect(err.to_string())),
        }
    }
}

pub(crate) type Filepath = sqlx::types::Text<RelativePathBuf>;

pub(crate) trait FilepathExt {
    fn to_filepath(self) -> Filepath;
    fn from_filepath(filepath: Filepath) -> Self;
}

impl FilepathExt for RelativePathBuf {
    fn to_filepath(self) -> Filepath {
        sqlx::types::Text(self)
    }

    fn from_filepath(filepath: Filepath) -> Self {
        filepath.0
    }
}
