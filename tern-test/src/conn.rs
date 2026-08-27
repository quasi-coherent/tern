use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU32, Ordering};
use tern_core::context::ConnStr;
use tern_core::error::{TernError, TernResult};

use crate::TestDatabase;

/// Value containing configuration for test DB setup.
///
/// We have to use one connection to create a database for another connection.
/// This type helps orchestrate this.
#[derive(Clone)]
pub struct TestConn {
    base_conn: ConnStr,
    test_conn: ConnStr,
    testdb_name: String,
}

impl TestConn {
    /// Construct the connection info for a test DB instance.
    pub fn new<T: Into<String>>(base_url: T) -> TernResult<Self> {
        let base_conn = ConnStr::new(base_url);
        let base_url = base_conn.try_into_url()?;
        let testdb_name = gen_name();

        if base_conn.scheme().is_some_and(|sch| sch.contains("sqlite")) {
            let path = std::env::temp_dir().join(format!("{testdb_name}.db"));
            let name = path.display().to_string();
            let test_conn = format!("sqlite://{name}").into();
            return Ok(Self { base_conn, test_conn, testdb_name });
        }

        let mut test_url = base_url.clone();
        test_url
            .path_segments_mut()
            .map_err(|_| TernError::Executor("db url missing path".into()))?
            .pop()
            .push(&testdb_name);
        let test_conn = ConnStr::new(test_url.as_str());

        Ok(Self { base_conn, test_conn, testdb_name })
    }

    /// Return the initial DB connection.
    pub fn base_conn(&self) -> &ConnStr {
        &self.base_conn
    }

    /// Return the test DB connection.
    pub fn test_conn(&self) -> &ConnStr {
        &self.test_conn
    }

    /// Return the generated DB name.
    pub fn testdb_name(&self) -> &str {
        &self.testdb_name
    }

    /// Initialize the [`TestDatabase`] executor `T`.
    pub async fn test_connect<T: TestDatabase>(&self) -> TernResult<T> {
        let mut exec = T::connect(&self.base_conn).await?;
        exec.create_database(&self.testdb_name).await?;
        T::connect(&self.test_conn).await
    }
}

impl Debug for TestConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestConn")
            .field("base_conn", &self.base_conn)
            .field("test_conn", &self.test_conn)
            .field("testdb_name", &self.testdb_name)
            .finish()
    }
}

fn gen_name() -> String {
    static N: AtomicU32 = AtomicU32::new(0);
    // TODO: Remove the unwrap before 2262-04-11 23:47:16.854775807 UTC.
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let count = N.fetch_add(1, Ordering::Relaxed);
    format!("tern_t{nanos:06x}{count:02x}")
}
