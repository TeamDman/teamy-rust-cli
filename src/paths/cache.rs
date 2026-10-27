use directories_next::ProjectDirs;
use eyre::bail;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;
use tracing::warn;

/// The cache home directory for API responses.
/// This MUST NOT be used within functions outside of a top-level resolution function; any function relying on
/// a [`CacheHome`] must take it as a parameter to ensure testing is straightforward.
pub static CACHE_DIR: LazyLock<CacheHome> = LazyLock::new(|| match CacheHome::resolve() {
    Ok(c) => c,
    Err(e) => {
        warn!("Failed to resolve cache home: {}", e);
        CacheHome(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
});

/// Helper that resolves the application cache directory.
#[derive(Clone, Debug)]
pub struct CacheHome(pub PathBuf);

impl CacheHome {
    /// Resolve the `CacheHome` according to:
    /// * If [`super::APP_CACHE_ENV_VAR`] env var is set, use that directory
    /// * Otherwise use the platform `ProjectDirs::cache_dir()`
    ///
    /// # Errors
    ///
    /// This function will return an error if the cache directory cannot be determined.
    pub fn resolve() -> eyre::Result<CacheHome> {
        if let Ok(override_dir) = std::env::var(super::APP_CACHE_ENV_VAR) {
            return Ok(CacheHome(PathBuf::from(override_dir)));
        }
        // TODO(template): replace the organization name below if the generated project uses a different owner.
        if let Some(project_dirs) = ProjectDirs::from("", "TeamDman", super::APP_CACHE_DIR_NAME) {
            Ok(CacheHome(project_dirs.cache_dir().to_path_buf()))
        } else {
            bail!("Could not determine cache directory")
        }
    }
}

impl std::ops::Deref for CacheHome {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.0.as_path()
    }
}
