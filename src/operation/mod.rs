use futures_core::Future;
use tern_core::error::TernResult;

use crate::migrate::TernMigrate;
use crate::report::Report;

/// An operation for `TernMigrate` to perform.
#[allow(unused)]
pub trait TernOp<T: TernMigrate> {
    /// Required arguments for this operation.
    type Args;

    /// Execute the operation, returning the `Report` of its result.
    fn exec(
        &self,
        migrate: &mut T,
        args: Self::Args,
    ) -> impl Future<Output = TernResult<Report>> + Send;
}
