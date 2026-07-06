//! Dynamic up migration in an up/down pair; the down side is plain SQL.
use tern::error::TernResult;
use tern::{Migration, Query, ResolveMigration};

use super::SqliteUpDown;

#[derive(Migration)]
pub struct SqliteUpDownCreateC {
    rows: i64,
}

impl ResolveMigration for SqliteUpDownCreateC {
    type Ctx = SqliteUpDown;

    async fn init(_ctx: &mut Self::Ctx) -> TernResult<Self> {
        Ok(Self { rows: 2 })
    }

    async fn resolve(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
        let mut builder = Query::builder();
        builder.push_sql("CREATE TABLE tern_sqlite_ud_c (x INTEGER);");
        for i in 1..=self.rows {
            builder.push_sql(format!(
                "INSERT INTO tern_sqlite_ud_c VALUES ({i});"
            ));
        }
        builder.build().read_query()
    }
}
