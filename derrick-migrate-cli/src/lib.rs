mod migrate;
mod opt;

pub use opt::Opt;
use opt::{Command, MigrateCommand};

use derrick_core::prelude::*;
use derrick_core::types::HistoryTableOptions;
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

                let table_options = HistoryTableOptions::default().set_name(table_name);
                let history = <R as Migrate>::History::new(&table_options);
                let mut runner = R::new_runner(db_url, history, data).await?;

                let report = runner.list().await?;
                report.show();

                Ok(())
            }
            MigrateCommand::Validate { connect_opts } => {
                let db_url = connect_opts.required_db_url()?.to_string();
                let table_name = connect_opts.history_table;

                let table_options = HistoryTableOptions::default().set_name(table_name);
                let history = <R as Migrate>::History::new(&table_options);
                let mut runner = R::new_runner(db_url, history, data).await?;

                runner.validate().await?;

                Ok(())
            }
            MigrateCommand::Run {
                dry_run,
                connect_opts,
            } => {
                let db_url = connect_opts.required_db_url()?.to_string();
                let table_name = connect_opts.history_table;

                let table_options = HistoryTableOptions::default().set_name(table_name);
                let history = <R as Migrate>::History::new(&table_options);
                let mut runner = R::new_runner(db_url, history, data).await?;

                let report = if dry_run {
                    runner.dryrun().await?
                } else {
                    runner.run().await?
                };
                report.show();

                Ok(())
            }
        },
    }
}
