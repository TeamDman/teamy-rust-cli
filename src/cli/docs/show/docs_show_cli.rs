use crate::cli::docs::generate_help_snapshots;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;

/// Print generated command help docs to stdout.
#[derive(Facet, Arbitrary, Debug, PartialEq, Default)]
pub struct DocsShowArgs;

impl DocsShowArgs {
    /// # Errors
    ///
    /// This function will return an error if help output cannot be generated.
    #[expect(clippy::unused_async)]
    pub async fn invoke(self) -> Result<()> {
        let snapshots = generate_help_snapshots()?;

        for (index, snapshot) in snapshots.iter().enumerate() {
            if index > 0 {
                println!();
            }
            println!("$ {}", snapshot.invocation);
            println!();
            print!("{}", snapshot.output);
            if !snapshot.output.ends_with('\n') {
                println!();
            }
        }

        Ok(())
    }
}
