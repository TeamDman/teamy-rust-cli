use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

#[derive(Facet, Debug)]
struct HomeShowReport {
    path: String,
}

/// Show the home path.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeShowArgs;

impl HomeShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(
        clippy::unused_async,
        reason = "command invoke methods share the async CLI dispatch shape"
    )]
    pub async fn invoke(self) -> Result<CliOutput> {
        Ok(CliOutput::facet(HomeShowReport {
            path: crate::paths::APP_HOME.display().to_string(),
        }))
    }
}
