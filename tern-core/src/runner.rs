//! A migration runner for a context.
//!
//! The [`Runner`] type accepts any [`MigrationContext`] and exposes the methods
//! needed for tasks related to database migrations.
//!
//! Each method also exists as a (sub)command of the `App`, available with the
//! feature flag "cli" enabled.
use crate::error::{DatabaseError as _, Error, TernResult};
use crate::migration::{AppliedMigration, Migration, MigrationContext, MigrationId};

use chrono::{DateTime, Utc};
use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use std::collections::HashSet;
use std::fmt::Write;

/// Run operations on a set of migrations for the chosen context.
pub struct Runner<C: MigrationContext> {
    context: C,
}

impl<C> Runner<C>
where
    C: MigrationContext,
{
    /// Create a new `Runner` with default arguments from a context.
    pub fn new(context: C) -> Self {
        Self { context }
    }

    /// `CREATE IF NOT EXISTS` the history table.
    pub async fn init_history(&mut self) -> TernResult<()> {
        self.context.check_history_table().await
    }

    /// `DROP` the history table.
    pub async fn drop_history(&mut self) -> TernResult<()> {
        self.context.drop_history_table().await
    }

    // Find applied migrations that are not in the source directory.
    async fn validate_source(&mut self) -> TernResult<()> {
        self.context.check_history_table().await?;
        let applied: HashSet<MigrationId> = self
            .context
            .previously_applied()
            .await?
            .into_iter()
            .map(MigrationId::from)
            .collect();
        let source: HashSet<MigrationId> = self
            .context
            .migration_set(None)
            .migration_ids()
            .into_iter()
            .collect();

        check_migrations_in_sync(applied, source)
    }

    // Check that the target migration version (for some operation) is valid.
    fn validate_target(
        &self,
        last_applied: Option<i64>,
        target_version: Option<i64>,
    ) -> TernResult<()> {
        let Some(source) = self.context.migration_set(None).max() else {
            return Ok(());
        };
        if let Some(target) = target_version {
            match last_applied {
                Some(applied) if target < applied => Err(Error::Invalid(format!(
                    "target version V{target} earlier than latest applied version V{applied}",
                )))?,
                _ if target > source => Err(Error::Invalid(format!(
                    "target version V{target} does not exist, latest version found was V{source}",
                )))?,
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    /// Apply unapplied migrations up to and including the specified version.
    pub async fn run_apply(
        &mut self,
        target_version: Option<i64>,
        dryrun: bool,
    ) -> TernResult<Report> {
        self.validate_source().await?;
        let last_applied = self.context.latest_version().await?;
        self.validate_target(last_applied, target_version)?;

        let unapplied = self.context.migration_set(last_applied);

        let mut results = Vec::new();
        for migration in &unapplied.migrations {
            let id = migration.migration_id();
            let ver = migration.version();

            // Reached the target version, break the loop.
            if matches!(target_version, Some(end) if ver > end) {
                break;
            }

            let result = if dryrun {
                // Build each query, which possibly includes dynamic ones.
                let query = migration
                    .build(&mut self.context)
                    .await
                    .with_report(&results)?;

                MigrationResult::from_unapplied(migration.as_ref(), query.sql())
            } else {
                log::trace!("applying migration {id}");

                self.context
                    .apply(migration.as_ref())
                    .await
                    .tern_migration_result(migration.as_ref())
                    .with_report(&results)
                    .map(|v| MigrationResult::from_applied(&v, Some(migration.no_tx())))?
            };

            results.push(result);
        }

        Ok(Report::new(results))
    }

    /// Apply all unapplied migrations.
    #[deprecated(since = "3.1.0", note = "use `run_apply_all`")]
    pub async fn apply_all(&mut self) -> TernResult<Report> {
        self.run_apply(None, false).await
    }

    /// Apply all unapplied migrations.
    pub async fn run_apply_all(&mut self, dryrun: bool) -> TernResult<Report> {
        self.run_apply(None, dryrun).await
    }

    /// List the migrations that have already been applied.
    pub async fn list_applied(&mut self) -> TernResult<Report> {
        self.validate_source().await?;

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

    #[deprecated(since = "3.1.0", note = "no valid use case for `start_version`")]
    pub async fn soft_apply(
        &mut self,
        start_version: Option<i64>,
        target_version: Option<i64>,
    ) -> TernResult<Report> {
        if start_version.is_some() {
            return Err(Error::Invalid(
                "no valid `start_version` other than the first unapplied, use `run_soft_apply`"
                    .into(),
            ));
        }
        self.run_soft_apply(target_version, false).await
    }

    /// Run a "soft apply" of the migrations up to and including the specified
    /// version.
    ///
    /// This means that the migration will be saved in the history table, but
    /// will not have its query applied.  This is useful in the case where you
    /// want to change migration tables, apply a patch to the current one,
    /// migrate from a different migration tool, etc.
    pub async fn run_soft_apply(
        &mut self,
        target_version: Option<i64>,
        dryrun: bool,
    ) -> TernResult<Report> {
        self.validate_source().await?;
        let last_applied = self.context.latest_version().await?;
        self.validate_target(last_applied, target_version)?;

        let unapplied = self.context.migration_set(last_applied);

        let mut results = Vec::new();
        for migration in &unapplied.migrations {
            let id = migration.migration_id();
            let ver = migration.version();

            // Reached the last version, break the loop.
            if matches!(target_version, Some(end) if ver > end) {
                break;
            }

            // Build each query, which possibly includes dynamic ones.
            let query = migration
                .build(&mut self.context)
                .await
                .with_report(&results)?;
            let mut content = String::from("-- SOFT APPLIED:\n\n");
            writeln!(content, "{query}")?;

            let applied = migration.to_applied(0, Utc::now(), &content);
            let result = MigrationResult::from_soft_applied(&applied, dryrun);

            if !dryrun {
                log::trace!("soft applying migration {id}");
                self.context
                    .insert_applied(&applied)
                    .await
                    .with_report(&results)?;
            }

            results.push(result);
        }
        let report = Report::new(results);

        Ok(report)
    }
}

/// A formatted version of a collection of migrations.
#[derive(Clone, Serialize, DebugAsJson, DisplayAsJsonPretty, Default)]
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

    /// Return the vector of results.
    pub fn results(&self) -> Vec<MigrationResult> {
        self.migrations.clone()
    }

    /// Return an iterator of the migration results.
    pub fn iter_results(&self) -> impl Iterator<Item = MigrationResult> {
        self.migrations.clone().into_iter()
    }
}

/// A formatted version of a migration that is the return type for `Runner`
/// actions.
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

impl std::fmt::Display for Transactional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTransaction => write!(f, "No Transaction"),
            Self::InTransaction => write!(f, "In Transaction"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl std::fmt::Display for MigrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Applied => write!(f, "Applied"),
            Self::SoftApplied => write!(f, "Soft Applied"),
            Self::Unapplied => write!(f, "Not Applied"),
        }
    }
}

impl std::fmt::Display for RunDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Duration(ms) => write!(f, "{}ms", ms),
            Self::Unapplied => write!(f, "Not Applied"),
        }
    }
}

// Migrations that have been applied already but do not exist locally.
fn check_migrations_in_sync(
    applied: HashSet<MigrationId>,
    source: HashSet<MigrationId>,
) -> TernResult<()> {
    let source_not_found: Vec<&MigrationId> = applied.difference(&source).collect();

    if !source_not_found.is_empty() {
        return Err(Error::OutOfSync {
            at_issue: source_not_found.into_iter().cloned().collect(),
            msg: "version/name applied but missing in source".into(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Error;
    use super::MigrationId;

    use std::collections::HashSet;

    #[test]
    fn missing_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "fourth".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
        ]
        .into_iter()
        .collect();
        let missing = vec![MigrationId::new(3, "third".into())];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == missing));
    }

    #[test]
    fn fewer_in_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fourth".into()),
        ]
        .into_iter()
        .collect();
        let missing = vec![MigrationId::new(4, "fourth".into())];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == missing));
    }

    #[test]
    fn mismatched_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fifth".into()),
            MigrationId::new(5, "sixth".into()),
            MigrationId::new(6, "seventh".into()),
            MigrationId::new(7, "eighth".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fourth".into()),
            MigrationId::new(5, "fifth".into()),
        ]
        .into_iter()
        .collect();
        let divergence = vec![
            MigrationId::new(4, "fourth".into()),
            MigrationId::new(5, "fifth".into()),
        ];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == divergence));
    }
}
