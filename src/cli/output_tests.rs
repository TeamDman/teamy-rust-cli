use super::CliOutput;
use super::OutputFormat;
use facet::Facet;
use std::cell::Cell;
use std::fmt;
use std::io;
use std::io::Write;
use std::rc::Rc;
use teamy_cancellation::CancellationErrorExt;
use teamy_cancellation::CancellationToken;

#[derive(Default)]
struct CapturingWriter {
    bytes: Vec<u8>,
    write_calls: usize,
    flushes: usize,
    cancel_on_write: Option<CancellationToken>,
    fail_after: Option<usize>,
    fail_flush: bool,
}

impl Write for CapturingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_calls += 1;
        let remaining = self
            .fail_after
            .map_or(buf.len(), |limit| limit.saturating_sub(self.bytes.len()));
        let count = buf.len().min(remaining);
        if count == 0 && !buf.is_empty() {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, OriginalFailure));
        }
        self.bytes.extend_from_slice(&buf[..count]);
        if let Some(token) = &self.cancel_on_write {
            token.request_cancel("cancelled during output");
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            return Err(io::Error::other(OriginalFailure));
        }
        Ok(())
    }
}

#[derive(Clone, Facet)]
#[facet(opaque, proxy = SerializationProbeProxy)]
struct SerializationProbe {
    token: CancellationToken,
    conversions: Rc<Cell<usize>>,
}

#[derive(Facet)]
#[facet(transparent)]
struct SerializationProbeProxy(usize);

impl From<&SerializationProbe> for SerializationProbeProxy {
    fn from(probe: &SerializationProbe) -> Self {
        let count = probe.conversions.get() + 1;
        probe.conversions.set(count);
        if count == 3 {
            probe.token.request_cancel("cancelled while preparing JSON");
        }
        Self(count)
    }
}

impl TryFrom<SerializationProbeProxy> for SerializationProbe {
    type Error = &'static str;

    fn try_from(_: SerializationProbeProxy) -> Result<Self, Self::Error> {
        Err("serialization-only cancellation probe")
    }
}

#[test]
fn json_observes_proxy_requested_cancellation_at_the_next_write() -> eyre::Result<()> {
    let token = CancellationToken::new();
    let conversions = Rc::new(Cell::new(0));
    let probe = SerializationProbe {
        token: token.clone(),
        conversions: Rc::clone(&conversions),
    };
    token.with_scope(|scope| {
        let mut writer = CapturingWriter::default();

        let error = CliOutput::facet(vec![probe; 32])
            .emit_to(&mut writer, Some(OutputFormat::Json), false, &scope)
            .unwrap_err();

        let cancellation = error.as_cancellation().unwrap();
        assert_eq!(cancellation.reason, "cancelled while preparing JSON");
        assert_eq!(conversions.get(), 3);
        assert!(
            !writer.bytes.is_empty(),
            "the completed prefix was streamed"
        );
        assert_eq!(writer.flushes, 0);
        Ok(())
    })
}

#[derive(Debug, Facet, PartialEq, Eq)]
struct Record {
    id: u32,
    text: String,
}

#[test]
fn each_output_format_stops_writing_and_keeps_typed_cancellation() -> eyre::Result<()> {
    for format in [OutputFormat::Text, OutputFormat::Json, OutputFormat::Csv] {
        let token = CancellationToken::new();
        let mut writer = CapturingWriter {
            cancel_on_write: Some(token.clone()),
            ..CapturingWriter::default()
        };
        token.with_scope(|scope| {
            let error = CliOutput::facet(Record {
                id: 1,
                text: "a large field".repeat(20_000),
            })
            .emit_to(&mut writer, Some(format), false, &scope)
            .unwrap_err();

            let cancellation = error.as_cancellation().unwrap();
            assert_eq!(cancellation.reason, "cancelled during output");
            assert_eq!(writer.write_calls, 1);
            assert!(!writer.bytes.is_empty());
            assert_eq!(writer.flushes, 0);
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}

#[derive(Debug)]
struct OriginalFailure;

impl fmt::Display for OriginalFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the original output failure")
    }
}

impl std::error::Error for OriginalFailure {}

#[test]
fn output_errors_keep_the_original_writer_kind_and_payload() -> eyre::Result<()> {
    CancellationToken::new().with_scope(|scope| {
        let mut writer = CapturingWriter {
            fail_after: Some(0),
            ..CapturingWriter::default()
        };
        let error = CliOutput::facet([1, 2, 3])
            .emit_to(&mut writer, Some(OutputFormat::Json), false, &scope)
            .unwrap_err();

        let original = error.downcast_ref::<io::Error>().unwrap();
        assert_eq!(original.kind(), io::ErrorKind::BrokenPipe);
        assert!(original.get_ref().unwrap().is::<OriginalFailure>());
        assert!(writer.bytes.is_empty());
        assert_eq!(writer.flushes, 0);
        Ok(())
    })
}

#[test]
fn successful_output_has_one_final_line_ending_and_one_flush() -> eyre::Result<()> {
    for format in [OutputFormat::Text, OutputFormat::Json, OutputFormat::Csv] {
        CancellationToken::new().with_scope(|scope| {
            let mut writer = CapturingWriter::default();

            CliOutput::facet(Record {
                id: 1,
                text: String::from("small record"),
            })
            .emit_to(&mut writer, Some(format), false, &scope)?;

            assert!(writer.bytes.ends_with(b"\n"));
            assert!(!writer.bytes.ends_with(b"\n\n"));
            assert_eq!(writer.flushes, 1);
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}

#[test]
fn injected_writer_uses_automatic_json_and_explicit_format_overrides() -> eyre::Result<()> {
    for (format, terminal) in [(None, false), (Some(OutputFormat::Json), true)] {
        CancellationToken::new().with_scope(|scope| {
            let mut writer = CapturingWriter::default();
            let supplied: &mut dyn Write = &mut writer;
            CliOutput::facet(Record {
                id: 7,
                text: "captured output".into(),
            })
            .emit_to(supplied, format, terminal, &scope)?;

            assert!(!writer.bytes.contains(&0x1b));
            let text = String::from_utf8(writer.bytes).unwrap();
            let record: Record = facet_json::from_str(&text).unwrap();
            assert_eq!(record.id, 7);
            assert_eq!(record.text, "captured output");
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}

#[test]
fn text_colors_follow_the_supplied_terminal_status() -> eyre::Result<()> {
    for (format, terminal) in [(Some(OutputFormat::Text), false), (None, true)] {
        CancellationToken::new().with_scope(|scope| {
            let mut writer = CapturingWriter::default();
            CliOutput::facet(Record {
                id: 7,
                text: "captured output".into(),
            })
            .emit_to(&mut writer, format, terminal, &scope)?;

            assert_eq!(writer.bytes.contains(&0x1b), terminal);
            assert!(
                String::from_utf8(writer.bytes)
                    .unwrap()
                    .contains("captured output")
            );
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}

#[test]
fn unsupported_csv_output_neither_writes_nor_flushes() -> eyre::Result<()> {
    CancellationToken::new().with_scope(|scope| {
        let mut writer = CapturingWriter::default();
        let error = CliOutput::facet(vec![Record {
            id: 1,
            text: "row".into(),
        }])
        .emit_to(&mut writer, Some(OutputFormat::Csv), false, &scope)
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("failed to serialize command output as CSV")
        );
        assert_eq!(writer.write_calls, 0);
        assert_eq!(writer.flushes, 0);
        Ok(())
    })
}

#[test]
fn partial_payload_and_newline_failures_keep_the_prefix_without_flushing() -> eyre::Result<()> {
    for (limit, prefix, context) in [
        (
            1,
            b"4".as_slice(),
            "failed to serialize command output as JSON",
        ),
        (2, b"42".as_slice(), "failed to terminate command output"),
    ] {
        CancellationToken::new().with_scope(|scope| {
            let mut writer = CapturingWriter {
                fail_after: Some(limit),
                ..CapturingWriter::default()
            };
            let error = CliOutput::facet(42_u32)
                .emit_to(&mut writer, Some(OutputFormat::Json), false, &scope)
                .unwrap_err();
            assert!(error.to_string().contains(context));
            assert_eq!(writer.bytes, prefix);
            assert_eq!(writer.flushes, 0);
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}

#[test]
fn flush_failure_reaches_the_caller_after_complete_output() -> eyre::Result<()> {
    CancellationToken::new().with_scope(|scope| {
        let mut writer = CapturingWriter {
            fail_flush: true,
            ..CapturingWriter::default()
        };
        let error = CliOutput::facet(42_u32)
            .emit_to(&mut writer, Some(OutputFormat::Json), false, &scope)
            .unwrap_err();
        assert!(error.to_string().contains("failed to flush command output"));
        assert!(
            error
                .downcast_ref::<io::Error>()
                .unwrap()
                .get_ref()
                .unwrap()
                .is::<OriginalFailure>()
        );
        assert_eq!(writer.bytes, b"42\n");
        assert_eq!(writer.flushes, 1);
        Ok(())
    })
}

#[test]
fn absent_output_does_not_touch_the_writer() -> eyre::Result<()> {
    CancellationToken::new().with_scope(|scope| {
        let mut writer = CapturingWriter {
            fail_after: Some(0),
            fail_flush: true,
            ..CapturingWriter::default()
        };
        CliOutput::none().emit_to(&mut writer, Some(OutputFormat::Csv), false, &scope)?;
        assert_eq!(writer.write_calls, 0);
        assert_eq!(writer.flushes, 0);
        Ok(())
    })
}
