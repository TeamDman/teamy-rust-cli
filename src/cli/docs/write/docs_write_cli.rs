use crate::cli::docs::generate_help_snapshots;
use arbitrary::Arbitrary;
use eyre::Result;
use eyre::WrapErr;
use facet::Facet;
use figue as args;

/// Write generated command help docs to files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct DocsWriteArgs {
    /// Output directory under which `command-help/*.txt` is written.
    #[facet(args::positional)]
    #[arbitrary(default)]
    pub path: String,
}

impl DocsWriteArgs {
    /// # Errors
    ///
    /// This function will return an error if writing docs files fails.
    pub async fn invoke(self) -> Result<()> {
        let output_path = std::path::PathBuf::from(self.path);
        let (docs_dir, file_count) =
            tokio::task::spawn_blocking(move || -> Result<(std::path::PathBuf, usize)> {
                let snapshots = generate_help_snapshots()?;
                let docs_dir = output_path.join("command-help");
                std::fs::create_dir_all(&docs_dir).wrap_err_with(|| {
                    format!("Failed to create docs directory {}", docs_dir.display())
                })?;

                for snapshot in &snapshots {
                    let file_path = docs_dir.join(&snapshot.file_name);
                    let content = format!(
                        "$ {}\n\n{}",
                        snapshot.invocation,
                        snapshot.output.trim_end()
                    );
                    std::fs::write(&file_path, content)
                        .wrap_err_with(|| format!("Failed to write {}", file_path.display()))?;
                }

                Ok((docs_dir, snapshots.len()))
            })
            .await
            .map_err(|err| eyre::eyre!("docs write task failed: {err}"))??;

        println!("Wrote {file_count} files under {}", docs_dir.display());
        Ok(())
    }
}
