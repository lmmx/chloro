//! The framing of the `chloro serve` protocol.
//!
//! A message is a sequence of frames, each a header line `NAME LEN\n` followed by exactly
//! `LEN` bytes, and ends with the frame `end 0\n`. `NAME` is 1 to [`MAX_NAME`] bytes of
//! `a-z`, `0-9` and `-`; `LEN` is decimal, without sign or leading zeros (except `0`), at
//! most [`MAX_VALUE`]. A message holds at most [`MAX_FRAMES`] frames. The frames a request
//! and a reply carry are documented in `server.rs` and in `chloro --help`.

use std::io::{self, BufRead, Write};

/// The protocol version a request names in its first frame (`chloro 1`).
pub const VERSION: &[u8] = b"1";

/// Longest frame name.
pub const MAX_NAME: usize = 32;

/// Largest frame value: 1 GiB.
pub const MAX_VALUE: usize = 1 << 30;

/// Most frames in one message.
pub const MAX_FRAMES: usize = 1 << 20;

/// One frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub name: String,
    pub value: Vec<u8>,
}

impl Frame {
    /// A frame of `name` and `value`.
    pub fn new(name: &str, value: impl Into<Vec<u8>>) -> Frame {
        Frame {
            name: name.to_string(),
            value: value.into(),
        }
    }

    /// The value as UTF-8.
    pub fn text(&self) -> io::Result<&str> {
        std::str::from_utf8(&self.value)
            .map_err(|_| invalid(format!("frame `{}` is not UTF-8", self.name)))
    }
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn valid_name(name: &str) -> bool {
    (1..=MAX_NAME).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Writes one frame.
///
/// # Errors
///
/// An invalid name or a value over [`MAX_VALUE`] (`InvalidInput`), or the writer's error.
pub fn write_frame(w: &mut dyn Write, name: &str, value: &[u8]) -> io::Result<()> {
    if !valid_name(name) || name == "end" || value.len() > MAX_VALUE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot frame `{name}` of {} bytes", value.len()),
        ));
    }
    writeln!(w, "{name} {}", value.len())?;
    w.write_all(value)
}

/// Writes `frames` and the end frame, then flushes.
///
/// # Errors
///
/// As [`write_frame`].
pub fn write_message(w: &mut dyn Write, frames: &[Frame]) -> io::Result<()> {
    for frame in frames {
        write_frame(w, &frame.name, &frame.value)?;
    }
    w.write_all(b"end 0\n")?;
    w.flush()
}

/// Reads one header line: the name and the length. `None` at the end frame.
fn read_header(r: &mut dyn BufRead) -> io::Result<Option<(String, usize)>> {
    // `NAME LEN\n`: at most the name, a space, ten digits and the newline.
    let mut line = Vec::with_capacity(48);
    let limit = MAX_NAME + 1 + 10 + 1;
    loop {
        let buf = r.fill_buf()?;
        if buf.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the message ended inside a frame header",
            ));
        }
        let take = buf.len().min(limit + 1 - line.len());
        match buf[..take].iter().position(|&b| b == b'\n') {
            Some(i) => {
                line.extend_from_slice(&buf[..i]);
                r.consume(i + 1);
                break;
            }
            None => {
                line.extend_from_slice(&buf[..take]);
                r.consume(take);
                if line.len() > limit {
                    return Err(invalid("a frame header is too long".to_string()));
                }
            }
        }
    }
    let line =
        std::str::from_utf8(&line).map_err(|_| invalid("a frame header is not ASCII".into()))?;
    let (name, len) = line
        .split_once(' ')
        .ok_or_else(|| invalid(format!("malformed frame header `{line}`")))?;
    let canonical = !len.is_empty()
        && len.bytes().all(|b| b.is_ascii_digit())
        && (len == "0" || !len.starts_with('0'));
    if !valid_name(name) || !canonical {
        return Err(invalid(format!("malformed frame header `{line}`")));
    }
    let len: usize = len
        .parse()
        .ok()
        .filter(|&n| n <= MAX_VALUE)
        .ok_or_else(|| invalid(format!("frame `{name}` is too long")))?;
    if name == "end" {
        if len != 0 {
            return Err(invalid("the end frame has a value".to_string()));
        }
        return Ok(None);
    }
    Ok(Some((name.to_string(), len)))
}

/// Reads one message: its frames, up to and without the end frame.
///
/// # Errors
///
/// `UnexpectedEof` when the stream ends inside the message, `InvalidData` for a malformed
/// header, a value over [`MAX_VALUE`] or more than [`MAX_FRAMES`] frames, or the reader's
/// error.
pub fn read_message(r: &mut dyn BufRead) -> io::Result<Vec<Frame>> {
    let mut frames = Vec::new();
    while let Some((name, len)) = read_header(r)? {
        if frames.len() == MAX_FRAMES {
            return Err(invalid("too many frames".to_string()));
        }
        let mut value = Vec::new();
        let read = io::Read::read_to_end(&mut io::Read::take(&mut *r, len as u64), &mut value)?;
        if read != len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("the message ended inside frame `{name}`"),
            ));
        }
        frames.push(Frame { name, value });
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(frames: &[Frame]) -> Vec<Frame> {
        let mut bytes = Vec::new();
        write_message(&mut bytes, frames).unwrap();
        read_message(&mut &bytes[..]).unwrap()
    }

    #[test]
    fn frames_round_trip_with_any_bytes() {
        let frames = vec![
            Frame::new("chloro", VERSION),
            Frame::new("stdin", b"fn a() {}\nend 0\n\0\xff".to_vec()),
            Frame::new("check", "a b\nc.rs"),
            Frame::new("flag", ""),
        ];
        assert_eq!(round_trip(&frames), frames);
        assert_eq!(round_trip(&[]), vec![]);
    }

    #[test]
    fn the_wire_format_is_as_documented() {
        let mut bytes = Vec::new();
        write_message(
            &mut bytes,
            &[Frame::new("check", "a.rs"), Frame::new("x-1", "")],
        )
        .unwrap();
        assert_eq!(bytes, b"check 4\na.rsx-1 0\nend 0\n");
    }

    #[test]
    fn two_messages_on_one_stream_are_read_in_turn() {
        let mut bytes = Vec::new();
        write_message(&mut bytes, &[Frame::new("ping", "")]).unwrap();
        write_message(&mut bytes, &[Frame::new("stop", "now")]).unwrap();
        let mut r = &bytes[..];
        assert_eq!(read_message(&mut r).unwrap(), vec![Frame::new("ping", "")]);
        assert_eq!(
            read_message(&mut r).unwrap(),
            vec![Frame::new("stop", "now")]
        );
        assert!(r.is_empty());
    }

    #[test]
    fn a_value_is_read_across_buffer_boundaries() {
        let value = vec![b'x'; 100_000];
        let mut bytes = Vec::new();
        write_message(&mut bytes, &[Frame::new("stdin", value.clone())]).unwrap();
        let mut r = io::BufReader::with_capacity(7, &bytes[..]);
        assert_eq!(
            read_message(&mut r).unwrap(),
            vec![Frame::new("stdin", value)]
        );
    }

    #[test]
    fn malformed_messages_are_refused() {
        for bad in [
            &b""[..],
            b"check 4\na.r",
            b"check 4\na.rs",
            b"check 4\na.rsend 1\nx",
            b"Check 1\nx\nend 0\n",
            b"check 01\nx\nend 0\n",
            b"check -1\nend 0\n",
            b"check +1\nx\nend 0\n",
            b"check1\nx\nend 0\n",
            b"check  1\nx\nend 0\n",
            b"check 99999999999\n",
            b"check 1073741825\n",
            b"a-very-long-frame-name-over-the-limit 0\nend 0\n",
            b"check 1\r\nx\nend 0\n",
        ] {
            let result = read_message(&mut &bad[..]);
            assert!(
                result.is_err(),
                "accepted {:?}",
                String::from_utf8_lossy(bad)
            );
        }
        // An endless header line is refused without reading it all.
        let endless = vec![b'a'; 1 << 20];
        let err = read_message(&mut &endless[..]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn unwritable_frames_are_refused() {
        let mut sink = Vec::new();
        assert!(write_frame(&mut sink, "end", b"").is_err());
        assert!(write_frame(&mut sink, "Bad", b"").is_err());
        assert!(write_frame(&mut sink, "", b"").is_err());
        assert!(sink.is_empty());
    }
}
