//! The client side of `chloro serve`: `--via-server` (or `CHLORO_VIA_SERVER=1`) sends the
//! run to the server on the socket and prints its reply, or runs it here when no server
//! answers or the server falls back (see `server.rs` for the protocol).

use super::args::Args;
use super::cache;
use super::protocol::{self, Frame};
use super::report::{AggregatedResults, finish};
use super::server::socket_path;
use std::io::{self, BufReader, BufWriter, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;

/// The frames of the request for this run, or `None` for a run the server does not take:
/// verbose output, print mode on paths, standard input that could not be read, and a
/// combination of options no run accepts (the client reports it).
pub fn request(args: &Args, stdin: Option<&io::Result<Vec<u8>>>) -> Option<Vec<Frame>> {
    if args.verbose || args.help || super::invalid_combination(args).is_some() {
        return None;
    }
    let stdin = match stdin {
        Some(Ok(bytes)) => Some(bytes),
        Some(Err(_)) => return None,
        None => None,
    };
    if stdin.is_none() && !(args.check || args.write) {
        return None;
    }
    let (identity, _) = cache::executable_identity()?;
    let cwd = std::env::current_dir().ok()?;
    let cache_dir = if args.no_cache {
        None
    } else {
        cache::resolve_dir(&args.cache_dir)
    };
    let mut frames = vec![
        Frame::new("chloro", protocol::VERSION),
        Frame::new("exe", identity),
        Frame::new("cwd", cwd.as_os_str().as_bytes()),
        Frame::new(
            "cache-dir",
            cache_dir
                .as_ref()
                .map_or(&[][..], |d| d.as_os_str().as_bytes()),
        ),
        Frame::new("config", args.config.as_str()),
        Frame::new("threads", args.threads.to_string()),
    ];
    if args.no_cache {
        frames.push(Frame::new("flag", "no-cache"));
    }
    if args.unit_cache {
        frames.push(Frame::new("flag", "unit-cache"));
    }
    match stdin {
        Some(bytes) => {
            let name = if args.check { "stdin-check" } else { "stdin" };
            frames.push(Frame::new(name, bytes.clone()));
        }
        None => {
            let mode = if args.check { "check" } else { "write" };
            frames.extend(args.sources.iter().map(|s| Frame::new(mode, s.as_str())));
        }
    }
    Some(frames)
}

/// The reply of a server: `(exit code, stdout, stderr)` when it answered the request, `None`
/// when it fell back or the exchange failed.
pub fn exchange(stream: &UnixStream, frames: &[Frame]) -> Option<(i32, Vec<u8>, Vec<u8>)> {
    protocol::write_message(&mut BufWriter::new(stream), frames).ok()?;
    let reply = protocol::read_message(&mut BufReader::new(stream)).ok()?;
    let (mut status, mut exit, mut stdout, mut stderr) = (None, None, Vec::new(), Vec::new());
    for frame in reply {
        match frame.name.as_str() {
            "status" => status = Some(frame.value),
            "exit" => exit = frame.text().ok()?.parse().ok(),
            "stdout" => stdout = frame.value,
            "stderr" => stderr = frame.value,
            _ => {}
        }
    }
    (status.as_deref() == Some(b"ok")).then_some((exit?, stdout, stderr))
}

/// Sends the run to the server and prints its reply: the exit code, or `None` to run here.
pub fn forward(args: &Args, stdin: Option<&io::Result<Vec<u8>>>) -> Option<i32> {
    let frames = request(args, stdin)?;
    let stream = UnixStream::connect(socket_path(args)?).ok()?;
    let (code, stdout, stderr) = exchange(&stream, &frames)?;
    let mut out = io::stdout();
    if let Err(e) = out.write_all(&stdout).and_then(|()| out.flush()) {
        // What the run in this process reports when it cannot print the formatted source
        // (only `--stdin` in print mode replies with standard output).
        let mut agg = AggregatedResults::new();
        agg.errors.push(format!("Failed to write to stdout: {e}"));
        return Some(finish(&agg, args, &mut io::stderr()));
    }
    if let Err(e) = io::stderr().write_all(&stderr) {
        panic!("failed printing to stderr: {e}");
    }
    Some(code)
}
