use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

/// Delete the cache files.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheCleanArgs;

impl CacheCleanArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(
        clippy::unused_async,
        reason = "command invoke methods share the async CLI dispatch shape"
    )]
    pub async fn invoke(self) -> Result<CliOutput> {
        Ok(CliOutput::none())
    }
}
