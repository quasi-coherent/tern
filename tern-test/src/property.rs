//! Properties certifying the effect of migrations.
use futures_core::future::{Future, LocalBoxFuture};
use std::collections::BTreeMap;
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;

/// A migration `Property` is a pair of user-supplied invariants attached to it.
pub trait Property<Ctx: MigrationContext> {
    /// Evaluate a condition with the migration context before applying this
    /// migration.
    fn pre_check(&self, ctx: &mut Ctx) -> impl Future<Output = TernResult<()>>;

    /// Evaluate a condition with the migration context after applying this
    /// migration.
    fn post_check(&self, ctx: &mut Ctx)
    -> impl Future<Output = TernResult<()>>;
}

/// Dynamically-typed `Property`.
pub struct Prop<Ctx>(Box<dyn BoxedProperty<Ctx>>);

impl<Ctx: MigrationContext> Prop<Ctx> {
    /// New.
    pub fn new<P: Property<Ctx> + 'static>(prop: P) -> Self {
        Self(Box::new(prop))
    }
}

impl<Ctx: MigrationContext> Property<Ctx> for Prop<Ctx> {
    async fn pre_check(&self, ctx: &mut Ctx) -> TernResult<()> {
        self.0.pre_check_box(ctx).await
    }

    async fn post_check(&self, ctx: &mut Ctx) -> TernResult<()> {
        self.0.post_check_box(ctx).await
    }
}

/// A mapping from migration version to property to certify for the migration.
pub trait PropertySet<Ctx: MigrationContext> {
    /// The property declared for `version`, if any.
    fn property(&self, version: i64) -> Option<&Prop<Ctx>>;
}

impl<Ctx: MigrationContext> PropertySet<Ctx> for () {
    fn property(&self, _: i64) -> Option<&Prop<Ctx>> {
        None
    }
}

/// A collection of `Property` assertions.
#[derive(Default)]
pub struct Properties<Ctx: MigrationContext>(BTreeMap<i64, Prop<Ctx>>);

impl<Ctx: MigrationContext> Properties<Ctx> {
    /// New empty set.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Declare `property` for the pair at `version`.
    pub fn with(
        mut self,
        version: i64,
        prop: impl Property<Ctx> + Send + 'static,
    ) -> Self {
        self.0.insert(version, Prop::new(prop));
        self
    }
}

impl<Ctx: MigrationContext> PropertySet<Ctx> for Properties<Ctx> {
    fn property(&self, version: i64) -> Option<&Prop<Ctx>> {
        self.0.get(&version)
    }
}

// Object-safe version.
trait BoxedProperty<Ctx: MigrationContext> {
    fn pre_check_box<'a>(
        &'a self,
        ctx: &'a mut Ctx,
    ) -> LocalBoxFuture<'a, TernResult<()>>;

    fn post_check_box<'a>(
        &'a self,
        ctx: &'a mut Ctx,
    ) -> LocalBoxFuture<'a, TernResult<()>>;
}

impl<Ctx: MigrationContext, P: Property<Ctx>> BoxedProperty<Ctx> for P {
    fn pre_check_box<'a>(
        &'a self,
        ctx: &'a mut Ctx,
    ) -> LocalBoxFuture<'a, TernResult<()>> {
        Box::pin(self.pre_check(ctx))
    }

    fn post_check_box<'a>(
        &'a self,
        ctx: &'a mut Ctx,
    ) -> LocalBoxFuture<'a, TernResult<()>> {
        Box::pin(self.post_check(ctx))
    }
}
