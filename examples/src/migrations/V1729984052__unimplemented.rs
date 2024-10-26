use derrick::prelude::*;
use derrick::sqlx_postgres::SqlxPgMigrate;
use derrick::Error;
use derrick::QueryBuilder;

#[derive(QueryBuilder)]
#[migration(no_transaction)]
pub struct Unimplemented(SqlxPgMigrate);

impl Unimplemented {
    async fn build_query(migrate: &mut SqlxPgMigrate) -> Result<String, Error> {
        let table = <SqlxPgMigrate as Migrate>::Table::new(&derrick::types::HistoryTableInfo::new(
            None, None,
        ));
        let rows = migrate.get_history_table(&table).await?;
        let sql = "SELECT 1;".to_string();

        Ok(sql)
    }
}
