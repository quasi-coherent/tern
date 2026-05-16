//! Displaying results of an operation.
//!
use chrono::{DateTime, Utc};
use display_json::DisplayAsJsonPretty;
use serde::Serialize;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{Applied, Migration, MigrationId, Query};

/// A `Report` of the result of an operation.
#[derive(Debug)]
pub enum Report {
    /// The operation was successful and the value contains individual results.
    Success(Completed),
    /// The operation encountered an error and the value contains the error and
    /// the partial successful results.
    Error(Incomplete),
}

/// Alias for a result with error `Incomplete` containing partial operation
/// outcomes.
pub type OpResult<T> = Result<T, Incomplete>;

/// The incomplete results of a migrate operation that exited due to error.
#[derive(Debug, thiserror::Error)]
pub struct Incomplete {
    partial: Vec<MigrateOk>,
    err: TernError,
}

impl Incomplete {
    /// New from components.
    pub(crate) fn new(partial: Vec<MigrateOk>, err: TernError) -> Self {
        Self { partial, err }
    }
}

impl Display for Incomplete {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.partial.iter().try_for_each(|m| {
            writeln!(f, "{m}")?;
            Ok::<_, fmt::Error>(())
        })?;
        writeln!(f, "{}", self.err)
    }
}

impl From<TernError> for Incomplete {
    fn from(err: TernError) -> Self {
        Self { partial: Vec::new(), err }
    }
}

/// The results of a successful migrate operation.
#[derive(Clone, Debug)]
pub struct Completed(Vec<MigrateOk>);

impl FromIterator<MigrateOk> for Completed {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = MigrateOk>,
    {
        let oks = iter.into_iter().collect();
        Self(oks)
    }
}

pub(crate) trait TryOpResult<T> {
    fn incomplete(self, partial: &[MigrateOk]) -> OpResult<T>;
}

impl<T> TryOpResult<T> for TernResult<T> {
    fn incomplete(self, partial: &[MigrateOk]) -> OpResult<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                let incomplete = Incomplete::new(partial.to_vec(), e);
                Err(incomplete)
            },
        }
    }
}

/// An operation that succeeded.
#[derive(Clone, Debug, Serialize, DisplayAsJsonPretty)]
pub(crate) struct MigrateOk {
    version: i64,
    description: String,
    query: String,
    dryrun: bool,
    outcome: Outcome,
    end_time: DateTime<Utc>,
    duration: Duration,
}

impl MigrateOk {
    pub(crate) fn new(
        id: &MigrationId,
        query: Option<Query>,
        outcome: Outcome,
    ) -> Self {
        Self {
            version: id.version(),
            description: id.description().to_string(),
            query: query
                .map(|q| q.to_string())
                .unwrap_or("Future<Output = Query>".to_string()),
            dryrun: true,
            outcome,
            end_time: Utc::now(),
            duration: Duration::default(),
        }
    }

    pub(crate) fn from_migration<M>(migration: M) -> Self
    where
        M: Migration,
    {
        let id = migration.migration_id();
        Self::new(id, None, Outcome::Unapplied)
    }

    pub(crate) fn unapplied(id: &MigrationId, query: Option<Query>) -> Self {
        Self::new(id, query, Outcome::Unapplied)
    }

    pub(crate) fn dryrun(
        id: &MigrationId,
        query: Option<Query>,
        start: DateTime<Utc>,
    ) -> Self {
        let this = Self::new(id, query, Outcome::Skipped);
        let duration: Option<u64> =
            (this.end_time - start).num_milliseconds().try_into().ok();
        Self {
            duration: duration.map(Duration::from_millis).unwrap_or_default(),
            ..this
        }
    }

    pub(crate) fn applied(value: Applied) -> Self {
        let duration_millis: u64 =
            value.duration_millis().try_into().unwrap_or_default();
        Self {
            version: value.version(),
            description: value.description(),
            query: value.content(),
            dryrun: false,
            outcome: Outcome::Applied,
            end_time: value.applied_at(),
            duration: Duration::from_millis(duration_millis),
        }
    }

    pub(crate) fn soft_applied(value: Applied) -> Self {
        let duration_millis: u64 =
            value.duration_millis().try_into().unwrap_or_default();
        Self {
            version: value.version(),
            description: value.description(),
            query: value.content(),
            dryrun: false,
            outcome: Outcome::SoftApplied,
            end_time: value.applied_at(),
            duration: Duration::from_millis(duration_millis),
        }
    }

    pub(crate) fn reverted(value: Applied) -> Self {
        let duration_millis: u64 =
            value.duration_millis().try_into().unwrap_or_default();
        Self {
            version: value.version(),
            description: value.description(),
            query: value.content(),
            dryrun: false,
            outcome: Outcome::Reverted,
            end_time: value.applied_at(),
            duration: Duration::from_millis(duration_millis),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) enum Outcome {
    Applied,
    SoftApplied,
    Skipped,
    Reverted,
    Unapplied,
}

impl Display for Outcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let data = match self {
            Self::Applied => "Applied",
            Self::SoftApplied => "Soft Applied",
            Self::Skipped => "Skipped",
            Self::Reverted => "Reverted",
            Self::Unapplied => "Unapplied",
        };
        f.write_str(data)
    }
}
