#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OverwritePolicy {
    RejectExisting,
    ReplaceGenerated,
}
