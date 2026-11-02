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
shell-restart logic. It has not been activated or visually qualified. Codex
and Defibrillator do not yet use it; Defibrillator can adapt its existing state
and backend after reviewing its separate ownership and input lifecycle.

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
