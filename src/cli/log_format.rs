use arbitrary::Arbitrary;
use facet::Facet;

/// Diagnostic encoding, independent of command result encoding.
#[derive(Arbitrary, Facet, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum LogFormat {
    #[default]
    Text,
    Ndjson,
}
