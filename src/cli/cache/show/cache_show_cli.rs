use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

#[derive(Facet, Debug)]
struct CacheShowReport {
    path: String,
}

/// Show the cache path.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct CacheShowArgs;

impl CacheShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<CliOutput> {
        Ok(CliOutput::facet(CacheShowReport {
            path: crate::paths::CACHE_DIR.display().to_string(),
        }))
    }
}
