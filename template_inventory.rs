use std::path::Path;
use std::path::PathBuf;

pub const TEMPLATE_DIRECTORIES: [&str; 5] =
    ["src", "tests", "resources", "docs", ".github/workflows"];

/// Curated template inputs. Neither compilation nor initialization reads local caches/secrets.
pub fn template_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "README.md",
        "LICENSE",
        "AGENTS.md",
        "build.rs",
        "template_inventory.rs",
        "check-all.ps1",
        "check-lint-policy.ps1",
        "clippy.toml",
        "rustfmt.toml",
        "run-profiler.ps1",
        "update.ps1",
        ".gitignore",
    ] {
        let path = root.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    for directory in TEMPLATE_DIRECTORIES {
        let path = root.join(directory);
        if path.is_dir() {
            collect(root, &path, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn collect(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let relative = path
                .strip_prefix(root)
                .expect("template traversal stays under root");
            if !entry.file_name().to_string_lossy().starts_with('.')
                && !["target", "logs"].contains(&entry.file_name().to_string_lossy().as_ref())
                && (entry.file_name() != "cache" || relative.starts_with("src"))
            {
                collect(root, &path, files)?;
            }
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("template traversal stays under root");
            let extension = path
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");
            let allowed = if relative.starts_with("src") || relative.starts_with("tests") {
                extension == "rs"
            } else if relative.starts_with("resources") {
                ["rc", "manifest", "ico", "png", "ps1"].contains(&extension)
            } else if relative.starts_with("docs") {
                ["logging-progress.md", "command-completion-budgets.md"]
                    .iter()
                    .any(|name| relative == Path::new("docs").join(name))
            } else {
                ["yml", "yaml"].contains(&extension)
            };
            if allowed && !entry.file_name().to_string_lossy().starts_with('.') {
                files.push(path);
            }
        }
    }
    Ok(())
}
