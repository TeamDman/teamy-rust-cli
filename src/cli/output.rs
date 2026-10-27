use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use facet_json::SerializeOptions;
use facet_pretty::ColorMode;
use facet_pretty::PrettyPrinter;
use std::io::Write;
use teamy_cancellation::CancellableWriter;
use teamy_cancellation::CancellationScope;

#[derive(Arbitrary, Facet, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Csv,
}

pub struct CliOutput(Option<Box<dyn CliOutputValue>>);

impl core::fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CliOutput")
            .field("has_value", &self.0.is_some())
            .finish()
    }
}

trait CliOutputValue {
    fn write_to(
        &self,
        format: OutputFormat,
        output_is_terminal: bool,
        writer: &mut dyn Write,
        cancellation: &CancellationScope,
    ) -> eyre::Result<()>;
}

struct FacetCliOutput<T> {
    value: T,
}

impl CliOutput {
    #[must_use]
    pub const fn none() -> Self {
        Self(None)
    }

    #[must_use]
    pub fn facet<T>(value: T) -> Self
    where
        T: Facet<'static> + 'static,
    {
        Self(Some(Box::new(FacetCliOutput { value })))
    }

    /// Render into the caller's writer, using its terminal status for automatic
    /// format selection and text colors.
    ///
    /// # Errors
    ///
    /// This function will return an error if serialization, writing, or flushing
    /// fails, or the supplied execution scope accepts cancellation.
    /// Already-written output is retained when cancellation or an error occurs.
    pub fn emit_to<W: Write + ?Sized>(
        self,
        writer: &mut W,
        requested_format: Option<OutputFormat>,
        output_is_terminal: bool,
        cancellation: &CancellationScope,
    ) -> eyre::Result<()> {
        let Some(output) = self.0 else {
            return Ok(());
        };
        let format = requested_format.unwrap_or(if output_is_terminal {
            OutputFormat::Text
        } else {
            OutputFormat::Json
        });
        let mut writer = CancellableWriter::new(writer, cancellation);
        output.write_to(format, output_is_terminal, &mut writer, cancellation)?;
        writer.flush().wrap_err("failed to flush command output")
    }
}

impl Default for CliOutput {
    fn default() -> Self {
        Self::none()
    }
}

impl<T> CliOutputValue for FacetCliOutput<T>
where
    T: Facet<'static> + 'static,
{
    fn write_to(
        &self,
        format: OutputFormat,
        output_is_terminal: bool,
        writer: &mut dyn Write,
        _cancellation: &CancellationScope,
    ) -> eyre::Result<()> {
        match format {
            OutputFormat::Text => {
                let rendered = std::fmt::from_fn(|formatter| {
                    PrettyPrinter::new()
                        .with_colors(if output_is_terminal {
                            ColorMode::Always
                        } else {
                            ColorMode::Never
                        })
                        .format_to(&self.value, formatter)
                });
                writeln!(writer, "{rendered}").wrap_err("failed to write command output")
            }
            OutputFormat::Json => {
                let options = SerializeOptions::new().pretty();
                facet_json::to_writer_std_with_options(&mut *writer, &self.value, &options)
                    .wrap_err("failed to serialize command output as JSON")?;
                writer
                    .write_all(b"\n")
                    .wrap_err("failed to terminate command output")
            }
            OutputFormat::Csv => {
                let rendered = facet_csv::to_string(&self.value)
                    .wrap_err("failed to serialize command output as CSV")?;
                writer
                    .write_all(rendered.as_bytes())
                    .wrap_err("failed to write command output")?;
                if !rendered.ends_with('\n') {
                    writer
                        .write_all(b"\n")
                        .wrap_err("failed to terminate command output")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
