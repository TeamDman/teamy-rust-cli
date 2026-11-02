# Optional inline terminal foundation

The `inline-terminal` feature adds small, caller-owned components; it does not
change the default CLI, install a subscriber, or open an input listener.
`InlineTerminal` enters raw mode and owns a Ratatui inline viewport. Check that
the streams are interactive and the application owns terminal input and output
before opening it. A process-local guard rejects a second or reentrant owner;
it cannot coordinate another process or code that bypasses this wrapper. Call
`close` and handle restoration errors; `Drop` makes a best-effort fallback. A
screen restoration failure leaves the guard unavailable for the process rather
than falsely claiming the screen is safe. This is not a nested-owner handoff
coordinator.

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

## Windows state adapter

The separate `windows-foundation` feature provides a `cfg(windows)`
`DesktopSession` with an injected `DesktopBackend`. The caller explicitly
creates the tray, routes actions on its own message loop, and calls `shutdown`
to restore an owned hidden console and remove the tray. The adapter refuses
show/hide for shared or unavailable contexts and changes recorded state only
after a backend succeeds. It contains no native backend, message loop, input
listener, or automatic activation. The caller must establish console ownership
from actual launch context; a handle alone is insufficient evidence. An
application such as Defibrillator can adapt its existing state and backend to
this interface after reviewing its separate ownership and input lifecycle.

## Planned Windows seams

Future opt-in native tray and console backends can use a dedicated
message-loop owner with explicit shutdown and join. Model an application-owned
visible or hidden console separately from inherited, ConPTY, and unavailable
contexts. Only an explicitly owned console may be hidden or restored. In
particular, `GetConsoleWindow` under ConPTY is not authority to hide the user's
Windows Terminal. Piing's owned/inherited distinction and tb's tray message
handling are useful prior art, not implementations supplied by this feature.
Raw Input capture, global hotkeys, elevation and service installation require
separate optional APIs and review; nothing here activates them.

## Template provenance

The baseline workflow copies the template into a new project and removes
unneeded features. Existing projects may copy selected modules, record the
source commit and local deviations, and review later template updates; no new
shared crate or publication is required for that workflow. Do not add a
machine-specific path dependency to connect independent checkouts.

`inline_pty_probe` exercises normal and application-error restoration,
reentrant-owner rejection, bounded acknowledgement, and escaped synthetic
logs. Its `close-error` mode injects a screen-write failure and confirms that
raw mode is restored while later ownership is refused. Run each mode in a
synthetic PTY; other terminal hosts and external terminal writers remain outside
the probe's guarantees.
