//! # tern-cli
//!
//! A helper CLI for `tern` applications.
use clap::Args;
use tern_core::error::{TernError, TernResult};
use tern_executor::{ConnStr, ExecutorOptions};

/// Option to provide a connection string for the target database.
#[derive(Clone, Args)]
pub struct ConnOpt {
    /// The full database URL.
    #[arg(long, short = 'D', env)]
    database_url: Option<ConnStr>,
}

impl ConnOpt {
    /// Get the database url from options.
    pub fn get_db_url(&self) -> TernResult<ConnStr> {
        self.database_url.clone().ok_or_else(|| {
            TernError::Invalid(
                "missing environment variable DATABASE_URL".into(),
            )
        })
    }
}

impl<E> ExecutorOptions<E> for ConnOpt
where
    ConnStr: ExecutorOptions<E>,
{
    async fn connect(self) -> TernResult<E> {
        let conn = self.get_db_url()?;
        conn.connect().await
    }
}
