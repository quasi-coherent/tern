//! Dynamic up migration in an up/down pair; the down side is plain SQL.
use tern::error::TernResult;
use tern::{Migration, Query, ResolveMigration};

use super::MySqlUpDown;

#[derive(Migration)]
pub struct MySqlUpDownCreateC {
    rows: i64,
}

impl ResolveMigration for MySqlUpDownCreateC {
    type Ctx = MySqlUpDown;

    async fn init(_ctx: &mut Self::Ctx) -> TernResult<Self> {
        Ok(Self { rows: 2 })
    }

    async fn resolve(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
        let mut builder = Query::builder();
        builder.push_sql("CREATE TABLE tern_mysql_ud_c (x BIGINT);");
        for i in 1..=self.rows {
            builder
                .push_sql(format!("INSERT INTO tern_mysql_ud_c VALUES ({i});"));
        }
        builder.build().read_query()
    }
}
