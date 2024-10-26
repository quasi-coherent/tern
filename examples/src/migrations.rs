use derrick::embed_migrations;
use derrick::sqlx_postgres::SqlxPgMigrate;

embed_migrations!("src/migrations", SqlxPgMigrate);
