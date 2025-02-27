//! A migration runner for a context.
//!
//! The [`Runner`] type accepts any [`MigrationContext`] and exposes the methods
//! needed for tasks related to database migrations.
//!
//! Each method also exists as a (sub)command of the `App`, available with the
//! feature flag "cli" enabled.
use chrono::{DateTime, Utc};

use crate::error::{DatabaseError as _, TernResult};
use crate::migration::{AppliedMigration, Migration, MigrationContext};

/// Run operations on a set of migrations for the chosen context.
pub struct Runner<C: MigrationContext> {
    context: C,
}

impl<C> Runner<C>
where
    C: MigrationContext,
{
    pub fn new(context: C) -> Self {
        Self { context }
    }

    /// Apply all unapplied migrations.
    pub async fn apply_all(&mut self) -> TernResult<Report> {
        self.context.check_history_table().await?;
        let latest = self.context.latest_version().await?;
        let set = self.context.migration_set(latest);

        // Check that the local migration set does not have fewer than what
        // has been applied according to the history table.
        // The derive macro already checked that the local migrations are sound
        // in that there are no duplicates or gaps in version.
        let set_latest = set.max();
        if let (Some(remote_last), Some(local_last)) = (latest, set_latest) {
            if local_last < remote_last {
                return Err(crate::error::Error::MissingSource(local_last, remote_last));
            }
        }

        let mut results = Vec::new();
        for migration in &set.migrations {
            let id = migration.migration_id();
            log::info!("applying migration {id}");

            let result = self
                .context
                .apply(migration.as_ref())
                .await
                .tern_migration_result(migration.as_ref())
                .map(|v| MigrationResult::from_applied(&v, Some(migration.no_tx())))?;

            results.push(result);
        }
        let report = Report::new(results);

        Ok(report)
    }

    /// Return the migration set that would be applied by `apply_all`.
    pub async fn dryrun(&mut self) -> TernResult<Report> {
        self.context.check_history_table().await?;
        let latest = self.context.latest_version().await?;
        let set = self.context.migration_set(latest);

        // Validate source versions more than macros can.
        let set_latest = set.max();
        if let (Some(remote_last), Some(local_last)) = (latest, set_latest) {
            if local_last < remote_last {
                return Err(crate::error::Error::MissingSource(local_last, remote_last));
            }
        }

        let mut unapplied = Vec::new();
        for migration in &set.migrations {
            let query = migration.build(&mut self.context).await?;
            unapplied.push(MigrationResult::from_unapplied(
                migration.as_ref(),
                query.sql(),
            ))
        }
        let report = Report::new(unapplied);

        Ok(report)
    }

    /// List the migrations that have already been applied.
    pub async fn list_applied(&mut self) -> TernResult<Report> {
        self.context.check_history_table().await?;

        let applied = self
            .context
            .previously_applied()
            .await?
            .iter()
            .map(|m| MigrationResult::from_applied(m, None))
            .collect::<Vec<_>>();
        let report = Report::new(applied);

        Ok(report)
    }

    /// `CREATE IF NOT EXISTS` the history table.
    pub async fn init_history(&mut self) -> TernResult<()> {
        self.context.check_history_table().await
    }

    /// `DROP` the history table.
    pub async fn drop_history(&mut self) -> TernResult<()> {
        self.context.drop_history_table().await
    }

    /// Run a "soft apply" for the supplied range of migration versions.
    ///
    /// This means that the migration will be saved in the history table, but
    /// will not have its query applied.  This is useful in the case where you
    /// want to change migration tables, apply a patch to the current one,
    /// migrate from a different migration tool, etc.
    ///
    /// If `from_version` (resp. `to_version`) is `None`, this will soft apply
    /// starting at the first migration (resp. ending with the last migration).
    pub async fn soft_apply(
        &mut self,
        from_version: Option<i64>,
        to_version: Option<i64>,
    ) -> TernResult<Report> {
        self.context.check_history_table().await?;
        let all = self.context.migration_set(None);

        let mut results = Vec::new();
        for migration in &all.migrations {
            let id = migration.migration_id();
            log::info!("soft applying migration {id}");

            let ver = migration.version();

            // Skip if before `from_version`.
            if matches!(from_version, Some(v) if ver < v) {
                continue;
            }
            // Break if after `to_version`.
            if matches!(to_version, Some(v) if ver > v) {
                break;
            }

            let applied = migration.to_applied(0, Utc::now(), "SELECT 1;");
            self.context.upsert_applied(&applied).await?;
            let result = MigrationResult::from_soft_applied(&applied);
            results.push(result);
        }
        let report = Report::new(results);

        Ok(report)
    }
}

/// A formatted version of a collection of migrations.
#[derive(Debug, Clone)]
pub struct Report {
    migrations: Vec<MigrationResult>,
}

impl Report {
    pub fn new(migrations: Vec<MigrationResult>) -> Self {
        Self { migrations }
    }

    pub fn count(&self) -> usize {
        self.migrations.len()
    }
}

/// A formatted version of a migration that is the return type for `Runner`
/// actions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MigrationResult {
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

    pub(crate) fn from_soft_applied(applied: &AppliedMigration) -> Self {
        Self {
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

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
enum MigrationState {
    Applied,
    SoftApplied,
    Unapplied,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy)]
enum RunDuration {
    Duration(i64),
    Unapplied,
}

impl std::fmt::Display for Transactional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTransaction => write!(f, "NO_TRANSACTION"),
            Self::InTransaction => write!(f, "IN_TRANSACTION"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::fmt::Display for MigrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Applied => write!(f, "APPLIED"),
            Self::SoftApplied => write!(f, "SOFT_APPLIED"),
            Self::Unapplied => write!(f, "UNAPPLIED"),
        }
    }
}

impl std::fmt::Display for RunDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duration(ms) => write!(f, "{}ms", ms),
            Self::Unapplied => write!(f, "UNAPPLIED"),
        }
    }
}
