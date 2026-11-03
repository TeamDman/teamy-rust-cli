use super::ChildConsole;
use super::Message;
use super::launcher::Bootstrap;
use super::launcher::Process;
use super::launcher::{self};
use super::read_next;
use super::write_message;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::thread::{self};
use std::time::Duration;
use std::time::Instant;

const QUEUE_CAPACITY: usize = 8;

#[derive(Debug)]
pub enum ChildEvent {
    Message(Message),
    /// The child's output pipe closed, even if the process is still alive.
    Disconnected,
    Exited(ExitStatus),
    Failed(String),
}

#[derive(Debug)]
struct Session {
    child: Process,
    bootstrap: Option<Bootstrap>,
    outbound: Option<mpsc::SyncSender<Message>>,
    inbound: mpsc::Receiver<ChildEvent>,
    overflow: Arc<AtomicBool>,
    writer: Option<JoinHandle<()>>,
    reader: Option<JoinHandle<()>>,
    ready: bool,
    sent: u64,
    ack: u64,
    outstanding: VecDeque<u64>,
    termination_requested: bool,
}

impl Session {
    fn new(
        child: Process,
        bootstrap: Option<Bootstrap>,
        outbound: Option<mpsc::SyncSender<Message>>,
        inbound: mpsc::Receiver<ChildEvent>,
        overflow: Arc<AtomicBool>,
        writer: Option<JoinHandle<()>>,
        reader: Option<JoinHandle<()>>,
    ) -> Self {
        Self {
            child,
            bootstrap,
            outbound,
            inbound,
            overflow,
            writer,
            reader,
            ready: false,
            sent: 0,
            ack: 0,
            outstanding: VecDeque::new(),
            termination_requested: false,
        }
    }
}

trait ManagedProcess {
    fn has_exited(&mut self) -> io::Result<bool>;
    fn terminate(&mut self) -> io::Result<()>;
}

impl ManagedProcess for Process {
    fn has_exited(&mut self) -> io::Result<bool> {
        Ok(self.try_wait()?.is_some())
    }
    fn terminate(&mut self) -> io::Result<()> {
        self.kill()
    }
}

#[derive(Debug, Eq, PartialEq)]
enum StopOutcome {
    Exited,
    ForcedExited,
    TerminationPending,
}

// No wait occurs after requesting termination: a still-live process stays owned.
fn stop_process(
    process: &mut impl ManagedProcess,
    termination_requested: &mut bool,
    timeout: Duration,
) -> io::Result<StopOutcome> {
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "UI shutdown deadline exceeds clock range",
        )
    })?;
    loop {
        if process.has_exited()? {
            return Ok(StopOutcome::Exited);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(Duration::from_millis(5)));
    }
    if *termination_requested {
        return Ok(StopOutcome::TerminationPending);
    }
    if let Err(error) = process.terminate() {
        return if process.has_exited()? {
            Ok(StopOutcome::Exited)
        } else {
            Err(error)
        };
    }
    *termination_requested = true;
    if process.has_exited()? {
        Ok(StopOutcome::ForcedExited)
    } else {
        Ok(StopOutcome::TerminationPending)
    }
}

fn start_writer(
    mut input: Box<dyn io::Write + Send>,
    outbound: mpsc::Receiver<Message>,
    errors: mpsc::SyncSender<ChildEvent>,
    overflow: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("ui-pipe-writer".into())
        .spawn(move || {
            while let Ok(message) = outbound.recv() {
                if let Err(error) = write_message(&mut input, &message) {
                    if errors
                        .try_send(ChildEvent::Failed(format!("UI write: {error}")))
                        .is_err()
                    {
                        overflow.store(true, Ordering::Release);
                    }
                    break;
                }
            }
        })
}

fn start_reader(
    mut output: Box<dyn io::Read + Send>,
    inbound: mpsc::SyncSender<ChildEvent>,
    overflow: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("ui-pipe-reader".into())
        .spawn(move || {
            loop {
                let event = match read_next(&mut output) {
                    Ok(Some(message)) => ChildEvent::Message(message),
                    Ok(None) => ChildEvent::Disconnected,
                    Err(error) => ChildEvent::Failed(format!("UI read: {error}")),
                };
                let stop = matches!(event, ChildEvent::Failed(_) | ChildEvent::Disconnected);
                if inbound.try_send(event).is_err() {
                    overflow.store(true, Ordering::Release);
                    break;
                }
                if stop {
                    break;
                }
            }
        })
}

fn validate_reply(
    session: &mut Session,
    generation: u64,
    event: ChildEvent,
) -> io::Result<ChildEvent> {
    if let ChildEvent::Message(message) = &event {
        if message.generation() != generation {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "stale UI reply"));
        }
        match message {
            Message::Ready { .. } if !session.ready => {
                session.ready = true;
                session.bootstrap.take();
            }
            Message::Ack { sequence, .. }
                if session.ready
                    && *sequence >= session.ack
                    && session.outstanding.contains(sequence) =>
            {
                session.ack = *sequence;
                while session
                    .outstanding
                    .front()
                    .is_some_and(|sent| *sent <= *sequence)
                {
                    session.outstanding.pop_front();
                }
            }
            Message::Close { .. } | Message::Intent { .. } if session.ready => {}
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid UI reply",
                ));
            }
        }
    }
    Ok(event)
}

/// One child at a time. The caller selects a trusted executable and console mode.
#[derive(Debug, Default)]
pub struct Supervisor {
    generation: u64,
    session: Option<Session>,
    backpressure: u64,
}

impl Supervisor {
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
    #[must_use]
    pub fn backpressure(&self) -> u64 {
        self.backpressure
    }
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.session.is_some()
    }
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.session.as_ref().is_some_and(|session| {
            session.ready && session.outbound.is_some() && !session.termination_requested
        })
    }

    /// Spawn a child with private pipes; its UI must use separate terminal handles.
    /// # Errors
    /// Returns an error if a child already exists, generation overflows, or spawn fails.
    /// If setup fails after process creation, the supervisor may retain that child
    /// for bounded cleanup; check `is_running` and retry shutdown or poll.
    pub fn spawn(
        &mut self,
        program: &Path,
        args: &[OsString],
        console: ChildConsole,
    ) -> io::Result<u64> {
        if self.session.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "UI child already exists",
            ));
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| io::Error::other("UI generation exhausted"))?;
        if !program.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "UI executable path must be absolute",
            ));
        }
        let launcher::Spawned {
            process: child,
            input,
            output,
            bootstrap,
        } = launcher::spawn(program, args, console)?;
        let (out_tx, out_rx) = mpsc::sync_channel::<Message>(QUEUE_CAPACITY);
        let (in_tx, in_rx) = mpsc::sync_channel::<ChildEvent>(QUEUE_CAPACITY);
        let overflow = Arc::new(AtomicBool::new(false));
        let writer_errors = in_tx.clone();
        let writer_overflow = Arc::clone(&overflow);
        let writer = start_writer(input, out_rx, writer_errors, writer_overflow);
        let writer = match writer {
            Ok(writer) => writer,
            Err(error) => {
                self.generation = generation;
                self.session = Some(Session::new(
                    child, bootstrap, None, in_rx, overflow, None, None,
                ));
                let cleanup = self.shutdown(Duration::ZERO);
                return Err(io::Error::other(format!(
                    "UI writer start: {error}; cleanup: {cleanup:?}"
                )));
            }
        };
        let reader_overflow = Arc::clone(&overflow);
        let reader = start_reader(output, in_tx, reader_overflow);
        let reader = match reader {
            Ok(reader) => reader,
            Err(error) => {
                drop(out_tx);
                self.generation = generation;
                self.session = Some(Session::new(
                    child,
                    bootstrap,
                    None,
                    in_rx,
                    overflow,
                    Some(writer),
                    None,
                ));
                let cleanup = self.shutdown(Duration::ZERO);
                return Err(io::Error::other(format!(
                    "UI reader start: {error}; cleanup: {cleanup:?}"
                )));
            }
        };
        self.generation = generation;
        self.session = Some(Session::new(
            child,
            bootstrap,
            Some(out_tx),
            in_rx,
            overflow,
            Some(writer),
            Some(reader),
        ));
        self.queue(Message::Hello { generation })?;
        Ok(generation)
    }

    /// Queue one message without blocking; a full queue is counted and reported.
    /// # Errors
    /// Returns an error for absent/stale/unready children or backpressure.
    pub fn queue(&mut self, message: Message) -> io::Result<()> {
        if !matches!(
            message,
            Message::Hello { .. }
                | Message::Close { .. }
                | Message::Record { .. }
                | Message::Gap { .. }
                | Message::AppEvent { .. }
        ) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid owner message",
            ));
        }
        if let Message::AppEvent { payload, .. } = &message
            && payload.len() > super::MAX_APP_BYTES
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "application message exceeds bound",
            ));
        }
        if message.generation() != self.generation {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "stale UI generation",
            ));
        }
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no UI child"))?;
        if !session.ready && !matches!(message, Message::Hello { .. } | Message::Close { .. }) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "UI handshake incomplete",
            ));
        }
        let sequence = if let Message::Record {
            sequence, payload, ..
        } = &message
        {
            if payload.len() > super::MAX_FRAME_BYTES - 18 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "UI frame exceeds bound",
                ));
            }
            if *sequence <= session.sent {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "UI sequence must increase",
                ));
            }
            if session.outstanding.len() >= 64 {
                self.backpressure = self.backpressure.saturating_add(1);
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "UI acknowledgement window full",
                ));
            }
            Some(*sequence)
        } else {
            None
        };
        let sender = session
            .outbound
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "UI sender closed"))?;
        match sender.try_send(message) {
            Ok(()) => {
                if let Some(sequence) = sequence {
                    session.sent = sequence;
                    session.outstanding.push_back(sequence);
                }
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.backpressure = self.backpressure.saturating_add(1);
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "UI delivery queue full",
                ))
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "UI writer stopped",
            )),
        }
    }

    /// Poll messages and child exit without blocking. Stale or invalid replies are errors.
    /// # Errors
    /// Returns an error on queue overflow, malformed state or process wait failure.
    pub fn poll(&mut self) -> io::Result<Option<ChildEvent>> {
        let Some(session) = self.session.as_mut() else {
            return Ok(None);
        };
        if session.overflow.swap(false, Ordering::AcqRel) {
            return Err(io::Error::other("UI event queue overflow"));
        }
        if let Ok(event) = session.inbound.try_recv() {
            return validate_reply(session, self.generation, event).map(Some);
        }
        if let Some(status) = session.child.try_wait()? {
            session.bootstrap.take();
            if !session.reader.as_ref().is_none_or(JoinHandle::is_finished) {
                return Ok(None);
            }
            if let Ok(event) = session.inbound.try_recv() {
                return validate_reply(session, self.generation, event).map(Some);
            }
            let mut finished = self
                .session
                .take()
                .ok_or_else(|| io::Error::other("UI session disappeared"))?;
            finished.outbound.take();
            for thread in [&mut finished.writer, &mut finished.reader] {
                if thread.as_ref().is_some_and(JoinHandle::is_finished)
                    && let Some(thread) = thread.take()
                {
                    let _ = thread.join();
                }
            }
            return Ok(Some(ChildEvent::Exited(status)));
        }
        Ok(None)
    }

    /// Request close, then request termination only for this owned child at the deadline.
    /// If exit is not yet observed, retain the session and handle for polling or retry.
    /// # Errors
    /// Returns an error if the child cannot be stopped or reaped.
    pub fn shutdown(&mut self, timeout: Duration) -> io::Result<()> {
        if self.session.is_none() {
            return Ok(());
        }
        let _ = self.queue(Message::Close {
            generation: self.generation,
        });
        if let Some(session) = self.session.as_mut() {
            session.outbound.take();
        }
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| io::Error::other("UI session disappeared"))?;
        let outcome = stop_process(
            &mut session.child,
            &mut session.termination_requested,
            timeout,
        )?;
        if outcome != StopOutcome::TerminationPending {
            let mut session = self
                .session
                .take()
                .ok_or_else(|| io::Error::other("UI session disappeared"))?;
            for thread in [&mut session.writer, &mut session.reader] {
                if thread.as_ref().is_some_and(JoinHandle::is_finished)
                    && let Some(thread) = thread.take()
                {
                    let _ = thread.join();
                }
            }
        }
        match outcome {
            StopOutcome::Exited => Ok(()),
            StopOutcome::ForcedExited => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "owned UI child terminated after shutdown deadline",
            )),
            StopOutcome::TerminationPending => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "owned UI child termination requested; exit not yet observed",
            )),
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown(Duration::from_millis(250))
            && self.is_running()
        {
            tracing::error!(%error, "UI child cleanup incomplete; process may still be running");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeProcess {
        exited: bool,
        exit_on_terminate: bool,
        terminate_fails: bool,
        terminations: usize,
    }
    impl ManagedProcess for FakeProcess {
        fn has_exited(&mut self) -> io::Result<bool> {
            Ok(self.exited)
        }
        fn terminate(&mut self) -> io::Result<()> {
            self.terminations += 1;
            if self.terminate_fails {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected failure",
                ));
            }
            self.exited = self.exit_on_terminate;
            Ok(())
        }
    }

    #[test]
    fn never_signalled_termination_is_bounded_and_retained_for_retry() {
        let mut process = FakeProcess::default();
        let mut requested = false;
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO).unwrap(),
            StopOutcome::TerminationPending
        );
        assert!(requested);
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO).unwrap(),
            StopOutcome::TerminationPending
        );
        assert_eq!(process.terminations, 1);
        process.exited = true;
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO).unwrap(),
            StopOutcome::Exited
        );
    }

    #[test]
    fn terminate_failure_does_not_report_exit_or_consume_retry() {
        let mut process = FakeProcess {
            terminate_fails: true,
            ..FakeProcess::default()
        };
        let mut requested = false;
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(!requested);
        assert!(!process.exited);
        process.terminate_fails = false;
        process.exit_on_terminate = true;
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO).unwrap(),
            StopOutcome::ForcedExited
        );
        assert_eq!(process.terminations, 2);
    }

    #[test]
    fn already_exited_child_is_not_terminated() {
        let mut process = FakeProcess {
            exited: true,
            ..FakeProcess::default()
        };
        let mut requested = false;
        assert_eq!(
            stop_process(&mut process, &mut requested, Duration::ZERO).unwrap(),
            StopOutcome::Exited
        );
        assert_eq!(process.terminations, 0);
    }
}
