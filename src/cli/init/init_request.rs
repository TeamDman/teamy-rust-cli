use super::copy_plan::CopyPlan;
use super::init_mode::InitMode;
use super::init_report::InitReport;
use super::overwrite_policy::OverwritePolicy;
use super::project_identity::ProjectIdentity;
use std::path::Path;
use std::path::PathBuf;

/// Scaffold intent, independent of argument parsing and output rendering.
#[derive(Debug)]
pub(super) struct InitRequest {
    pub destination: PathBuf,
    pub identity: ProjectIdentity,
    pub overwrite: OverwritePolicy,
    pub mode: InitMode,
}

impl InitRequest {
    pub(super) fn run(self, template_root: &Path) -> eyre::Result<InitReport> {
        let plan = CopyPlan::build(template_root, &self.destination, self.overwrite)?;
        match self.mode {
            InitMode::Preview => Ok(plan.preview(&self.identity)),
            InitMode::Apply => plan.execute(&self.identity),
        }
    }
}
