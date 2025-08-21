use tern_core::error::{Error, TernResult};
use tern_core::source::{AppliedMigration, Migration};

use chrono::{DateTime, Utc};
use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use std::fmt::{self, Display, Formatter};

/// Attach a partial migration report to an error in case some operation failed
/// while in the middle of the operation.
pub trait AttachReport<T, E> {
    /// Return the partial report as context for the error.
    fn with_report(self, migrations: &[MigrationResult]) -> TernResult<T>;
}

impl<T, E> AttachReport<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_report(self, migrations: &[MigrationResult]) -> TernResult<T> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Partial {
                source: Box::new(e),
                report: Box::new(Report::new(migrations.to_vec())),
            }),
        }
    }
}

/// A collection of formatted migration results.
#[derive(Clone, Serialize, DebugAsJson, DisplayAsJsonPretty, Default)]
pub struct Report {
    migrations: Vec<MigrationResult>,
}

impl Report {
    pub(super) fn new(migrations: Vec<MigrationResult>) -> Self {
        Self { migrations }
    }

    /// Return the vector of results.
    pub fn results(&self) -> Vec<MigrationResult> {
        self.migrations.clone()
    }

    /// Return an iterator of the migration results.
    pub fn iter_results(&self) -> impl Iterator<Item = MigrationResult> {
        self.migrations.clone().into_iter()
    }
}

/// A formatted migration result.
#[derive(Clone, Serialize, DebugAsJson, DisplayAsJsonPretty)]
#[allow(dead_code)]
pub struct MigrationResult {
    dryrun: bool,
    version: i64,
    state: MigrationState,
    applied_at: Option<DateTime<Utc>>,
    description: String,
    content: String,
    transactional: Transactional,
    duration_ms: RunDuration,
}

impl MigrationResult {
    pub(crate) fn from_applied(applied: &AppliedMigration, no_tx: Option<bool>) -> Self {
        Self {
            dryrun: false,
            version: applied.version,
            state: MigrationState::Applied,
            applied_at: Some(applied.applied_at),
            description: applied.description.clone(),
            content: applied.content.clone(),
            transactional: no_tx
                .map(Transactional::from_boolean)
                .unwrap_or(Transactional::Other("Previously applied".to_string())),
            duration_ms: RunDuration::Duration(applied.duration_ms),
        }
    }

    pub(crate) fn from_soft_applied(applied: &AppliedMigration, dryrun: bool) -> Self {
        Self {
            dryrun,
            version: applied.version,
            state: MigrationState::SoftApplied,
            applied_at: Some(applied.applied_at),
            description: applied.description.clone(),
            content: applied.content.clone(),
            transactional: Transactional::Other("Soft applied".to_string()),
            duration_ms: RunDuration::Duration(applied.duration_ms),
        }
    }

    pub(crate) fn from_unapplied<M>(migration: &M, content: &str) -> Self
    where
        M: Migration + ?Sized,
    {
        Self {
            dryrun: true,
            version: migration.version(),
            state: MigrationState::Unapplied,
            applied_at: None,
            description: migration.migration_id().description(),
            content: content.into(),
            transactional: Transactional::from_boolean(migration.no_tx()),
            duration_ms: RunDuration::Unapplied,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize)]
enum MigrationState {
    Applied,
    SoftApplied,
    Unapplied,
}

#[derive(Debug, Clone, Serialize)]
enum Transactional {
    NoTransaction,
    InTransaction,
    Other(String),
}

impl Transactional {
    fn from_boolean(v: bool) -> Self {
        if v {
            return Self::NoTransaction;
        };
        Self::InTransaction
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
enum RunDuration {
    Duration(i64),
    Unapplied,
}

impl Display for Transactional {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTransaction => write!(f, "No Transaction"),
            Self::InTransaction => write!(f, "In Transaction"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl Display for MigrationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Applied => write!(f, "Applied"),
            Self::SoftApplied => write!(f, "Soft Applied"),
            Self::Unapplied => write!(f, "Not Applied"),
        }
    }
}

impl Display for RunDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duration(ms) => write!(f, "{}ms", ms),
            Self::Unapplied => write!(f, "Not Applied"),
        }
    }
}
