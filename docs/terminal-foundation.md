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
`DesktopSession` and an injectable `DesktopBackend`. The optional
`NativeDesktopBackend` starts an invisible top-level window and its message
thread only when the caller explicitly activates the tray. A bounded queue
delivers `Show`, `Hide`, `Exit`, or a native error; the application interprets
these typed events and can inspect the queue's dropped-event counter. The tray
thread re-adds its icon after the `TaskbarCreated` broadcast. A private kernel
event requests shutdown; the explicit wait is bounded to two seconds and a
timeout leaves shutdown pending for retry. The backend attempts to restore an
owned hidden console and remove the icon on cleanup. Dropping is best effort;
call and check `shutdown`.

The adapter refuses show/hide for shared or unavailable contexts. The native
backend additionally requires an `OwnedConsole` token obtained only through a
successful explicit `AllocConsole` call after checking that the process has no
console. Allocation creates a window and changes that process's standard
handles; it is never automatic. Revalidation checks that the same window is
still attached before changing visibility or detaching. The application must
own and serialize its process-wide console lifecycle; unrelated code can still
change it behind this guard. `GetConsoleWindow` alone is not proof of ownership:
under ConPTY its handle is not the visible terminal window. See Microsoft's
[console documentation](https://learn.microsoft.com/en-us/windows/console/getconsolewindow)
and [taskbar creation notification](https://learn.microsoft.com/en-us/windows/win32/shell/taskbar#taskbar-creation-notification).

The native backend has only been compiled and tested through injected state and
shell-restart logic. It has not been activated or visually qualified. Local
Codex and Defibrillator tray implementations have copied or adapted the
foundation; their current console and worker still share a process.

## Console close and a separate UI process

The current `OwnedConsole` is allocated in the calling process. Hiding that
console does not guarantee that clicking its window Close button leaves the
tray owner alive. The next shared design places the unelevated tray and worker
in a background owner and its console UI in a separately spawned child. The
owner retains bounded history and continues its work when the child closes;
Show opens one replacement child and replays retained records with loss
indicators. In tray mode, `q` closes the UI and tray Exit stops the owner.
Ordinary attached-terminal commands keep their current exit behavior.

The private child channel uses bounded, versioned typed frames over anonymous
pipes, with one child generation at a time, restricted handle access, no shell
or arbitrary commands, and explicit EOF,
failure and shutdown handling. If stdin/stdout carry protocol frames, the
child must open its console input and output separately; it cannot assume the
redirected standard streams are a TTY. History stays in the owner rather than
being acknowledged away by a child. The existing `TerminalLogBuffer` removes
acknowledged records, so it needs a separate retained store or a deliberate
contract change for reopen replay. UI loss cancels any in-progress shortcut
capture and retains the previous binding. The shared contract and staged
synthetic, Windows process and later opt-in close-button tests are in the
adjacent Teamy Docs checkout's `src/terminal-ownership.md`, in “Keep the tray
owner alive when the console closes.” This split is
planned, not implemented; it does not promise survival of session shutdown or
external job termination.

The optional `ui-process` feature now supplies a tested process and transport
foundation, not yet a complete application split. `History` retains records
independently of acknowledgements, with caller-selected record and byte limits
and explicit eviction/rejection counts. Frames are limited to 64 KiB; the
supervisor has bounded delivery queues and an acknowledgement window, rejects
stale generations, and reports backpressure. Shutdown requests a clean close,
then terminates only the child it started if the supplied deadline expires.

On Windows the launcher creates anonymous pipes that are **never inheritable**
and starts the child with handle inheritance disabled. The child verifies the
parent process identity and creation time, then briefly opens that trusted
parent with duplicate-handle access and duplicates the two designated pipe
ends. This avoids both unrelated handles entering the child and a concurrent
sibling inheriting the UI pipes. The child executable must be trusted and
chosen by the application, not by a message from the UI. The bootstrap process
identity and numeric handle values are passed in process arguments; they are
not credentials. Other platforms currently use piped standard streams. In
both cases the terminal UI must open its console separately.

Synthetic hidden-child tests exercise framing, startup, reaping, crashes,
stale replies, rejected acknowledgements, backpressure, history across
reopening, and handle isolation with a positive control. They do not launch a
console UI or prove close-button behavior, console host compatibility, or job
object behavior. The actual tray and consumer integrations remain separate
qualification work.

## Planned Windows seams

Piing's owned/inherited distinction and tb's tray message handling are prior
art. Raw Input capture, global hotkeys, elevation and service installation
require separate optional APIs and review; nothing here activates them.

## Later manual tray qualification

After the graphics hold is lifted and a user explicitly opts in, build the
synthetic GUI-subsystem probe with
`cargo build --offline --locked --features windows-foundation --example windows_tray_probe`.
From PowerShell in this repository, start the shared-state probe:

```powershell
$trayProbe = Join-Path (Get-Location) 'target/debug/examples/windows_tray_probe.exe'
$sharedProbe = Start-Process -FilePath $trayProbe -ArgumentList '--shared' -WindowStyle Hidden -PassThru
```

Right-click the **Teamy** notification icon (including the overflow area),
choose **Show console** and **Hide console**, and confirm the existing terminal
is unchanged; choose **Exit**, then check `$sharedProbe.WaitForExit(10000)` and
`$sharedProbe.ExitCode` for `True` and zero.
This mode deliberately models a shared console without attaching to one, so it
does not prove detection in a real ConPTY host.

Separately start the owned-console probe, which intentionally creates a visible
window:

```powershell
$ownedProbe = Start-Process -FilePath $trayProbe -ArgumentList '--owned' -WindowStyle Normal -PassThru
```

It explicitly allocates a
new standalone console; use the tray menu to hide and show that new window,
then choose **Exit** and check `$ownedProbe.WaitForExit(10000)` and
`$ownedProbe.ExitCode` for `True` and zero. Confirm the icon disappears. The existing
terminal must stay visible. The probe reads no keys, session logs, or telemetry
and writes no files or settings; its only input is those tray menu selections.
Do not restart Explorer for this minimal test. Real Explorer restart recovery,
additional terminal hosts and actual ConPTY ownership checks remain separate
unqualified tests. Neither probe has been run.

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

## Application messages and disconnects

`Message::Intent` carries a child-to-owner application intent and `Message::AppEvent`
carries transient owner-to-child state. Each has a numeric kind and at most 4096
payload bytes; application adapters must define a closed typed enum, validate every
kind and payload, and reject unknown or invalid values. These messages never authorize
arbitrary commands. Generation checks and bounded queues apply to both directions;
transient state is not retained automatically and must not become a keystroke log.

`ChildEvent::Disconnected` reports clean pipe EOF independently of process exit.
The owner must immediately cancel any UI-bound operation such as binding capture,
preserve the previous binding, and call `Supervisor::shutdown` with an explicit
bounded deadline. At the deadline it requests termination only for the owned
child and performs one nonblocking exit check. If exit is unconfirmed, it reports
timeout and retains the process handle and session: do not spawn a replacement;
poll or retry shutdown until exit is observed. A failed termination also retains
ownership for a retry. A repeated shutdown does not repeatedly terminate a child
whose termination was already requested. If setup fails after starting a child,
check `is_running` and complete cleanup in the same way.

Dropping a supervisor makes one bounded cleanup attempt. If a child is still
unconfirmed, Drop emits a tracing error (if a subscriber is available) and releases ownership; it cannot guarantee
that the operating system has stopped the process or that blocked pipe workers
have finished. An owner that needs this guarantee must keep the supervisor alive,
surface the error, and retry or poll. The deadline controls our sleeps and avoids
an infinite wait after termination; individual OS calls and scheduler delays are
not real-time guarantees. Background input matching and retained history remain
owner-managed.
