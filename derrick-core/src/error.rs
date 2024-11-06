use crate::migrations::migration::Migration;

use std::error::Error as StdError;

type BoxDynError = Box<dyn StdError + 'static + Send + Sync>;

/// All the ways the lifecycle of applying migrations
/// can end in failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An error that came from applying migrations.
    #[error("error applying migrations: {0}")]
    Execute(#[source] BoxDynError),
    /// Error from one migration.
    #[error("error applying migration {1}: {0}")]
    ExecuteMigration(#[source] BoxDynError, i64),
    /// Error resolving query before migration.
    #[error("runtime could not resolve query: {0}")]
    ResolveQuery(String),
    #[error("could not parse migration query: {0}")]
    Sql(#[from] std::fmt::Error),
    /// A migration was found in the history table but not
    /// in the directory of migrations.
    #[error("migration {0} applied before but missing in resolved migrations")]
    VersionMissing(i64),
    /// A migration exists in the source directory
    /// and the history table, but there is a mismatch
    /// of checksums generated from the migration content.
    #[error("migration {0} modified since apply: applied {1}, source {2}")]
    VersionModified(i64, String, String),
    /// A version specified does not exist in the source.
    #[error("version {0} not found in source")]
    VersionNotPresent(i64),
    /// A version specified is older than would be valid
    /// for the operation requested.
    #[error("version {0} too old for the requested operation")]
    VersionTooOld(i64),
    /// A version specified is newer than would be valid
    /// for the operation requested.
    #[error("version {0} too new for the requested operation")]
    VersionTooNew(i64),
}

/// To support a generic database backend error and
/// still keep `self::Error` as the core error type.
pub trait DatabaseError<T, E> {
    fn into_error(self) -> Result<T, Error>;
    fn into_error_with(self, m: &Migration) -> Result<T, Error>;
    fn into_error_void(self) -> Result<(), Error>;
    fn into_error_with_void(self, m: &Migration) -> Result<(), Error>;
}

impl<T, E> DatabaseError<T, E> for Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn into_error(self) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Error::Execute(Box::new(e))),
        }
    }

    fn into_error_with(self, m: &Migration) -> Result<T, Error> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Error::ExecuteMigration(Box::new(e), m.version)),
        }
    }

    fn into_error_void(self) -> Result<(), Error> {
        match self {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Execute(Box::new(e))),
        }
    }

    fn into_error_with_void(self, m: &Migration) -> Result<(), Error> {
        match self {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::ExecuteMigration(Box::new(e), m.version)),
        }
    }
}
