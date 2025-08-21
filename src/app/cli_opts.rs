use crate::app::{Report, Tern};

use tern_cli::{CliOpts, ConnectOptions, HistoryOpts, MigrateOpts, Opts};
use tern_core::context::MigrationContext;
use tern_core::error::TernResult;

impl<Ctx: MigrationContext> Tern<Ctx> {
    /// Run this [Tern] application directly by parsing CLI options to get
    /// arguments for the operation and for creating a [MigrationContext].
    ///
    /// [Tern]: crate::app::Tern
    /// [MigrationContext]: tern_core::context::MigrationContext
    #[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
    pub async fn run_cli<C>() -> TernResult<Option<Report>>
    where
        C: ConnectOptions<Ctx = Ctx>,
    {
        let cli = CliOpts::<C>::new();
        let conn = cli.connect_opts;
        let context = conn.connect().await?;

        match cli.opts {
            Opts::Migrate(migrate) => match migrate.opts {
                MigrateOpts::ListApplied => {
                    Tern::builder()
                        .list_applied()
                        .build_with_context(context)
                        .run()
                        .await
                }
                MigrateOpts::Apply {
                    dry_run,
                    skip_validate,
                    target_version,
                } => {
                    let mut builder = Tern::builder().apply();

                    if dry_run {
                        builder = builder.dry_run();
                    }
                    if skip_validate {
                        builder = builder.skip_validate();
                    }
                    if let Some(v) = target_version {
                        builder = builder.with_target_version(v);
                    }

                    builder.build_with_context(context).run().await
                }
                MigrateOpts::SoftApply {
                    dry_run,
                    skip_validate,
                    target_version,
                } => {
                    let mut builder = Tern::builder().soft_apply();

                    if dry_run {
                        builder = builder.dry_run();
                    }
                    if skip_validate {
                        builder = builder.skip_validate();
                    }
                    if let Some(v) = target_version {
                        builder = builder.with_target_version(v);
                    }

                    builder.build_with_context(context).run().await
                }
            },
            Opts::History(history) => match history.opts {
                HistoryOpts::Init => {
                    Tern::builder()
                        .init_history()
                        .build_with_context(context)
                        .run()
                        .await
                }
                HistoryOpts::Drop => {
                    Tern::builder()
                        .drop_history()
                        .build_with_context(context)
                        .run()
                        .await
                }
            },
        }
    }
}
