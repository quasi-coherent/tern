//! Dynamic migration whose INSERT statements are resolved at apply time.
use tern::error::TernResult;
use tern::{Migration, Query, ResolveMigration};

use super::PgPlain;

#[derive(Migration)]
pub struct PgPlainInsertRows {
    rows: i64,
}

impl ResolveMigration for PgPlainInsertRows {
    type Ctx = PgPlain;

    async fn init(_ctx: &mut Self::Ctx) -> TernResult<Self> {
        Ok(Self { rows: 3 })
    }

    async fn resolve(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
        let mut builder = Query::builder();
        for i in 1..=self.rows {
            builder.push_sql(format!(
                "INSERT INTO tern_pg_plain (x, y) VALUES ({i}, 'row{i}');"
            ));
        }
        builder.build().read_query()
    }
}
