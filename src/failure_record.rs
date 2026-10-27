use facet::Facet;

/// Emergency diagnostic envelope for reports emitted outside the tracing subscriber.
#[derive(Debug, Facet)]
pub struct FailureRecord<'a> {
    pub level: &'a str,
    pub event_kind: &'a str,
    pub message: &'a str,
}
