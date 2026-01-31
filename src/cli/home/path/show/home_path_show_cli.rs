use eyre::Result;
use facet::Facet;

/// Show the home path.
#[derive(Facet, Debug)]
pub struct HomePathShowArgs;

impl HomePathShowArgs {
    /// # Errors
    ///
    /// This function does not return any errors.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        println!("{}", crate::paths::APP_HOME.display());
        Ok(())
    }
}
