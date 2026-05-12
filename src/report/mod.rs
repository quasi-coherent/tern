//! Displaying results of an operation.
//!
use chrono::{DateTime, Utc};
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{Applied, Migration, MigrationId};
use tern_core::query::Query;

/// A `Report` is the result of a failed operation whose error contains the
/// successful outcomes up to the failure.
pub type Report<T> = Result<T, Incomplete>;

/// The incomplete results of a migrate operation that exited due to error.
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

pub(crate) trait TryReport<T> {
    fn report(self, partial: &[MigrateOk]) -> Report<T>;
}

impl<T> TryReport<T> for TernResult<T> {
    fn report(self, partial: &[MigrateOk]) -> Report<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                let incomplete = Incomplete::new(partial.to_vec(), e);
                Err(incomplete)
            }
        }
    }
}

/// An operation that succeeded.
#[derive(Clone, Debug)]
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
        id: MigrationId,
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

    pub(crate) fn version(&self) -> i64 {
        self.version
    }

    pub(crate) fn from_migration<M>(migration: M) -> Self
    where
        M: Migration,
    {
        let id = migration.migration_id();
        Self::new(id, None, Outcome::Unapplied)
    }

    pub(crate) fn unapplied(id: MigrationId, query: Option<Query>) -> Self {
        Self::new(id, query, Outcome::Unapplied)
    }

    pub(crate) fn dryrun(id: MigrationId, query: Option<Query>) -> Self {
        Self::new(id, query, Outcome::Skipped)
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

#[derive(Debug, Clone, Copy)]
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
