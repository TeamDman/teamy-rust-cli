use super::file_action::FileAction;
use super::file_plan::FilePlan;
use super::init_report::InitReport;
use super::overwrite_policy::OverwritePolicy;
use super::project_identity::ProjectIdentity;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const TEMPLATE_FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/template_files.rs"));

/// Validated destinations and file actions for an embedded template.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CopyPlan {
    source: PathBuf,
    destination: PathBuf,
    directories: Vec<PathBuf>,
    files: Vec<FilePlan>,
    skipped_files: usize,
    overwrite: OverwritePolicy,
}

impl CopyPlan {
    pub(super) fn build(
        source: &Path,
        destination: &Path,
        overwrite: OverwritePolicy,
    ) -> Result<Self> {
        // The binary carries a reviewed template; the compile-time root is only an overlap boundary.
        let source = normalized_destination(source)?;
        let destination = normalized_destination(destination)?;
        if destination.starts_with(&source) || source.starts_with(&destination) {
            bail!("template source and destination must not overlap");
        }
        let source = source.as_path();
        let destination = destination.as_path();
        if destination.exists() && !destination.is_dir() {
            bail!(
                "Destination path exists but is not a directory: {}",
                destination.display()
            );
        }

        let mut directories = Vec::new();
        let mut files = Vec::new();
        let mut conflicts = Vec::new();
        let mut skipped_files = 0;
        directories.push(destination.to_path_buf());
        for &(relative, bytes) in TEMPLATE_FILES {
            let relative_path = Path::new(relative);
            let target = destination.join(relative);
            if let Some(parent) = target.parent() {
                directories.push(parent.to_path_buf());
            }
            if should_preserve_existing_file(relative_path) && target.exists() {
                skipped_files += 1;
                continue;
            }
            if fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                bail!("refusing to overwrite a destination symlink");
            }
            let action = if target.exists() {
                if overwrite == OverwritePolicy::ReplaceGenerated && target.is_file() {
                    FileAction::Overwrite
                } else {
                    conflicts.push(relative);
                    continue;
                }
            } else {
                FileAction::Copy
            };
            files.push(FilePlan {
                relative,
                bytes,
                destination: target,
                action,
            });
        }
        directories.sort();
        directories.dedup();
        for directory in &directories {
            for ancestor in directory
                .ancestors()
                .take_while(|ancestor| ancestor.starts_with(destination))
            {
                if fs::symlink_metadata(ancestor)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    bail!("refusing a destination containing directory symlinks");
                }
            }
        }
        if !conflicts.is_empty() {
            let shown_conflicts = conflicts
                .iter()
                .take(10)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            let remaining = conflicts.len().saturating_sub(10);
            let suffix = if remaining == 0 {
                String::new()
            } else {
                format!(" and {remaining} more")
            };
            return Err(eyre::eyre!(
                "Destination already contains generated files that would be overwritten: {shown_conflicts}{suffix}. Re-run with --force to overwrite generated files."
            ).wrap_err(crate::safe_failure::SafeFailure::GeneratedFileConflict));
        }

        Ok(Self {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            directories,
            files,
            skipped_files,
            overwrite,
        })
    }

    pub(super) fn preview(&self, identity: &ProjectIdentity) -> InitReport {
        self.report(
            identity,
            true,
            self.directories
                .iter()
                .filter(|path| !path.exists())
                .count(),
        )
    }

    pub(super) fn execute(self, identity: &ProjectIdentity) -> Result<InitReport> {
        // Recheck conflicts and path resolution before any writes. This does not lock
        // the filesystem: concurrent changes during execution remain possible.
        let current = Self::build(&self.source, &self.destination, self.overwrite)?;
        eyre::ensure!(
            current == self,
            "template destinations changed; build a new copy plan"
        );
        let mut created_directories = 0;
        for directory in &self.directories {
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

        for file in &self.files {
            file.write(identity)?;
        }

        Ok(self.report(identity, false, created_directories))
    }

    fn report(
        &self,
        identity: &ProjectIdentity,
        dry_run: bool,
        created_directories: usize,
    ) -> InitReport {
        InitReport {
            source: self.source.display().to_string(),
            destination: self.destination.display().to_string(),
            project_name: identity.name().to_owned(),
            dry_run,
            copied_files: self
                .files
                .iter()
                .filter(|file| matches!(file.action, FileAction::Copy))
                .count(),
            overwritten_files: self
                .files
                .iter()
                .filter(|file| matches!(file.action, FileAction::Overwrite))
                .count(),
            skipped_files: self.skipped_files,
            created_directories,
            files: self
                .files
                .iter()
                .map(|file| file.relative.to_owned())
                .collect(),
        }
    }
}

fn normalized_destination(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                eyre::ensure!(normalized.pop(), "destination leaves filesystem root");
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    let mut ancestor = normalized.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| eyre::eyre!("destination has no existing ancestor"))?;
    }
    Ok(fs::canonicalize(ancestor)?.join(normalized.strip_prefix(ancestor)?))
}

fn should_preserve_existing_file(relative: &Path) -> bool {
    relative == Path::new("README.md") || relative == Path::new("LICENSE")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_template_does_not_require_the_original_source_directory() {
        let root = std::env::temp_dir().join(format!(
            "teamy-bundle-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("absent-source");
        let destination = root.join("bundle-tool");
        let identity = ProjectIdentity::validate("bundle-tool", None).unwrap();
        let plan = CopyPlan::build(&source, &destination, OverwritePolicy::RejectExisting).unwrap();
        let preview = plan.preview(&identity);
        assert!(!destination.exists());
        assert!(preview.dry_run);
        let report = plan.execute(&identity).unwrap();
        assert!(!report.dry_run);
        assert_eq!(preview.files, report.files);
        assert_eq!(preview.copied_files, report.copied_files);
        assert_eq!(preview.overwritten_files, report.overwritten_files);
        assert_eq!(preview.created_directories, report.created_directories);
        assert!(destination.join("src/cli/cache/mod.rs").is_file());
        assert!(!source.exists());
        assert!(
            std::fs::read_to_string(destination.join("src/main.rs"))
                .unwrap()
                .contains("bundle_tool::entrypoint()")
        );
        assert!(
            !std::fs::read_to_string(destination.join("src/lib.rs"))
                .unwrap()
                .contains("TeamDman/teamy-rust-cli")
        );
        assert!(
            !std::fs::read_to_string(destination.join("src/cli/mod.rs"))
                .unwrap()
                .contains("TeamDman/teamy-rust-cli")
        );
        let manifest = destination.join("Cargo.toml");
        std::fs::write(&manifest, "existing manifest").unwrap();
        let error =
            CopyPlan::build(&source, &destination, OverwritePolicy::RejectExisting).unwrap_err();
        assert!(error.to_string().contains("Re-run with --force"));
        assert_eq!(
            std::fs::read_to_string(&manifest).unwrap(),
            "existing manifest"
        );
        let plan =
            CopyPlan::build(&source, &destination, OverwritePolicy::ReplaceGenerated).unwrap();
        let preview = plan.preview(&identity);
        let report = plan.execute(&identity).unwrap();
        assert_eq!(preview.overwritten_files, report.overwritten_files);
        assert!(report.overwritten_files > 0);
        assert_eq!(report.copied_files, 0);
        assert_eq!(report.skipped_files, 2);

        let changed_destination = root.join("changed-tool");
        let stale = CopyPlan::build(
            &source,
            &changed_destination,
            OverwritePolicy::ReplaceGenerated,
        )
        .unwrap();
        std::fs::create_dir_all(&changed_destination).unwrap();
        let changed_manifest = changed_destination.join("Cargo.toml");
        std::fs::write(&changed_manifest, "created after planning").unwrap();
        let error = stale.execute(&identity).unwrap_err();
        assert!(error.to_string().contains("destinations changed"));
        assert_eq!(
            std::fs::read_to_string(changed_manifest).unwrap(),
            "created after planning"
        );
        assert!(!changed_destination.join("src").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
