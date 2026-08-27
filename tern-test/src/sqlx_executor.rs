//! Test executors for `sqlx`.
//!
//! This module implements the main traits [`TestExecutor`] and
//! [`TestExecutorOptions`] for the `sqlx` types from `tern-executor`.
//!
//! [`TestExecutor`]: crate::executor::TestExecutor
//! [`TestExecutorOptions`]: crate::executor::TestExecutorOptions
use sqlx::Executor as _;
use tern_core::error::TernResult;
use tern_executor::sqlx_executor::SqlxError;

use crate::TestDatabase;

#[cfg(feature = "sqlx_mysql")]
mod mysql {
    use tern_executor::sqlx_executor::SqlxMySqlExecutor;

    use super::*;

    impl TestDatabase for SqlxMySqlExecutor {
        async fn create_database(&mut self, dbname: &str) -> TernResult<()> {
            let sql = format!("CREATE DATABASE `{dbname}`;");
            log::trace!("running {sql}");
            self.inner_mut()
                .execute(sqlx::raw_sql(&sql))
                .await
                .map_err(SqlxError::from)?;
            Ok(())
        }

        async fn drop_database(&mut self, dbname: &str) -> TernResult<()> {
            let sql = format!("DROP DATABASE IF EXISTS `{dbname}`;");
            log::trace!("running {sql}");
            self.inner_mut()
                .execute(sqlx::raw_sql(&sql))
                .await
                .map_err(SqlxError::from)?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlx_postgres")]
mod psql {
    use tern_executor::sqlx_executor::SqlxPgExecutor;

    use super::*;

    impl TestDatabase for SqlxPgExecutor {
        async fn create_database(&mut self, dbname: &str) -> TernResult<()> {
            let sql = format!(r#"CREATE DATABASE "{dbname}";"#);
            log::trace!("running {sql}");
            self.inner_mut()
                .execute(sqlx::raw_sql(&sql))
                .await
                .map_err(SqlxError::from)?;
            Ok(())
        }

        async fn drop_database(&mut self, dbname: &str) -> TernResult<()> {
            let sql =
                format!(r#"DROP DATABASE IF EXISTS "{dbname}" WITH (FORCE);"#);
            log::trace!("running {sql}");
            self.inner_mut()
                .execute(sqlx::raw_sql(&sql))
                .await
                .map_err(SqlxError::from)?;
            Ok(())
        }
    }
}

#[cfg(feature = "sqlx_sqlite")]
mod sqlite {
    use std::path::PathBuf;
    use tern_core::error::TernError;
    use tern_executor::sqlx_executor::SqlxSqliteExecutor;

    use super::*;

    impl TestDatabase for SqlxSqliteExecutor {
        async fn create_database(&mut self, _: &str) -> TernResult<()> {
            // This is created on connection.
            Ok(())
        }

        async fn drop_database(&mut self, dbname: &str) -> TernResult<()> {
            // "Dropping" a sqlite DB amounts to deleting the file plus its
            // ancillary data files (dbname-wal and dbname-shm):
            ["", "-wal", "-shm"]
                .iter()
                .map(|suffix| {
                    let f = PathBuf::from(format!("{dbname}{suffix}"));
                    std::fs::remove_file(f)
                })
                .filter_map(|res| {
                    if res.as_ref().is_err_and(|e| {
                        matches!(e.kind(), std::io::ErrorKind::NotFound)
                    }) {
                        return None;
                    }
                    Some(res.map_err(TernError::exec_err))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(())
        }
    }
}
