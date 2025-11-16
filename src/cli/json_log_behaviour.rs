use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonLogBehaviour {
    None,
    Some(PathBuf),
    SomeAutomaticPath,
}

impl JsonLogBehaviour {
    /// Get the path where JSON logs should be written.
    /// Returns None if JSON logging is disabled.
    /// Returns Some with the path if a specific path was provided or if using automatic path generation.
    pub fn get_path(&self) -> Option<Cow<'_, PathBuf>> {
        match self {
            JsonLogBehaviour::None => None,
            JsonLogBehaviour::Some(path) => Some(Cow::Borrowed(path)),
            JsonLogBehaviour::SomeAutomaticPath => {
                Some(Cow::Owned(crate::tracing::default_json_log_path()))
            }
        }
    }
}

impl Default for JsonLogBehaviour {
    fn default() -> Self {
        JsonLogBehaviour::None
    }
}

impl FromStr for JsonLogBehaviour {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(JsonLogBehaviour::Some(PathBuf::from(s)))
    }
}

// For arbitrary/fuzzing support
impl arbitrary::Arbitrary<'_> for JsonLogBehaviour {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let choice: u8 = u.int_in_range(0..=2)?;
        Ok(match choice {
            0 => JsonLogBehaviour::None,
            1 => JsonLogBehaviour::Some(PathBuf::arbitrary(u)?),
            2 => JsonLogBehaviour::SomeAutomaticPath,
            _ => unreachable!(),
        })
    }
}
