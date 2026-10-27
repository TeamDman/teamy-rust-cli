#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FileAction {
    Copy,
    Overwrite,
}
