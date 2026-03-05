use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use facet::Facet;

/// Open the home path in the platform file manager.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct HomeOpenArgs;

impl HomeOpenArgs {
    /// # Errors
    ///
    /// This function will return an error if the home directory cannot be created
    /// or the file manager cannot be launched.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        crate::paths::APP_HOME.ensure_dir()?;
        open::that_detached(crate::paths::APP_HOME.0.as_path()).wrap_err_with(|| {
            format!(
                "Failed to open {} in file manager",
                crate::paths::APP_HOME.0.display()
            )
        })?;
        Ok(())
    }
}
