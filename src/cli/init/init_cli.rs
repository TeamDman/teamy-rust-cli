use super::init_mode::InitMode;
use super::init_request::InitRequest;
use super::overwrite_policy::OverwritePolicy;
use super::project_identity::ProjectIdentity;
use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue as args;
use std::path::Path;
use std::path::PathBuf;

/// Scaffold a new Teamy Rust CLI repository from this template.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct InitArgs {
    /// Destination directory to create or update.
    #[facet(args::positional)]
    pub destination: Option<String>,

    /// Crate and binary identity. With no destination, create NAME in the current directory.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub name: Option<String>,

    /// Explicit repository metadata only; does not create or publish a GitHub repository.
    #[facet(args::named)]
    #[arbitrary(default)]
    pub repository_url: Option<String>,

    /// Return the complete copy plan without creating files or directories.
    #[facet(args::named, default)]
    pub dry_run: bool,

    /// Overwrite existing generated files except `README.md` and `LICENSE`.
    #[facet(args::named, default)]
    pub force: bool,
}

impl InitArgs {
    /// # Errors
    ///
    /// This function will return an error if the template cannot be copied, or if destination files would be overwritten without `--force`.
    #[expect(
        clippy::unused_async,
        reason = "command invoke methods share the async CLI dispatch shape"
    )]
    pub async fn invoke(self) -> Result<CliOutput> {
        let report = self
            .into_request()?
            .run(Path::new(env!("CARGO_MANIFEST_DIR")))?;
        let budget = Some(
            std::time::Duration::from_secs(3).saturating_add(
                std::time::Duration::from_millis(20)
                    .saturating_mul(u32::try_from(report.files.len()).unwrap_or(u32::MAX)),
            ),
        );
        Ok(CliOutput::facet(report, budget))
    }
    fn into_request(self) -> Result<InitRequest> {
        let destination = match (&self.destination, &self.name) {
            (Some(destination), _) => resolve_destination_path(destination)?,
            (None, Some(name)) => {
                ProjectIdentity::validate(name, self.repository_url.as_deref())?;
                resolve_destination_path(name)?
            }
            (None, None) => bail!("init requires a destination or --name"),
        };
        let name = self
            .name
            .as_deref()
            .or_else(|| destination.file_name().and_then(std::ffi::OsStr::to_str))
            .ok_or_else(|| eyre::eyre!("cannot infer project name"))?;
        let identity = ProjectIdentity::validate(name, self.repository_url.as_deref())?;
        Ok(InitRequest {
            destination,
            identity,
            overwrite: if self.force {
                OverwritePolicy::ReplaceGenerated
            } else {
                OverwritePolicy::RejectExisting
            },
            mode: if self.dry_run {
                InitMode::Preview
            } else {
                InitMode::Apply
            },
        })
    }
}

fn resolve_destination_path(raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(std::env::current_dir()
        .wrap_err("failed to resolve current directory")?
        .join(path))
}
