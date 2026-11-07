use std::io::Read;
use std::io::Write;
use std::io::{self};

pub const MAX_FRAME_BYTES: usize = 65_536;
/// Application-specific typed payloads have a smaller, independently bounded budget.
pub const MAX_APP_BYTES: usize = 4096;
const VERSION: u8 = 1;

/// Fixed typed control messages; record payloads are interpreted by the application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    Hello {
        generation: u64,
    },
    Ready {
        generation: u64,
    },
    Record {
        generation: u64,
        sequence: u64,
        payload: Vec<u8>,
    },
    Gap {
        generation: u64,
        evicted: u64,
        rejected: u64,
    },
    Ack {
        generation: u64,
        sequence: u64,
    },
    Close {
        generation: u64,
    },
    /// Child-to-owner application intent; the consumer validates the kind and payload.
    Intent {
        generation: u64,
        kind: u16,
        payload: Vec<u8>,
    },
    /// Owner-to-child transient application state, independent of retained history.
    AppEvent {
        generation: u64,
        kind: u16,
        payload: Vec<u8>,
    },
}

impl Message {
    #[must_use]
    pub fn generation(&self) -> u64 {
        match self {
            Self::Hello { generation }
            | Self::Ready { generation }
            | Self::Record { generation, .. }
            | Self::Gap { generation, .. }
            | Self::Ack { generation, .. }
            | Self::Close { generation }
            | Self::Intent { generation, .. }
            | Self::AppEvent { generation, .. } => *generation,
        }
    }
}

/// Write one bounded frame and flush it.
/// # Errors
/// Returns an error for oversized payloads or a failed write/flush.
pub fn write_message(mut output: impl Write, message: &Message) -> io::Result<()> {
    if let Message::Intent { payload, .. } | Message::AppEvent { payload, .. } = message
        && payload.len() > MAX_APP_BYTES
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "application message exceeds bound",
        ));
    }
    if let Message::Record { payload, .. } = message
        && payload.len() > MAX_FRAME_BYTES - 18
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "UI frame exceeds bound",
        ));
    }
    let mut body = Vec::new();
    let (tag, generation) = match message {
        Message::Hello { generation } => (1, *generation),
        Message::Ready { generation } => (2, *generation),
        Message::Record { generation, .. } => (3, *generation),
        Message::Gap { generation, .. } => (4, *generation),
        Message::Ack { generation, .. } => (5, *generation),
        Message::Close { generation } => (6, *generation),
        Message::Intent { generation, .. } => (7, *generation),
        Message::AppEvent { generation, .. } => (8, *generation),
    };
    body.extend_from_slice(&[VERSION, tag]);
    body.extend_from_slice(&generation.to_be_bytes());
    match message {
        Message::Record {
            sequence, payload, ..
        } => {
            body.extend_from_slice(&sequence.to_be_bytes());
            body.extend_from_slice(payload);
        }
        Message::Gap {
            evicted, rejected, ..
        } => {
            body.extend_from_slice(&evicted.to_be_bytes());
            body.extend_from_slice(&rejected.to_be_bytes());
        }
        Message::Ack { sequence, .. } => body.extend_from_slice(&sequence.to_be_bytes()),
        Message::Intent { kind, payload, .. } | Message::AppEvent { kind, payload, .. } => {
            body.extend_from_slice(&kind.to_be_bytes());
            body.extend_from_slice(payload);
        }
        _ => {}
    }
    if body.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "UI frame exceeds bound",
        ));
    }
    let len = u32::try_from(body.len()).map_err(io::Error::other)?;
    output.write_all(&len.to_be_bytes())?;
    output.write_all(&body)?;
    output.flush()
}

/// Read one frame, rejecting unknown versions, invalid shapes and partial input.
/// # Errors
/// Returns an error for EOF, malformed/oversized frames or input failures.
pub fn read_message(mut input: impl Read) -> io::Result<Message> {
    read_next(&mut input)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "UI stream ended"))
}

/// Return `None` only for clean EOF between frames; partial frames remain errors.
/// # Errors
/// Returns an error for malformed/oversized frames, partial EOF or input failures.
pub fn read_next(mut input: impl Read) -> io::Result<Option<Message>> {
    let mut header = [0u8; 4];
    loop {
        match input.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    input.read_exact(&mut header[1..])?;
    let len = usize::try_from(u32::from_be_bytes(header)).map_err(io::Error::other)?;
    if !(10..=MAX_FRAME_BYTES).contains(&len) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid UI frame length",
        ));
    }
    let mut body = vec![0u8; len];
    input.read_exact(&mut body)?;
    if body[0] != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported UI protocol version",
        ));
    }
    let number = |bytes: &[u8]| -> io::Result<u64> {
        Ok(u64::from_be_bytes(bytes.try_into().map_err(|_length| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid UI number")
        })?))
    };
    let generation = number(&body[2..10])?;
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid UI message shape");
    let message = match (body[1], len) {
        (1, 10) => Ok(Message::Hello { generation }),
        (2, 10) => Ok(Message::Ready { generation }),
        (3, 18..) => Ok(Message::Record {
            generation,
            sequence: number(&body[10..18])?,
            payload: body[18..].to_vec(),
        }),
        (4, 26) => Ok(Message::Gap {
            generation,
            evicted: number(&body[10..18])?,
            rejected: number(&body[18..26])?,
        }),
        (5, 18) => Ok(Message::Ack {
            generation,
            sequence: number(&body[10..18])?,
        }),
        (6, 10) => Ok(Message::Close { generation }),
        (tag @ (7 | 8), 12..) if len - 12 <= MAX_APP_BYTES => {
            let kind = u16::from_be_bytes([body[10], body[11]]);
            let payload = body[12..].to_vec();
            if tag == 7 {
                Ok(Message::Intent {
                    generation,
                    kind,
                    payload,
                })
            } else {
                Ok(Message::AppEvent {
                    generation,
                    kind,
                    payload,
                })
            }
        }
        _ => Err(invalid()),
    }?;
    Ok(Some(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_messages_enforce_wire_bounds_and_shape() {
        let message = Message::Intent {
            generation: 8,
            kind: 17,
            payload: vec![3; MAX_APP_BYTES],
        };
        let mut wire = Vec::new();
        write_message(&mut wire, &message).unwrap();
        assert_eq!(read_message(wire.as_slice()).unwrap(), message);
        assert_eq!(
            write_message(
                Vec::new(),
                &Message::Intent {
                    generation: 8,
                    kind: 17,
                    payload: vec![0; MAX_APP_BYTES + 1]
                }
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidInput
        );
        let mut oversized = Vec::new();
        oversized.extend_from_slice(&u32::try_from(13 + MAX_APP_BYTES).unwrap().to_be_bytes());
        oversized.extend_from_slice(&[VERSION, 7]);
        oversized.extend_from_slice(&8u64.to_be_bytes());
        oversized.extend_from_slice(&17u16.to_be_bytes());
        oversized.resize(4 + 13 + MAX_APP_BYTES, 0);
        assert_eq!(
            read_message(oversized.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
