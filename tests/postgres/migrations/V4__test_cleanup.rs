use super::TestSqlxPgContext;

use tern::error::TernResult;
use tern::{Migration, Query, QueryBuilder};

#[derive(Migration)]
#[tern(no_transaction)]
pub struct TernMigration;

impl QueryBuilder for TernMigration {
    type Ctx = TestSqlxPgContext;

    async fn build(&self, _ctx: &mut Self::Ctx) -> TernResult<Query> {
        Ok(Query::new("DROP TABLE pg_test.users CASCADE;".into()))
    }
}
