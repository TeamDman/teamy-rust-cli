use crate::cancellation::CancellationToken;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tracing::Event;
use tracing::Id;
use tracing::Metadata;
use tracing::Subscriber;
use tracing::field::Field;
use tracing::field::Visit;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug)]
pub struct StopAfterLayer {
    spec: StopAfterSpec,
    cancellation_token: CancellationToken,
    triggered: AtomicBool,
}

impl StopAfterLayer {
    #[must_use]
    pub fn new(raw: impl Into<String>, cancellation_token: CancellationToken) -> Self {
        Self {
            spec: StopAfterSpec::parse(raw),
            cancellation_token,
            triggered: AtomicBool::new(false),
        }
    }
}

impl<S> Layer<S> for StopAfterLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if self.triggered.load(Ordering::Acquire) {
            return;
        }

        let Some(span) = ctx.span(&id) else {
            return;
        };

        if !self.spec.matches(span.metadata()) {
            return;
        }

        if self.triggered.swap(true, Ordering::AcqRel) {
            return;
        }

        self.cancellation_token.request_cancel(format!(
            "Operation cancelled after --stop-after matched tracing span `{}`",
            span.metadata().name()
        ));
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if self.triggered.load(Ordering::Acquire) {
            return;
        }

        let mut fields = EventFields::default();
        event.record(&mut fields);
        let matches = self.spec.matches(event.metadata())
            || fields
                .message
                .as_deref()
                .is_some_and(|message| self.spec.matches_message(message));
        if !matches {
            return;
        }

        if self.triggered.swap(true, Ordering::AcqRel) {
            return;
        }

        self.cancellation_token.request_cancel(format!(
            "Operation cancelled after --stop-after matched tracing event `{}`",
            fields
                .message
                .as_deref()
                .unwrap_or_else(|| event.metadata().name())
        ));
    }
}

#[derive(Default)]
struct EventFields {
    message: Option<String>,
}

impl Visit for EventFields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}").trim_matches('"').to_string());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

#[derive(Debug)]
struct StopAfterSpec {
    span_name: Option<String>,
    location: Option<StopAfterLocation>,
}

impl StopAfterSpec {
    fn parse(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let normalized = normalize_copied_value(&raw);
        let location = parse_location(&normalized).or_else(|| parse_location(&raw));
        let span_name = if location.is_some() {
            None
        } else {
            let name = strip_tracy_fields(&normalized).trim();
            (!name.is_empty()).then(|| name.to_string())
        };

        Self {
            span_name,
            location,
        }
    }

    fn matches(&self, metadata: &Metadata<'_>) -> bool {
        if self
            .span_name
            .as_deref()
            .is_some_and(|span_name| metadata.name() == span_name)
        {
            return true;
        }

        let Some(location) = &self.location else {
            return false;
        };
        let Some(file) = metadata.file() else {
            return false;
        };
        if metadata.line() != Some(location.line) {
            return false;
        }

        let metadata_file = normalize_path(file);
        metadata_file.ends_with(&location.file) || location.file.ends_with(&metadata_file)
    }

    fn matches_message(&self, message: &str) -> bool {
        self.span_name
            .as_deref()
            .is_some_and(|span_name| message == span_name)
    }
}

#[derive(Debug)]
struct StopAfterLocation {
    file: String,
    line: u32,
}

fn normalize_copied_value(raw: &str) -> String {
    raw.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn strip_tracy_fields(raw: &str) -> &str {
    raw.split_once('{').map_or(raw, |(name, _fields)| name)
}

fn parse_location(raw: &str) -> Option<StopAfterLocation> {
    let raw = normalize_copied_value(strip_tracy_fields(raw));
    let (file, line) = raw.rsplit_once(':')?;
    let line = line.parse::<u32>().ok()?;
    let file = normalize_path(file);
    (!file.is_empty()).then_some(StopAfterLocation { file, line })
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::StopAfterSpec;

    #[test]
    fn parses_copied_tracy_span_name_without_fields() {
        let spec = StopAfterSpec::parse("create_plan_for_target{drive=C refresh=false}");

        assert_eq!(spec.span_name.as_deref(), Some("create_plan_for_target"));
        assert!(spec.location.is_none());
    }

    #[test]
    fn parses_copied_location() {
        let spec = StopAfterSpec::parse(r"src\query\query_runtime.rs:42");

        let location = spec.location.expect("location should parse");
        assert_eq!(location.file, "src/query/query_runtime.rs");
        assert_eq!(location.line, 42);
        assert!(spec.span_name.is_none());
    }

    #[test]
    fn span_name_spec_matches_event_message() {
        let spec = StopAfterSpec::parse("Daemon query stream cancelled by client");

        assert!(spec.matches_message("Daemon query stream cancelled by client"));
        assert!(!spec.matches_message("other message"));
    }
}
