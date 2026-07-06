//! The entrypoint to a migration application.
use futures_core::future::Future;
use tern_core::context::{ConnStr, MigrationContext, MigrationExecutor};
use tern_core::error::TernResult;
use tern_core::migration::Migration;
use tern_core::ops::Operation as _;

use crate::ops::{self, OpResult};

#[cfg(feature = "cli")]
pub mod cli;

/// `TernApp` combines a `MigrationContext` and an associated migration set.
pub trait TernApp: MigrationContext + Sized {
    /// The type of value that can construct this app.
    type Options: AppOptions<App = Self>;

    /// Migration set for this app.
    ///
    /// This is expressed as a `DoubleEndedIterator` whose item type implements
    /// the `Migration` interface.  Using the `Iterator` is meant to construct
    /// the database (i.e., these are the "up" migrations), while the
    /// `DoubleEndedIterator` deconstructs it.
    ///
    /// The `DoubleEndedIterator` impl is still required even if the migration
    /// set does not have up/down pairs; use [`std::iter::Empty`] as the type
    /// in this case.
    type Set: DoubleEndedIterator<Item: Migration<Ctx = Self> + 'static>;

    /// Return the [`MigrationSet`] for this app.
    fn new(&mut self) -> Self::Set;
}

/// Options to create a `TernApp`.
///
/// This is simply any custom configuration for the context plus its executor.
pub trait AppOptions {
    /// The [`TernApp`] that these options build.
    type App: TernApp<Options = Self>;

    /// Initialize the `TernApp`.
    fn initialize(
        self,
        exec: <Self::App as MigrationContext>::Executor,
    ) -> impl Future<Output = TernResult<Self::App>> + Send;
}

/// Tern app container.
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Tern<T>(pub(crate) T);

impl<T> Tern<T>
where
    T: TernApp + 'static,
    <T::Set as Iterator>::Item: Migration<Ctx = T> + 'static,
{
    /// New `Tern` app.
    pub fn new(inner: T) -> Tern<T>
    where
        T: TernApp,
    {
        Tern(inner)
    }

    /// Use `AppOptions` to initialize an app.
    pub async fn init(conn: &ConnStr, opts: T::Options) -> TernResult<Tern<T>> {
        let exec = <T as MigrationContext>::Executor::connect(conn).await?;
        let this = opts.initialize(exec).await?;
        Ok(Self(this))
    }

    /// `List` operation.
    pub async fn list(&mut self, args: ops::source::ListArgs) -> OpResult {
        let input = ops::source::ListInput::new(self.0.new(), args);
        ops::source::List::new(&mut self.0).exec(input).await
    }

    /// `Show` operation.
    pub async fn show(&mut self, args: ops::source::ShowArgs) -> OpResult {
        let input = ops::source::ShowInput::new(self.0.new(), args);
        ops::source::Show::new(&mut self.0).exec(input).await
    }

    /// `Apply` operation.
    pub async fn apply(&mut self, args: ops::migrate::ApplyArgs) -> OpResult {
        let input = ops::migrate::ApplyInput::new(self.0.new(), args);
        ops::migrate::Apply::new(&mut self.0).exec(input).await
    }

    /// `Revert` operation.
    pub async fn revert(&mut self, args: ops::migrate::RevertArgs) -> OpResult {
        let input = ops::migrate::RevertInput::new(self.0.new(), args);
        ops::migrate::Revert::new(&mut self.0).exec(input).await
    }

    /// `New` operation to start a new project.
    pub async fn create_if_not_exists(&mut self) -> TernResult<()> {
        let history = self.0.history_table();
        ops::history::CreateIfNotExists::new(&mut self.0).exec(history).await
    }

    /// `DropIfExists` operation to delete a migration project.
    pub async fn drop_if_exists(&mut self) -> TernResult<()> {
        let history = self.0.history_table();
        ops::history::DropIfExists::new(&mut self.0).exec(history).await
    }
}
