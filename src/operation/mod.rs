//! Core operations with a set of migrations.
use futures_core::Future;
use std::fmt::Display;
use tern_core::error::{TernError, TernResult};

use crate::migrate::TernMigrate;
use crate::report::Report;

mod history;
pub use history::{Drop, Init};

mod migrate;
pub use migrate::{Apply, Revert, SoftApply};

mod source;
pub use source::{Diff, List};

/// An operation for `TernMigrate` to perform.
pub trait TernMigrateOp<T: TernMigrate>: Display {
    /// The type of output when the operation was successful.
    type Output;

    /// Execute the operation.
    fn exec(
        &self,
        migrate: &mut T,
    ) -> impl Future<Output = Report<Self::Output>> + Send;
}

fn check_range(from: Option<i64>, to: Option<i64>) -> TernResult<()> {
    let f = from.unwrap_or(i64::MIN);
    let t = to.unwrap_or(i64::MAX);
    if f >= t {
        return Err(TernError::Invalid(format!(
            "`from` not less than `to`: {from:?}, {to:?}"
        )));
    }
    Ok(())
}
