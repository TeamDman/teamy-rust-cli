use super::CliOutput;
use super::OutputFormat;
use facet::Facet;
use std::cell::Cell;
use std::fmt;
use std::io;
use std::io::Write;
use std::rc::Rc;
use teamy_cancellation::CancellationError;
use teamy_cancellation::CancellationToken;

#[derive(Default)]
struct CapturingWriter {
    bytes: Vec<u8>,
    write_calls: usize,
    flushes: usize,
    cancel_on_write: Option<CancellationToken>,
}

impl Write for CapturingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_calls += 1;
        self.bytes.extend_from_slice(buf);
        if let Some(token) = &self.cancel_on_write {
            token.request_cancel("cancelled during output");
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
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
fn json_stops_during_traversal_and_preserves_the_cancellation_reason() -> eyre::Result<()> {
    let token = CancellationToken::new();
    let conversions = Rc::new(Cell::new(0));
    let probe = SerializationProbe {
        token: token.clone(),
        conversions: Rc::clone(&conversions),
    };
    token.with_scope(|scope| {
        let mut writer = CapturingWriter::default();

        let error = CliOutput::facet(vec![probe; 32])
            .write_to(OutputFormat::Json, false, &mut writer, &scope)
            .unwrap_err();

        let cancellation = error.downcast_ref::<CancellationError>().unwrap();
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

#[derive(Facet)]
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
            .write_to(format, false, &mut writer, &scope)
            .unwrap_err();

            let cancellation = error.downcast_ref::<CancellationError>().unwrap();
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

struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, OriginalFailure))
    }

    fn flush(&mut self) -> io::Result<()> {
        panic!("a failed output must not be flushed")
    }
}

#[test]
fn output_errors_keep_the_original_writer_kind_and_payload() -> eyre::Result<()> {
    CancellationToken::new().with_scope(|scope| {
        let error = CliOutput::facet([1, 2, 3])
            .write_to(OutputFormat::Json, false, &mut BrokenWriter, &scope)
            .unwrap_err();

        let original = error.downcast_ref::<io::Error>().unwrap();
        assert_eq!(original.kind(), io::ErrorKind::BrokenPipe);
        assert!(original.get_ref().unwrap().is::<OriginalFailure>());
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
            .write_to(format, false, &mut writer, &scope)?;

            assert!(writer.bytes.ends_with(b"\n"));
            assert!(!writer.bytes.ends_with(b"\n\n"));
            assert_eq!(writer.flushes, 1);
            Ok::<_, eyre::Report>(())
        })?;
    }
    Ok(())
}
