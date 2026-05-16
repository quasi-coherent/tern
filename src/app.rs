use std::ops::{Deref, DerefMut};
use tern_core::migrate::{TernMigrate, TernMigrateOp};

/// `TernMigrate` app container.
pub struct Tern<T: ?Sized>(T);

impl<T: TernMigrate> Tern<T> {
    /// New from a `TernMigrate`.
    pub fn new(migrate: T) -> Tern<T> {
        Tern(migrate)
    }

    /// Run the operation and return the output.
    pub async fn exec<Op: TernMigrateOp<T>>(
        &mut self,
        op: &Op,
    ) -> Result<Op::Success, Op::Error>
    where
        T: TernMigrate,
    {
        op.exec(&mut *self).await
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
