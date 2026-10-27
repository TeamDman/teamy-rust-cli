use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use facet_pretty::ColorMode;
use facet_pretty::PrettyPrinter;
use std::io::Write;

#[derive(Arbitrary, Facet, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Csv,
}

pub struct CliOutput {
    value: Option<Box<dyn CliOutputValue>>,
    /// Completion-time budget, including output. None explicitly exempts interactive or unbounded work.
    pub elapsed_duration_warn_threshold: Option<std::time::Duration>,
}

impl core::fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CliOutput")
            .field("has_value", &self.value.is_some())
            .field(
                "elapsed_duration_warn_threshold",
                &self.elapsed_duration_warn_threshold,
            )
            .finish()
    }
}

trait CliOutputValue {
    fn render(&self, format: OutputFormat, output_is_terminal: bool) -> eyre::Result<String>;
}

struct FacetCliOutput<T> {
    value: T,
}

impl CliOutput {
    #[must_use]
    pub const fn none(elapsed_duration_warn_threshold: Option<std::time::Duration>) -> Self {
        Self {
            value: None,
            elapsed_duration_warn_threshold,
        }
    }

    #[must_use]
    pub fn facet<T>(value: T, elapsed_duration_warn_threshold: Option<std::time::Duration>) -> Self
    where
        T: Facet<'static> + 'static,
    {
        Self {
            value: Some(Box::new(FacetCliOutput { value })),
            elapsed_duration_warn_threshold,
        }
    }

    /// Render into the caller's writer, using its explicit terminal status for automatic
    /// format selection and text colors. This does not select an operating-system stream.
    ///
    /// # Errors
    ///
    /// This function will return an error if rendering, writing or flushing fails.
    pub fn emit_to<W: Write + ?Sized>(
        self,
        writer: &mut W,
        requested_format: Option<OutputFormat>,
        output_is_terminal: bool,
    ) -> eyre::Result<()> {
        let Some(output) = self.value else {
            return Ok(());
        };

        let format = requested_format.unwrap_or(if output_is_terminal {
            OutputFormat::Text
        } else {
            OutputFormat::Json
        });
        let rendered = output.render(format, output_is_terminal)?;

        writer
            .write_all(rendered.as_bytes())
            .wrap_err("failed to write command output")?;
        if !rendered.ends_with('\n') {
            writer
                .write_all(b"\n")
                .wrap_err("failed to terminate command output")?;
        }
        writer.flush().wrap_err("failed to flush command output")?;
        Ok(())
    }
}

impl<T> CliOutputValue for FacetCliOutput<T>
where
    T: Facet<'static> + 'static,
{
    fn render(&self, format: OutputFormat, output_is_terminal: bool) -> eyre::Result<String> {
        let rendered = match format {
            OutputFormat::Text => PrettyPrinter::new()
                .with_colors(if output_is_terminal {
                    ColorMode::Always
                } else {
                    ColorMode::Never
                })
                .format(&self.value),
            OutputFormat::Json => facet_json::to_string_pretty(&self.value)
                .wrap_err("failed to serialize command output as JSON")?,
            OutputFormat::Csv => facet_csv::to_string(&self.value)
                .wrap_err("failed to serialize command output as CSV")?,
        };
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[derive(Debug, Facet, PartialEq, Eq)]
    struct Report {
        message: String,
        count: u32,
    }

    #[test]
    fn budget_metadata_never_changes_serialized_result() {
        let mut reference = Vec::new();
        CliOutput::facet(report(), Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut reference, Some(OutputFormat::Json), false)
            .unwrap();
        for threshold in [None, Some(std::time::Duration::from_secs(30))] {
            let mut bytes = Vec::new();
            CliOutput::facet(report(), threshold)
                .emit_to(&mut bytes, Some(OutputFormat::Json), false)
                .unwrap();
            assert_eq!(bytes, reference);
        }
    }

    fn report() -> Report {
        Report {
            message: "Captured output".to_owned(),
            count: 3,
        }
    }

    #[derive(Debug)]
    struct FixedOutput(&'static str);

    impl CliOutputValue for FixedOutput {
        fn render(&self, _: OutputFormat, _: bool) -> eyre::Result<String> {
            Ok(self.0.to_owned())
        }
    }

    fn fixed_output(text: &'static str) -> CliOutput {
        CliOutput {
            value: Some(Box::new(FixedOutput(text))),
            elapsed_duration_warn_threshold: Some(std::time::Duration::from_secs(1)),
        }
    }

    #[derive(Debug, Default)]
    struct FaultWriter {
        bytes: Vec<u8>,
        fail_after: Option<usize>,
        fail_flush: bool,
        flushes: usize,
    }

    impl Write for FaultWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let remaining = self
                .fail_after
                .map_or(buf.len(), |limit| limit.saturating_sub(self.bytes.len()));
            let count = buf.len().min(remaining);
            if count == 0 && !buf.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "fixture write failure",
                ));
            }
            self.bytes.extend_from_slice(&buf[..count]);
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            if self.fail_flush {
                return Err(io::Error::other("fixture flush failure"));
            }
            Ok(())
        }
    }

    #[test]
    fn injected_vec_captures_typed_json_for_pipe_default_and_explicit_format() {
        for (format, terminal) in [(None, false), (Some(OutputFormat::Json), true)] {
            let mut bytes = Vec::new();
            CliOutput::facet(report(), Some(std::time::Duration::from_secs(1)))
                .emit_to(&mut bytes, format, terminal)
                .expect("captured JSON output");
            assert_eq!(bytes.last(), Some(&b'\n'));
            assert!(!bytes.contains(&0x1b));
            let text = String::from_utf8(bytes).expect("UTF-8 output");
            let decoded: Report = facet_json::from_str(&text).expect("typed JSON output");
            assert_eq!(decoded, report());
        }
    }

    #[test]
    fn text_output_uses_supplied_terminal_status_for_colors() {
        let mut pipe = Vec::new();
        CliOutput::facet(report(), Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut pipe, Some(OutputFormat::Text), false)
            .expect("plain text capture");
        assert!(!pipe.contains(&0x1b));
        assert!(String::from_utf8(pipe).unwrap().contains("Captured output"));

        let mut terminal = Vec::new();
        CliOutput::facet(report(), Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut terminal, None, true)
            .expect("terminal text capture");
        assert!(terminal.contains(&0x1b));
    }

    #[test]
    fn injected_vec_captures_csv_row_without_terminal_decoration() {
        let mut bytes = Vec::new();
        CliOutput::facet(report(), Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut bytes, Some(OutputFormat::Csv), true)
            .expect("captured CSV output");
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert!(!bytes.contains(&0x1b));
        let text = String::from_utf8(bytes).expect("UTF-8 output");
        let decoded: Report =
            facet_csv::from_str(text.trim_end_matches('\n')).expect("typed CSV row");
        assert_eq!(decoded, report());
    }

    #[test]
    fn csv_sequence_serialization_failure_neither_writes_nor_flushes() {
        let mut writer = FaultWriter::default();
        let error = CliOutput::facet(vec![report()], Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut writer, Some(OutputFormat::Csv), false)
            .expect_err("the pinned CSV serializer does not support sequences");
        assert!(
            error
                .to_string()
                .contains("failed to serialize command output as CSV")
        );
        assert_eq!(error.root_cause().to_string(), "format serializer error");
        assert!(writer.bytes.is_empty());
        assert_eq!(writer.flushes, 0);
    }

    #[test]
    fn existing_or_missing_newline_is_terminated_once_then_flushed() {
        for text in ["payload", "payload\n"] {
            let mut writer = FaultWriter::default();
            fixed_output(text)
                .emit_to(&mut writer, Some(OutputFormat::Text), false)
                .expect("captured payload");
            assert_eq!(writer.bytes, b"payload\n");
            assert_eq!(writer.flushes, 1);
        }
    }

    #[test]
    fn partial_write_failure_reaches_the_caller_without_flushing() {
        let mut writer = FaultWriter {
            fail_after: Some(2),
            ..Default::default()
        };
        let error = fixed_output("payload")
            .emit_to(&mut writer, Some(OutputFormat::Text), false)
            .expect_err("partial payload write must fail");
        assert!(error.to_string().contains("failed to write command output"));
        assert!(format!("{error:#}").contains("fixture write failure"));
        assert_eq!(writer.bytes, b"pa");
        assert_eq!(writer.flushes, 0);
    }

    #[test]
    fn newline_write_failure_reaches_the_caller_without_flushing() {
        let mut writer = FaultWriter {
            fail_after: Some(7),
            ..Default::default()
        };
        let error = fixed_output("payload")
            .emit_to(&mut writer, Some(OutputFormat::Text), false)
            .expect_err("newline write must fail");
        assert!(
            error
                .to_string()
                .contains("failed to terminate command output")
        );
        assert_eq!(writer.bytes, b"payload");
        assert_eq!(writer.flushes, 0);
    }

    #[test]
    fn flush_failure_reaches_the_caller_after_complete_output() {
        let mut writer = FaultWriter {
            fail_flush: true,
            ..Default::default()
        };
        let error = fixed_output("payload")
            .emit_to(&mut writer, Some(OutputFormat::Text), false)
            .expect_err("flush must fail");
        assert!(error.to_string().contains("failed to flush command output"));
        assert!(format!("{error:#}").contains("fixture flush failure"));
        assert_eq!(writer.bytes, b"payload\n");
        assert_eq!(writer.flushes, 1);
    }

    #[test]
    fn absent_output_neither_writes_nor_flushes() {
        let mut writer = FaultWriter {
            fail_after: Some(0),
            fail_flush: true,
            ..Default::default()
        };
        CliOutput::none(Some(std::time::Duration::from_secs(1)))
            .emit_to(&mut writer, Some(OutputFormat::Csv), false)
            .expect("no output does not use writer");
        assert!(writer.bytes.is_empty());
        assert_eq!(writer.flushes, 0);
    }
}
