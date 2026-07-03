use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue as args;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Scaffold a new Teamy Rust CLI repository from this template.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct InitArgs {
    /// Destination directory to create or update.
    #[facet(args::positional)]
    pub destination: String,

    /// Overwrite existing generated files except `README.md` and `LICENSE`.
    #[facet(args::named, default)]
    pub force: bool,
}

#[derive(Facet, Debug)]
struct InitReport {
    source: String,
    destination: String,
    copied_files: usize,
    overwritten_files: usize,
    skipped_files: usize,
    created_directories: usize,
}

#[derive(Debug)]
struct CopyPlan {
    directories: Vec<PathBuf>,
    files: Vec<FilePlan>,
    skipped_files: usize,
}

#[derive(Debug)]
struct FilePlan {
    source: PathBuf,
    destination: PathBuf,
    action: FileAction,
}

#[derive(Debug)]
enum FileAction {
    Copy,
    Overwrite,
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
        let source = Path::new(env!("CARGO_MANIFEST_DIR"));
        let destination = resolve_destination_path(&self.destination)?;
        let report = scaffold_template(source, &destination, self.force)?;
        Ok(CliOutput::facet(report))
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

fn scaffold_template(source: &Path, destination: &Path, force: bool) -> Result<InitReport> {
    if !source.is_dir() {
        bail!("Template source is not a directory: {}", source.display());
    }

    if destination.exists() && !destination.is_dir() {
        bail!(
            "Destination path exists but is not a directory: {}",
            destination.display()
        );
    }

    let plan = build_copy_plan(source, destination, force)?;
    let mut created_directories = 0;
    for directory in &plan.directories {
        if directory.exists() {
            if !directory.is_dir() {
                bail!(
                    "Destination path exists but is not a directory: {}",
                    directory.display()
                );
            }
            continue;
        }
        fs::create_dir_all(directory)
            .wrap_err_with(|| format!("failed to create directory {}", directory.display()))?;
        created_directories += 1;
    }

    let mut copied_files = 0;
    let mut overwritten_files = 0;
    for file in &plan.files {
        if let Some(parent) = file.destination.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::copy(&file.source, &file.destination).wrap_err_with(|| {
            format!(
                "failed to copy {} to {}",
                file.source.display(),
                file.destination.display()
            )
        })?;
        match file.action {
            FileAction::Copy => copied_files += 1,
            FileAction::Overwrite => overwritten_files += 1,
        }
    }

    Ok(InitReport {
        source: source.display().to_string(),
        destination: destination.display().to_string(),
        copied_files,
        overwritten_files,
        skipped_files: plan.skipped_files,
        created_directories,
    })
}

fn build_copy_plan(source: &Path, destination: &Path, force: bool) -> Result<CopyPlan> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut conflicts = Vec::new();
    let mut skipped_files = 0;
    collect_copy_plan(
        source,
        source,
        destination,
        force,
        &mut directories,
        &mut files,
        &mut conflicts,
        &mut skipped_files,
    )?;

    if !conflicts.is_empty() {
        let shown_conflicts = conflicts
            .iter()
            .take(10)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        let remaining = conflicts.len().saturating_sub(10);
        let suffix = if remaining == 0 {
            String::new()
        } else {
            format!(" and {remaining} more")
        };
        bail!(
            "Destination already contains generated files that would be overwritten: {shown_conflicts}{suffix}. Re-run with --force to overwrite generated files."
        );
    }

    Ok(CopyPlan {
        directories,
        files,
        skipped_files,
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "recursive copy planning carries shared accumulators explicitly"
)]
fn collect_copy_plan(
    root: &Path,
    current: &Path,
    destination_root: &Path,
    force: bool,
    directories: &mut Vec<PathBuf>,
    files: &mut Vec<FilePlan>,
    conflicts: &mut Vec<String>,
    skipped_files: &mut usize,
) -> Result<()> {
    let relative = current
        .strip_prefix(root)
        .wrap_err("template traversal left source root")?;

    if is_excluded_directory(relative) {
        return Ok(());
    }

    directories.push(destination_root.join(relative));

    let mut entries = fs::read_dir(current)
        .wrap_err_with(|| format!("failed to read directory {}", current.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .wrap_err_with(|| format!("failed to read directory entry in {}", current.display()))?;
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let file_type = entry
            .file_type()
            .wrap_err_with(|| format!("failed to inspect {}", entry.path().display()))?;
        let source_path = entry.path();
        let relative_path = source_path
            .strip_prefix(root)
            .wrap_err("template traversal left source root")?;

        if file_type.is_dir() {
            collect_copy_plan(
                root,
                &source_path,
                destination_root,
                force,
                directories,
                files,
                conflicts,
                skipped_files,
            )?;
            continue;
        }

        if !file_type.is_file() || is_excluded_file(relative_path) {
            *skipped_files += 1;
            continue;
        }

        let destination_path = destination_root.join(relative_path);
        if should_preserve_existing_file(relative_path) && destination_path.exists() {
            *skipped_files += 1;
            continue;
        }

        if destination_path.exists() {
            if force && destination_path.is_file() {
                files.push(FilePlan {
                    source: source_path,
                    destination: destination_path,
                    action: FileAction::Overwrite,
                });
            } else {
                conflicts.push(path_for_report(relative_path));
            }
            continue;
        }

        files.push(FilePlan {
            source: source_path,
            destination: destination_path,
            action: FileAction::Copy,
        });
    }

    Ok(())
}

fn is_excluded_directory(relative: &Path) -> bool {
    relative == Path::new(".git")
        || relative == Path::new("target")
        || relative
            == Path::new(".github")
                .join("skills")
                .join("initialize-from-teamy-rust-cli")
}

fn is_excluded_file(relative: &Path) -> bool {
    relative == Path::new("init-other-repo.ps1")
}

fn should_preserve_existing_file(relative: &Path) -> bool {
    relative == Path::new("README.md") || relative == Path::new("LICENSE")
}

fn path_for_report(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
