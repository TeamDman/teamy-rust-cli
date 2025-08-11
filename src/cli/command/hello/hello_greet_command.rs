use arbitrary::Arbitrary;
use clap::Args;
use std::ffi::OsString;
use crate::cli::to_args::ToArgs;

#[derive(Args, Clone, Arbitrary, PartialEq, Debug)]
pub struct HelloGreetArgs {
    pub name: String,
}

impl HelloGreetArgs {
    pub fn invoke(self) -> eyre::Result<()> {
        println!("Ahoy, {}", self.name);
        Ok(())
    }
}

impl ToArgs for HelloGreetArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![OsString::from(&self.name)]
    }
}
