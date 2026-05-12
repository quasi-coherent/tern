//! Top-level application types.
//!
//! The module defines the main interface [`TernMigrate`], which combines a
//! custom context with a collection of migrations over that context.  This
//! combination is suitable for any operation performed during a database
//! migration.  These operations are organized and made available by the app
//! type [`Tern`].
use std::collections::HashSet;
use tern_core::context::MigrationContext;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{Migration, MigrationId};

use crate::migration::{DownMigrationSet, UpMigrationSet};
use crate::operation::TernMigrateOp;
use crate::report::Report;

/// `TernMigrate` is a migration context combined with a set of migrations
/// constructing the database.
pub trait TernMigrate: MigrationContext {
    /// Migration set to construct the target database.
    fn up_migrations(&self) -> UpMigrationSet<Self>
    where
        Self: Sized;
}

/// The `TernMigrate` context is `Invertible` if it can produce a set of down
/// migrations for reverting the database to an earlier state.
pub trait Invertible: TernMigrate {
    /// Migration set to revert the version of the target database.
    fn down_migrations(&self) -> DownMigrationSet<Self>
    where
        Self: Sized;
}

/// `Tern` is the main application.
pub struct Tern<T> {
    inner: T,
    skip_validate: bool,
}

impl<T: TernMigrate> Tern<T> {
    /// Create a new `Tern` app with migrations defined by `T`.
    pub fn new(inner: T) -> Self {
        Self { inner, skip_validate: false }
    }

    /// Skip the check that local and remote migrations are in sync.
    pub fn skip_validate(self) -> Self {
        Self { skip_validate: true, ..self }
    }

    /// Run the operation and return the report of the run.
    pub async fn run<Op: TernMigrateOp<T>>(
        &mut self,
        op: &Op,
    ) -> Report<Op::Output> {
        if !self.skip_validate {
            let mut v = Validate::new(&mut self.inner);
            v.validate().await?;
        }
        op.exec(&mut self.inner).await
    }
}

struct Validate<'a, T> {
    inner: &'a mut T,
}

impl<'a, T: TernMigrate> Validate<'a, T> {
    fn new(inner: &'a mut T) -> Validate<'a, T> {
        Validate { inner }
    }

    async fn validate(&mut self) -> TernResult<()> {
        let local = self
            .inner
            .up_migrations()
            .into_iter()
            .map(|m| m.migration_id())
            .collect::<HashSet<_>>();
        let remote = self
            .inner
            .all_applied()
            .await?
            .into_iter()
            .map(|m| m.migration_id())
            .collect::<HashSet<_>>();
        let at_issue: Vec<MigrationId> =
            remote.difference(&local).cloned().collect();
        if !at_issue.is_empty() {
            return Err(TernError::OutOfSync {
                at_issue,
                msg: "local and remote out of sync".into(),
            });
        }
        Ok(())
    }
}
