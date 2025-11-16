use std::ffi::OsString;

/// Trait for converting CLI structures to command line arguments
pub trait ToArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}

// Blanket implementation for references
impl<T: ToArgs> ToArgs for &T {
    fn to_args(&self) -> Vec<OsString> {
        (*self).to_args()
    }
}