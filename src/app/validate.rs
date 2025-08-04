use std::collections::HashSet;
use tern_core::context::MigrationContext;
use tern_core::error::{Error, TernResult};
use tern_core::source::MigrationId;

/// Validates the migration set for the given `MigrationContext`.
pub(super) struct Validator<'a, Ctx> {
    context: &'a mut Ctx,
}

impl<'a, Ctx> Validator<'a, Ctx>
where
    Ctx: MigrationContext,
{
    pub(super) fn new(context: &'a mut Ctx) -> Self {
        Self { context }
    }

    /// Combined validation rules.
    pub(super) async fn validate(&mut self, target: Option<i64>) -> TernResult<()> {
        self.context.check_history_table().await?;
        self.validate_source().await?;

        let last_applied = self.context.latest_version().await?;
        self.validate_target(last_applied, target)
    }

    /// Find applied migrations that are not in the source directory.
    async fn validate_source(&mut self) -> TernResult<()> {
        let applied: HashSet<MigrationId> = self
            .context
            .previously_applied()
            .await?
            .into_iter()
            .map(MigrationId::from)
            .collect();
        let source: HashSet<MigrationId> = self
            .context
            .migration_set(None)
            .migration_ids()
            .into_iter()
            .collect();

        check_migrations_in_sync(applied, source)
    }

    /// Check that the target migration version (for some operation) is valid.
    fn validate_target(
        &self,
        last_applied: Option<i64>,
        target_version: Option<i64>,
    ) -> TernResult<()> {
        let Some(source) = self.context.migration_set(None).max() else {
            return Ok(());
        };
        if let Some(target) = target_version {
            match last_applied {
                Some(applied) if target < applied => Err(Error::Invalid(format!(
                    "target version V{target} earlier than latest applied version V{applied}",
                )))?,
                _ if target > source => Err(Error::Invalid(format!(
                    "target version V{target} does not exist, latest version found was V{source}",
                )))?,
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}

/// Migrations that have been applied already but do not exist locally.
fn check_migrations_in_sync(
    applied: HashSet<MigrationId>,
    source: HashSet<MigrationId>,
) -> TernResult<()> {
    let source_not_found: Vec<&MigrationId> = applied.difference(&source).collect();

    if !source_not_found.is_empty() {
        return Err(Error::OutOfSync {
            at_issue: source_not_found.into_iter().cloned().collect(),
            msg: "version/name applied but missing in source".into(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tern_core::error::Error;
    use tern_core::source::MigrationId;

    use std::collections::HashSet;

    #[test]
    fn missing_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "fourth".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
        ]
        .into_iter()
        .collect();
        let missing = vec![MigrationId::new(3, "third".into())];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == missing));
    }

    #[test]
    fn fewer_in_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fourth".into()),
        ]
        .into_iter()
        .collect();
        let missing = vec![MigrationId::new(4, "fourth".into())];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == missing));
    }

    #[test]
    fn mismatched_source() {
        let source: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fifth".into()),
            MigrationId::new(5, "sixth".into()),
            MigrationId::new(6, "seventh".into()),
            MigrationId::new(7, "eighth".into()),
        ]
        .into_iter()
        .collect();
        let applied: HashSet<MigrationId> = vec![
            MigrationId::new(1, "first".into()),
            MigrationId::new(2, "second".into()),
            MigrationId::new(3, "third".into()),
            MigrationId::new(4, "fourth".into()),
            MigrationId::new(5, "fifth".into()),
        ]
        .into_iter()
        .collect();
        let divergence = vec![
            MigrationId::new(4, "fourth".into()),
            MigrationId::new(5, "fifth".into()),
        ];
        let result = super::check_migrations_in_sync(applied, source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::OutOfSync { at_issue, .. } if *at_issue == divergence));
    }
}
