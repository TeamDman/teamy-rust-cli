use super::file_action::FileAction;
use super::project_identity::ProjectIdentity;
use eyre::Context;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FilePlan {
    pub relative: &'static str,
    pub bytes: &'static [u8],
    pub destination: PathBuf,
    pub action: FileAction,
}

impl FilePlan {
    pub(super) fn write(&self, identity: &ProjectIdentity) -> eyre::Result<()> {
        if let Some(parent) = self.destination.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create directory {}", parent.display()))?;
        }
        let bytes = identity.render(Path::new(self.relative), self.bytes.to_vec());
        fs::write(&self.destination, bytes).wrap_err("failed to write generated file")
    }
}
