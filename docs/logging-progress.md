# Logging and progress contract

Command results use stdout and the existing `--output-format` contract. Diagnostics use stderr and an optional explicitly named file.

| Setting | Default |
| --- | --- |
| `--log-format text\|ndjson` | text |
| `--log-filter DIRECTIVES` | explicit flag, then `--debug`, then `RUST_LOG`, then info |
| `--log-file FILE` | no file sink |
| `--log-file-format text\|ndjson` | ndjson |
| `--log-file-filter DIRECTIVES` | debug, independent of stderr and environment |

An explicit stderr filter wins over `--debug`; `--log-level` remains its existing alias. Filters accept full EnvFilter directive lists. Regex matching in field directives is disabled. File filters never inherit stderr filters. Both are attached to their own layers, so INFO stderr cannot suppress DEBUG file events. Tracy retains its separate collection policy.

Files append, create missing parent directories, and require a filename rather than a directory. A mutex guard covers each full formatted file record. JSON sinks and file text never emit ANSI. All filters are validated before file effects; subscriber installation failures are errors. Do not use explicit log files as a durable audit guarantee: there is no fsync/rotation policy.

The binary installs color-eyre before parsing. Runtime failures retain their error
chain and source location; panic reports retain the panic message and location.
Backtraces follow color-eyre's environment settings. Reports use a plain theme so
the same content remains ANSI-free in text, files and NDJSON. Text diagnostics
print the report directly; NDJSON encodes it as the message of one JSON record,
including escaped newlines. Before parsing succeeds, diagnostics use text. After
parsing, they use the typed `--log-format` selection, including when logging setup
fails or diagnostics are disabled. Startup does not scan arguments separately.
Explicit help/version/completions retain Figue's presentation.
Figue owns argument interpretation, validation and help precedence. There is no application pre-parser or argument rewriting. Unsuccessful parse outcomes become the existing safe invalid-input diagnostic; help retains Figue's presentation with argument-bearing suggestions removed.

Parse failures omit raw argv-bearing parser diagnostics. Runtime error and panic
reports preserve their supplied context; formatting does not redact that context.
Choose error context and structured tracing fields deliberately.
Emit direct tracing events at meaningful call sites. Choose DEBUG for routine dispatch and completion, INFO for useful operation updates, and WARN/ERROR for reviewed failures. Add operation-specific structured fields, such as completed_items, bytes_written, elapsed_ms, or phase, when useful. There is no mandatory progress schema, universal task/run/state vocabulary, or custom caller-location wrapper. Text and NDJSON retain tracing's actual event file/line. Existing command spans still provide timing receipts, and existing expected-duration checks remain warning-only. Logs do not replace command results or typed service status.

There is no GUI or OpenTelemetry dependency. A downstream application can interpret the fields directly.

The initializer carries its curated template bytes in the executable. It does not require the original checkout to remain available at runtime. Build-time inventory includes known source/test/resource extensions and selected root files; links, hidden local files, caches and logs are excluded.


Command runtime expectations describe a command contract (for example, expected
completion under one second where that contract already exists). They are not
work progress or an ETA. Actual work may emit item/byte counts, remaining work,
rates, or evidence-based item/byte ETAs as operation-specific fields. Consumers
may derive rates and ETAs from suitable measurements. Do not substitute command
completed=0/1, mandatory percentages, or invented estimates for those measurements.

```rust
tracing::debug!(items_processed = processed, bytes_written = written, "Batch written");
```

Use safe fields at the actual operation call site; no helper envelope is required.

Raw process arguments are not logged. Command lifecycle events contain the selected
command path derived from Facet and Figue's schema. Argument values are excluded.
Non-Unicode arguments are rejected before logging; help, version and completions
exit before log-file creation or command execution.