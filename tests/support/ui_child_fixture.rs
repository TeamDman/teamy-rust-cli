//! Invented child protocol fixture. It never opens a console or network listener.
use std::io::Write;
use std::io::{self};
use teamy_rust_cli::ui_process::Message;
use teamy_rust_cli::ui_process::connect_child;
use teamy_rust_cli::ui_process::read_message;
use teamy_rust_cli::ui_process::write_message;

fn main() -> io::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let mode = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("normal");
    if mode == "crash" {
        std::process::exit(23);
    }
    let (mut input, mut output): (Box<dyn io::Read + Send>, Box<dyn Write + Send>) =
        if args.iter().any(|value| value == "--teamy-ui-ipc") {
            connect_child(&args)?
        } else {
            (Box::new(io::stdin()), Box::new(io::stdout()))
        };
    if mode == "malformed" {
        output.write_all(&[0, 0, 0, 10, 2, 1, 0, 0, 0, 0, 0, 0, 0, 1])?;
        return Ok(());
    }
    if mode == "partial" {
        output.write_all(&[0, 0, 0, 10, 1])?;
        return Ok(());
    }
    let Message::Hello { generation } = read_message(&mut input)? else {
        return Err(io::Error::other("expected hello"));
    };
    if mode == "argv" && std::env::args().nth(2).as_deref() != Some("space \\\"quote\" trailing\\")
    {
        return Err(io::Error::other("argument quoting changed"));
    }
    let reply_generation = if mode == "stale" {
        generation.saturating_sub(1)
    } else {
        generation
    };
    write_message(
        &mut output,
        &Message::Ready {
            generation: reply_generation,
        },
    )?;
    if mode == "stale" {
        return Ok(());
    }
    if mode == "close-ui" {
        write_message(&mut output, &Message::Close { generation })?;
        return Ok(());
    }
    if mode == "bad-ack" {
        write_message(
            &mut output,
            &Message::Ack {
                generation,
                sequence: 42,
            },
        )?;
        return Ok(());
    }
    if mode == "slow" {
        std::thread::sleep(std::time::Duration::from_millis(800));
        return Ok(());
    }
    loop {
        match read_message(&mut input) {
            Ok(Message::Record {
                generation,
                sequence,
                ..
            }) => write_message(
                &mut output,
                &Message::Ack {
                    generation,
                    sequence,
                },
            )?,
            Ok(Message::Gap { .. }) => {}
            Ok(Message::Close { .. }) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Ok(_) => return Err(io::Error::other("unexpected message")),
            Err(error) => return Err(error),
        }
    }
}
