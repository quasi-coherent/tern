#[cfg(feature = "sqlx_mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_mysql")))]
pub mod mysql;

#[cfg(feature = "sqlx")]
#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "sqlx_mysql",
        feature = "sqlx_postgres",
        feature = "sqlx_sqlite"
    )))
)]
pub mod pool;

#[cfg(feature = "sqlx_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_postgres")))]
pub mod postgres;

#[cfg(feature = "sqlx_sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx_sqlite")))]
pub mod sqlite;
