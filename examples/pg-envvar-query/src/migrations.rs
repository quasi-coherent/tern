use derrick::embed_migrations;

use crate::ExampleMigrate;

embed_migrations!("src/migrations", ExampleMigrate);
