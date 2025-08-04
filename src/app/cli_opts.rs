use crate::app::{Report, Tern, TernOp};
use crate::cli::ConnectContext;

use tern_cli::{CliOpts, HistoryOpts, MigrateOpts, Opts};
use tern_core::context::MigrationContext;
use tern_core::error::{Error, TernResult};

impl<Ctx: MigrationContext> Tern<Ctx> {
    /// Run this `Tern` application directly by parsing CLI options to get
    /// arguments for the operation and for creating the `MigrationContext` from
    /// a connection string using the implementation of a
    /// [`ConnectContext`][conn].
    ///
    /// [`ConnectContext`]: crate::cli::ConnectContxt
    #[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
    pub async fn run_cli<T>(conn: T) -> TernResult<Option<Report>>
    where
        T: ConnectContext<Ctx = Ctx>,
    {
        let cli = CliOpts::new();

        match cli.opts {
            Opts::Migrate(migrate) => match migrate.opts {
                MigrateOpts::ListApplied { connect_opts } => {
                    let db_url = connect_opts
                        .required_db_url()
                        .map_err(|e| Error::Invalid(e.to_string()))?;

                    let context = conn.connect(db_url).await?;
                    let mut app = Tern::new(context);

                    app.run().await
                }
                MigrateOpts::Apply {
                    dryrun,
                    target_version,
                    connect_opts,
                } => {
                    let db_url = connect_opts
                        .required_db_url()
                        .map_err(|e| Error::Invalid(e.to_string()))?;

                    let context = conn.connect(db_url).await?;

                    let mut app = match target_version {
                        Some(v) => Tern::new(context).with_operation(TernOp::ApplyThrough(v)),
                        _ => Tern::new(context).with_operation(TernOp::ApplyAll),
                    };

                    if dryrun {
                        app = app.dryrun();
                    }

                    app.run().await
                }
                MigrateOpts::SoftApply {
                    dryrun,
                    target_version,
                    connect_opts,
                } => {
                    let db_url = connect_opts
                        .required_db_url()
                        .map_err(|e| Error::Invalid(e.to_string()))?;

                    let context = conn.connect(db_url).await?;

                    let mut app = match target_version {
                        Some(v) => Tern::new(context).with_operation(TernOp::SoftApplyThrough(v)),
                        _ => Tern::new(context).with_operation(TernOp::SoftApplyAll),
                    };

                    if dryrun {
                        app = app.dryrun();
                    }

                    app.run().await
                }
            },
            Opts::History(history) => match history.opts {
                HistoryOpts::Init { connect_opts } => {
                    let db_url = connect_opts
                        .required_db_url()
                        .map_err(|e| Error::Invalid(e.to_string()))?;

                    let context = conn.connect(db_url).await?;

                    let mut app = Tern::new(context).with_operation(TernOp::InitHistory);

                    app.run().await
                }
                HistoryOpts::Drop { connect_opts } => {
                    let db_url = connect_opts
                        .required_db_url()
                        .map_err(|e| Error::Invalid(e.to_string()))?;

                    let context = conn.connect(db_url).await?;

                    let mut app = Tern::new(context).with_operation(TernOp::DropHistory);

                    app.run().await
                }
            },
        }
    }
}
