use sqlx::Pool;

pub(crate) mod add;

pub type DB = sqlx::sqlite::Sqlite;

#[derive(Debug)]
pub struct MantraDb {
    pool: Pool<DB>,
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
    pub async fn new(cfg: &Config) -> Result<Self, DbError> {
        let url = cfg
            .url
            .clone()
            .unwrap_or("sqlite://mantra.db?mode=rwc".to_string());
        let pool = Pool::<DB>::connect(&url)
            .await
            .map_err(|err| DbError::Connect(err.to_string()))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|err| DbError::Migrate(err.to_string()))?;

        Ok(Self { pool })
    }
}
