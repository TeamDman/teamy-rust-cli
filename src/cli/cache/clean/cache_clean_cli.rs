use eyre::Result;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Debug)]
pub struct CacheCleanArgs;

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        Ok(())
    }
}
