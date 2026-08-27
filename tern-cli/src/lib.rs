//! # tern-cli
//!
//! A helper CLI for `tern` applications.
use clap::Args;
use std::fmt::{self, Debug, Formatter};
use tern_core::context::ConnStr;
use tern_core::error::{TernError, TernResult};

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

impl Debug for ConnOpt {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnOpt")
            .field(
                "database_url",
                &self
                    .database_url
                    .as_ref()
                    .map(|_| "REDACTED")
                    .unwrap_or("None"),
            )
            .finish()
    }
}
