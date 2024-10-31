mod migrate;
mod opt;

pub use opt::Opt;
use opt::{Command, MigrateCommand};

use derrick_migrate::MigrationRuntime;

pub async fn cli<R>(opt: Opt) -> anyhow::Result<()>
where
    R: MigrationRuntime,
{
    match opt.command {
        Command::Migrate(migrate) => match migrate.command {
            MigrateCommand::New {
                description,
                no_tx,
                migration_type,
                source,
            } => migrate::new(description, no_tx, migration_type, source.path),
            MigrateCommand::List { connect_opts } => {
                let db_url = connect_opts.required_db_url()?.to_string();
                let table_name = connect_opts.history_table;
                let schema = connect_opts.history_schema;
                let mut runtime = R::init(db_url, schema, table_name).await?;

                let applied = runtime.applied().await?;
                println!("{:?}", applied);

                Ok(())
            }
            MigrateCommand::Run {
                dry_run,
                connect_opts,
            } => {
                let db_url = connect_opts.required_db_url()?.to_string();
                let table_name = connect_opts.history_table;
                let schema = connect_opts.history_schema;
                let mut runtime = R::init(db_url, schema, table_name).await?;

                if dry_run {
                    let unapplied = runtime.unapplied().await?;
                    println!("unapplied migrations {:?}", unapplied);
                } else {
                    let applied = runtime.run().await?;
                    println!("applied migrations {:?}", applied);
                }

                Ok(())
            }
        },
    }
}
