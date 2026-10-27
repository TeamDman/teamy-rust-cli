use facet::Facet;

#[derive(Facet, Debug)]
pub(crate) struct InitReport {
    pub source: String,
    pub destination: String,
    pub project_name: String,
    pub dry_run: bool,
    pub copied_files: usize,
    pub overwritten_files: usize,
    pub skipped_files: usize,
    pub created_directories: usize,
    pub files: Vec<String>,
}
