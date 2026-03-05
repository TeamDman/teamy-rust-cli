use crate::cli::ToArgs;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;

/// Open the cache path in the platform file manager.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheOpenArgs;

impl CacheOpenArgs {
    /// # Errors
    ///
    /// This function will return an error if the cache directory cannot be created
    /// or the file manager cannot be launched.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        std::fs::create_dir_all(crate::paths::CACHE_DIR.0.as_path())?;
        open::that_detached(crate::paths::CACHE_DIR.0.as_path()).wrap_err_with(|| {
            format!(
                "Failed to open {} in file manager",
                crate::paths::CACHE_DIR.0.display()
            )
        })?;
        Ok(())
    }
}

impl ToArgs for CacheOpenArgs {}