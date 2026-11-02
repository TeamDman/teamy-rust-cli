# Optional inline terminal foundation

The `inline-terminal` feature adds small, caller-owned components; it does not
change the default CLI, install a subscriber, or open an input listener.
`InlineTerminal` enters raw mode and owns a Ratatui inline viewport. Check that
the streams are interactive and the application owns terminal input and output
before opening it. Call `close` and handle restoration errors; `Drop` makes a
best-effort fallback. This is not a coordinator for nested terminal owners.

`TerminalLogBuffer` is a `tracing_subscriber::fmt::MakeWriter` for a layer the
application constructs. Its defaults retain at most 512 records and 8,192
input bytes per record; configurable nonzero limits, eviction, truncation and
lock failure counts are exposed. A snapshot copies up to the requested number
of retained entries. Acknowledge only after presenting an entry successfully.
`replay` writes and flushes each entry before acknowledging it: a partial write
can appear again on retry. Replay writes stored text verbatim, so callers must
sanitize untrusted data before sending it to a terminal. `print_above` escapes
control and bidirectional characters and clips a single display line. Neither
component owns a file sink, a global subscriber, or a durable queue. Keep file
and console layers separate and never send an ingested OTLP event back to the
exporter that delivered it.

`ScreenStack` provides a caller-bounded navigation stack with a permanent
root; screen transitions do not transfer terminal ownership. Teamy
Defibrillator's recovery/binding state can remain application-specific while
using such navigation and logs after an explicit integration review.

## Planned Windows seams

Future opt-in, `cfg(windows)` tray and console features can use a dedicated
message-loop owner with explicit shutdown and join. Model an application-owned
visible or hidden console separately from inherited, ConPTY, and unavailable
contexts. Only an explicitly owned console may be hidden or restored. In
particular, `GetConsoleWindow` under ConPTY is not authority to hide the user's
Windows Terminal. Piing's owned/inherited distinction and tb's tray message
handling are useful prior art, not implementations supplied by this feature.
Raw Input capture, global hotkeys, elevation and service installation require
separate optional APIs and review; nothing here activates them.
