mod migrate;
mod opt;

pub use opt::Opt;
use opt::{Command, MigrateCommand};

use derrick_core::prelude::*;
use derrick_core::types::HistoryTableInfo;
use derrick_migrate::Runner;

pub async fn run<R, I>(opt: Opt, data: I) -> anyhow::Result<()>
where
    I: Clone,
    R: Runner + Migrate<Init = I>,
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

                let table_info = HistoryTableInfo::default()
                    .set_table_name_if_some(table_name)
                    .set_schema_if_some(schema);
                let history = <R as Migrate>::History::new(&table_info);

                let mut runner = R::new_runner(db_url, history, data).await?;

                let applied = runner.applied().await?;
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

                let table_info = HistoryTableInfo::default()
                    .set_table_name_if_some(table_name)
                    .set_schema_if_some(schema);
                let history = <R as Migrate>::History::new(&table_info);

                let mut runner = R::new_runner(db_url, history, data).await?;

                if dry_run {
                    let unapplied = runner.unapplied().await?;
                    println!("unapplied migrations {:?}", unapplied);
                } else {
                    let applied = runner.run().await?;
                    println!("applied migrations {:?}", applied);
                }

                Ok(())
            }
        },
    }
}
