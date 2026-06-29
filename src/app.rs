//! The entrypoint to a migration application.
use futures_core::future::Future;
use std::ops::{Deref, DerefMut};
use tern_core::context::MigrationContext;
use tern_core::error::{TernError, TernResult};
use tern_core::migration::{DownMigrationSet, UpMigrationSet};

use crate::ops::{self, MigrationOp, OpResult, OpSuccess};

/// `TernApp` combines a `MigrationContext` and an associated migration set.
pub trait TernApp: MigrationContext {
    /// Migration set to construct the target database.
    fn up_migrations(&self) -> UpMigrationSet<Self>
    where
        Self: Sized;

    /// Migration set to deconstruct the target database.
    ///
    /// If the migration source consists of up/down pairs, the returned value
    /// will be non-`None`. This means that every migration has two files with
    /// the same version and name, with prefix "U" or "D" respectively, e.g.,
    /// `U4__something.sql` and `D4__something.rs` or `U19__something_else.sql`
    /// and `D19__something_else.sql`.
    ///
    /// The up/down pair may be any combination of .rs or .sql.
    fn down_migrations(&self) -> Option<DownMigrationSet<Self>>
    where
        Self: Sized,
    {
        None
    }
}

/// Options to create a `MigrationContext`.
pub trait ContextOptions<Ctx: MigrationContext> {
    /// Initialize the `MigrationContext`.
    fn initialize(self) -> impl Future<Output = TernResult<Ctx>> + Send;
}

/// Tern app container.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Tern<T = ()>(pub(crate) T);

impl<T: TernApp> Tern<T> {
    /// New `Tern`.
    pub fn new(inner: T) -> Tern<T> {
        Tern(inner)
    }

    /// Use `ContextOptions` to initialize an app.
    pub async fn initialize<U>(options: U) -> TernResult<Tern<T>>
    where
        U: ContextOptions<T>,
    {
        let inner = options.initialize().await?;
        Ok(Self(inner))
    }

    /// `List` operation.
    pub async fn list(&mut self, args: ops::source::ListArgs) -> OpResult {
        let set = self.up_migrations();
        let input = ops::source::ListInput::new(set, args);
        ops::source::List::new(&mut self.0).exec(input).await
    }

    /// `Show` operation.
    pub async fn show(&mut self, args: ops::source::ShowArgs) -> OpResult {
        let set = self.up_migrations();
        let input = ops::source::ShowInput::new(set, args);
        let success = ops::source::Show::new(&mut self.0)
            .exec(input)
            .await
            .map(OpSuccess::from)?;
        Ok(success.into())
    }

    /// `Apply` operation.
    pub async fn apply(&mut self, args: ops::migrate::ApplyArgs) -> OpResult {
        let set = self.up_migrations();
        let input = ops::migrate::ApplyInput::new(set, args);
        ops::migrate::Apply::new(&mut self.0).exec(input).await
    }

    /// `Revert` operation.
    pub async fn revert(&mut self, args: ops::migrate::RevertArgs) -> OpResult {
        let set =
            self.down_migrations().ok_or_else(|| TernError::MissingDown)?;
        let input = ops::migrate::RevertInput::new(set, args);
        ops::migrate::Revert::new(&mut self.0).exec(input).await
    }

    /// `Init` operation to start a new project.
    pub async fn init(&mut self) -> TernResult<()> {
        ops::history::Init::new(&mut self.0).exec(()).await
    }

    /// `DropIfExists` operation to delete a migration project.
    pub async fn drop_if_exists(&mut self) -> TernResult<()> {
        ops::history::DropIfExists::new(&mut self.0).exec(()).await
    }
}

impl<T> Deref for Tern<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Tern<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
