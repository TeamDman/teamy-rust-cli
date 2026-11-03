#![cfg(feature = "ui-process-fixtures")]
use std::ffi::OsString;
use std::io::Cursor;
use std::io::{self};
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use teamy_rust_cli::ui_process::ChildConsole;
use teamy_rust_cli::ui_process::ChildEvent;
use teamy_rust_cli::ui_process::History;
use teamy_rust_cli::ui_process::MAX_FRAME_BYTES;
use teamy_rust_cli::ui_process::Message;
use teamy_rust_cli::ui_process::Supervisor;
use teamy_rust_cli::ui_process::read_message;
use teamy_rust_cli::ui_process::read_next;
use teamy_rust_cli::ui_process::write_message;

fn spawn(supervisor: &mut Supervisor, mode: &str) -> io::Result<u64> {
    supervisor.spawn(
        Path::new(env!("CARGO_BIN_EXE_ui-child-fixture")),
        &[OsString::from(mode)],
        ChildConsole::Hidden,
    )
}

fn wait_for(supervisor: &mut Supervisor, predicate: impl Fn(&ChildEvent) -> bool) -> ChildEvent {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(event) = supervisor.poll().unwrap() {
            if predicate(&event) {
                return event;
            }
            if matches!(event, ChildEvent::Failed(_) | ChildEvent::Exited(_)) {
                panic!("unexpected synthetic child event: {event:?}");
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for synthetic child"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn framing_rejects_oversize_unknown_partial_and_distinguishes_eof() {
    let message = Message::Record {
        generation: 7,
        sequence: 9,
        payload: b"synthetic".to_vec(),
    };
    let mut bytes = Vec::new();
    write_message(&mut bytes, &message).unwrap();
    assert_eq!(read_message(Cursor::new(&bytes)).unwrap(), message);
    assert_eq!(read_next(Cursor::new([])).unwrap(), None);
    assert_eq!(
        read_message(Cursor::new(&bytes[..5])).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
    bytes[4] = 99;
    assert_eq!(
        read_message(Cursor::new(bytes)).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    let huge = Message::Record {
        generation: 1,
        sequence: 1,
        payload: vec![0; MAX_FRAME_BYTES],
    };
    assert_eq!(
        write_message(Vec::new(), &huge).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn retained_records_survive_ack_and_replay_for_new_generation() {
    let mut history = History::new(2, 32).unwrap();
    assert!(history.push(b"one".to_vec()));
    assert!(history.push(b"two".to_vec()));
    assert!(history.push(b"three".to_vec()));
    assert_eq!((history.stats().records, history.stats().evicted), (2, 1));
    assert!(!history.push(vec![0; 33]));
    assert_eq!(history.stats().rejected, 1);
    let mut supervisor = Supervisor::default();
    for expected in 1..=2 {
        if expected == 2 {
            assert!(history.push(b"while child closed".to_vec()));
        }
        let generation = spawn(&mut supervisor, "normal").unwrap();
        assert_eq!(generation, expected);
        assert_eq!(
            spawn(&mut supervisor, "normal").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        wait_for(&mut supervisor, |event| {
            matches!(event, ChildEvent::Message(Message::Ready { .. }))
        });
        supervisor
            .queue(Message::Gap {
                generation,
                evicted: history.stats().evicted,
                rejected: history.stats().rejected,
            })
            .unwrap();
        for entry in history.iter() {
            supervisor
                .queue(Message::Record {
                    generation,
                    sequence: entry.sequence,
                    payload: entry.payload.clone(),
                })
                .unwrap();
        }
        let last = history.stats().accepted;
        wait_for(
            &mut supervisor,
            |event| matches!(event, ChildEvent::Message(Message::Ack { sequence, .. }) if *sequence == last),
        );
        assert_eq!(
            history
                .iter()
                .map(|entry| entry.sequence)
                .collect::<Vec<_>>(),
            if expected == 1 {
                vec![2, 3]
            } else {
                vec![3, 4]
            }
        );
        assert_eq!(
            supervisor
                .queue(Message::Close {
                    generation: generation + 1
                })
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        supervisor.shutdown(Duration::from_secs(2)).unwrap();
    }
}

#[test]
fn child_crash_is_reaped_and_next_generation_can_start() {
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "crash").unwrap();
    let event = wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Exited(_))
    });
    assert!(matches!(event, ChildEvent::Exited(status) if status.code() == Some(23)));
    assert_eq!(spawn(&mut supervisor, "normal").unwrap(), 2);
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    supervisor.shutdown(Duration::from_secs(2)).unwrap();
}

#[test]
fn closing_ui_does_not_stop_owner_or_erase_history() {
    let mut history = History::new(3, 64).unwrap();
    assert!(history.push(b"before".to_vec()));
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "close-ui").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Close { .. }))
    });
    wait_for(
        &mut supervisor,
        |event| matches!(event, ChildEvent::Exited(status) if status.success()),
    );
    assert!(history.push(b"after".to_vec()));
    assert_eq!(spawn(&mut supervisor, "normal").unwrap(), 2);
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    for entry in history.iter() {
        supervisor
            .queue(Message::Record {
                generation: 2,
                sequence: entry.sequence,
                payload: entry.payload.clone(),
            })
            .unwrap();
    }
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ack { sequence: 2, .. }))
    });
    assert_eq!(history.stats().records, 2);
    supervisor.shutdown(Duration::from_secs(2)).unwrap();
}

#[test]
fn disconnect_is_observed_while_child_is_alive_and_deadline_cleans_it_up() {
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "disconnect").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Disconnected)
    });
    assert!(supervisor.is_running());
    assert_eq!(
        supervisor
            .shutdown(Duration::from_millis(10))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(!supervisor.is_running());
}

#[test]
fn application_intent_and_event_are_bounded_and_generation_checked() {
    use teamy_rust_cli::ui_process::MAX_APP_BYTES;
    let mut supervisor = Supervisor::default();
    let generation = spawn(&mut supervisor, "intent").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    wait_for(
        &mut supervisor,
        |event| matches!(event, ChildEvent::Message(Message::Intent { kind: 7, payload, .. }) if payload == &[1, 2]),
    );
    supervisor
        .queue(Message::AppEvent {
            generation,
            kind: 3,
            payload: vec![4],
        })
        .unwrap();
    assert_eq!(
        supervisor
            .queue(Message::AppEvent {
                generation,
                kind: 3,
                payload: vec![0; MAX_APP_BYTES + 1]
            })
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        supervisor
            .queue(Message::AppEvent {
                generation: generation + 1,
                kind: 3,
                payload: vec![]
            })
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    supervisor.shutdown(Duration::from_secs(2)).unwrap();
}

#[test]
fn relative_child_program_is_rejected() {
    let mut supervisor = Supervisor::default();
    assert_eq!(
        supervisor
            .spawn(Path::new("relative-child"), &[], ChildConsole::Hidden)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn malformed_and_partial_child_frames_are_reported() {
    for mode in ["malformed", "partial"] {
        let mut supervisor = Supervisor::default();
        spawn(&mut supervisor, mode).unwrap();
        wait_for(
            &mut supervisor,
            |event| matches!(event, ChildEvent::Failed(message) if message.contains("UI read")),
        );
        wait_for(&mut supervisor, |event| {
            matches!(event, ChildEvent::Exited(_))
        });
    }
}

#[test]
fn stale_generation_is_rejected() {
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "stale").unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Err(error) = supervisor.poll() {
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    supervisor.shutdown(Duration::from_secs(1)).unwrap();
}

#[test]
fn acknowledgement_for_unsent_record_is_rejected() {
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "bad-ack").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Err(error) = supervisor.poll() {
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    supervisor.shutdown(Duration::from_secs(1)).unwrap();
}

#[cfg(windows)]
#[test]
fn child_bootstrap_rejects_a_parent_id_that_is_not_its_parent() {
    let args = [
        OsString::from("--teamy-ui-ipc"),
        OsString::from(std::process::id().to_string()),
        OsString::from("0"),
        OsString::from("1"),
        OsString::from("2"),
    ];
    let Err(error) = teamy_rust_cli::ui_process::connect_child(&args) else {
        panic!("accepted a non-parent process");
    };
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn slow_reader_cannot_block_owner_and_owned_child_is_cleaned_up() {
    let mut supervisor = Supervisor::default();
    let generation = spawn(&mut supervisor, "slow").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    let mut full = false;
    for sequence in 1..=128 {
        if let Err(error) = supervisor.queue(Message::Record {
            generation,
            sequence,
            payload: vec![b'x'; 60_000],
        }) {
            assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
            full = true;
            break;
        }
    }
    assert!(full);
    assert!(supervisor.backpressure() > 0);
    assert_eq!(
        supervisor
            .shutdown(Duration::from_millis(10))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    assert!(!supervisor.is_running());
}

#[cfg(windows)]
#[test]
fn unrelated_inheritable_handle_does_not_reach_child() {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::io::FromRawHandle;
    use std::os::windows::io::OwnedHandle;
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::process::Stdio;
    use windows::Win32::Foundation::ERROR_BROKEN_PIPE;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Foundation::HANDLE_FLAG_INHERIT;
    use windows::Win32::Foundation::HANDLE_FLAGS;
    use windows::Win32::Foundation::SetHandleInformation;
    use windows::Win32::Security::SECURITY_ATTRIBUTES;
    use windows::Win32::System::Pipes::CreatePipe;
    use windows::Win32::System::Pipes::PeekNamedPipe;
    use windows::core::HRESULT;

    fn sentinel() -> (OwnedHandle, OwnedHandle) {
        let security = SECURITY_ATTRIBUTES {
            nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap(),
            bInheritHandle: true.into(),
            ..Default::default()
        };
        let (mut read, mut write) = (HANDLE::default(), HANDLE::default());
        // SAFETY: The output handles and attributes are valid; this creates an anonymous synthetic pipe.
        unsafe { CreatePipe(&raw mut read, &raw mut write, Some(&raw const security), 0) }.unwrap();
        // SAFETY: CreatePipe returned two distinct owned handles.
        let read = unsafe { OwnedHandle::from_raw_handle(read.0) };
        // SAFETY: The write handle is independently owned and transferred once.
        let write = unsafe { OwnedHandle::from_raw_handle(write.0) };
        // SAFETY: The read handle is valid; only the write end is left inheritable.
        unsafe {
            SetHandleInformation(
                HANDLE(read.as_raw_handle()),
                HANDLE_FLAG_INHERIT.0,
                HANDLE_FLAGS(0),
            )
        }
        .unwrap();
        (read, write)
    }
    fn peek(read: &OwnedHandle) -> windows::core::Result<()> {
        // SAFETY: The read end is valid; the zero-length peek does not block or access a buffer.
        unsafe { PeekNamedPipe(HANDLE(read.as_raw_handle()), None, 0, None, None, None) }
    }

    let (read, write) = sentinel();
    peek(&read).unwrap();
    let mut supervisor = Supervisor::default();
    spawn(&mut supervisor, "normal").unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    drop(write);
    assert_eq!(
        peek(&read).unwrap_err().code(),
        HRESULT::from_win32(ERROR_BROKEN_PIPE.0)
    );
    supervisor.shutdown(Duration::from_secs(2)).unwrap();

    // Positive control: a normal std::process spawn inherits the sentinel write end.
    let (read, write) = sentinel();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ui-child-fixture"));
    command
        .arg("normal")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0000);
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    write_message(&mut input, &Message::Hello { generation: 1 }).unwrap();
    assert_eq!(
        read_message(&mut output).unwrap(),
        Message::Ready { generation: 1 }
    );
    drop(write);
    peek(&read).unwrap();
    write_message(&mut input, &Message::Close { generation: 1 }).unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(
        peek(&read).unwrap_err().code(),
        HRESULT::from_win32(ERROR_BROKEN_PIPE.0)
    );
}

#[test]
fn launcher_preserves_spaces_quotes_and_trailing_slashes() {
    let mut supervisor = Supervisor::default();
    supervisor
        .spawn(
            Path::new(env!("CARGO_BIN_EXE_ui-child-fixture")),
            &[
                OsString::from("argv"),
                OsString::from("space \\\"quote\" trailing\\"),
            ],
            ChildConsole::Hidden,
        )
        .unwrap();
    wait_for(&mut supervisor, |event| {
        matches!(event, ChildEvent::Message(Message::Ready { .. }))
    });
    supervisor.shutdown(Duration::from_secs(2)).unwrap();
}
