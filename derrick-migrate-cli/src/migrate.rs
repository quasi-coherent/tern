use crate::opt::MigrationType;

use anyhow::Context;
use regex::Regex;
use std::{fs::File, path::PathBuf, sync::OnceLock};

pub fn new(
    description: String,
    no_tx: bool,
    migration_type: MigrationType,
    path: PathBuf,
) -> anyhow::Result<()> {
    if !path.is_dir() {
        return Err(anyhow::anyhow!(
            "supplied path is not a directory or does not exist"
        ));
    };
    let mut buf = PathBuf::new();
    let latest = get_latest_version(&buf)?;
    let filename = new_filename(&description, latest, migration_type);
    buf.push(path);
    buf.push(filename);

    println!("Creating {}", console::style(buf.display()).cyan());
    let mut file = File::create(&buf).context("Failed to create migration file")?;
    std::io::Write::write_all(
        &mut file,
        migration_template(no_tx, migration_type).as_bytes(),
    )?;

    Ok(())
}

fn filename_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^V(\d+)__(\w+)\.(sql|rs)$").unwrap())
}

fn get_latest_version(source: &PathBuf) -> anyhow::Result<u16> {
    source
        .read_dir()?
        .map(|entry| {
            let e = entry?;
            let filetype = e.file_type()?;
            if filetype.is_file() {
                Ok(e)
            } else {
                Err(anyhow::anyhow!("directory contains non-file types"))
            }
        })
        .try_fold(0, |acc, f| {
            let filename = f?
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("converting filename to str"))?;
            let captures = filename_re()
                .captures(&filename)
                .ok_or(anyhow::anyhow!("{filename} does not match expected format"))?;
            let version = captures
                .get(1)
                .ok_or(anyhow::anyhow!("{filename} missing version"))?
                .as_str()
                .parse::<u16>()?;
            if version > 0 {
                Ok::<u16, anyhow::Error>(version)
            } else {
                Ok::<u16, anyhow::Error>(acc)
            }
        })
}

fn new_filename(descr: &str, version: u16, ty: MigrationType) -> String {
    let descr = descr.to_ascii_lowercase().replace(" ", "_");
    let ver = format!("V{}", version + 1);
    let ext = match ty {
        MigrationType::Rust => ".rs",
        _ => ".sql",
    };
    format!("{ver}__{descr}.{ext}")
}

fn migration_template(no_tx: bool, ty: MigrationType) -> String {
    let mut content = String::new();
    match ty {
        MigrationType::Sql => {
            if no_tx {
                content += "-- derrick:noTransaction";
            }
            content += r#"
-- Add the SQL for the migration below. If this file has a comment
-- on the first line containing `derrick:noTransaction` then the query
-- will not be ran in a transaction.
"#;
        }
        MigrationType::Rust => {
            let migration_annotation = if no_tx {
                "#[migration(no_transaction, runtime = YourMigrationRuntime)]"
            } else {
                "#[migration(runtime = YourMigrationRuntime)]"
            };
            content += &format!(
                r#"
/// This file needs to define one thing: the `build_query` function below.
/// It takes one argument, the runtime context needed to build the query, and it
/// returns the query.
use derrick::{{Error, QueryBuilder}};

use crate::YourMigrationRuntime;

#[derive(QueryBuilder)]
{migration_annotation}
pub struct PlaceholderCanChange;

pub async fn build_query(runtime: &mut YourMigrationRuntime) -> Result<String, Error> {{
    todo!()
}}
"#
            )
        }
    }

    content
}
