//! `chloro serve`: a resident process that answers formatting requests on a Unix-domain
//! socket, keeping the caches of one cache directory in memory (experiment E21,
//! `docs/journal/2026-10-14-autoresearch/experiments/EXP-021-resident.md`).
//!
//! # What it keeps
//!
//! - per distinct configuration: the parsed `Config`, the fixed-point cache ([`Cache`], read
//!   from its file once, recorded in memory on every request) and the unit cache ([`Units`],
//!   read from its file on the first request that uses it, kept in memory, bounded to
//!   [`units::MAX_BYTES`]);
//! - per path: the stamp of the file when its content was last read and hashed
//!   ([`StatTable`]), so a request on an unchanged formatted file reads nothing;
//! - its executable's identity ([`cache::executable_identity`]), compared with every client's.
//!
//! The caches are written to their files (as the CLI writes them) [`SAVE_DELAY`] after the
//! last request that changed them, and when the server exits.
//!
//! # Requests
//!
//! One request per connection, in the framing of [`protocol`]. Frames of a request, in any
//! order after the first:
//!
//! | frame | value |
//! |---|---|
//! | `chloro` | the protocol version, `1` (first frame) |
//! | `exe` | the client's executable identity; a request from another executable is refused |
//! | `cwd` | the client's working directory, against which relative paths resolve |
//! | `cache-dir` | the cache directory the client resolved (empty with `--no-cache`) |
//! | `config` | the `--config` string |
//! | `threads` | the `--threads` value (`0`: the server's own, see `State::item_threads`) |
//! | `flag` | `no-cache` or `unit-cache`, once per flag given |
//! | `check`, `write` | a source path, once per path; one mode per request |
//! | `stdin`, `stdin-check` | the source text (`--stdin`, `--stdin --check`) |
//! | `ping`, `status`, `stop` | administration, alone after `chloro` (value ignored) |
//!
//! The reply: `status` (`ok`, `fallback` or `error`), `reason` (for `fallback` and `error`),
//! `exit` (decimal), `stdout`, `stderr`; the reply to `ping` adds `exe`, the server's
//! executable identity (its process id is the `stdout`), and the reply to `status` gives
//! counts and memory as text lines in `stdout`. On `fallback` the client runs the request itself:
//! the server refuses a request from another executable, for another cache directory, with
//! an unknown frame or version, or one that panicked.
//!
//! # Lifecycle
//!
//! One server per socket, by default `chloro.sock` in the cache directory (so one per cache
//! directory; see [`socket_path`] for a cache directory whose path is too long for a socket). A server that finds the socket answering exits; a socket nobody answers on
//! (left by a killed server) is replaced. The server exits after `--idle-timeout` seconds
//! without a request, on a `stop` request, and when the file of its executable changes on
//! disk (checked every [`TICK`] and on every refused request), removing its socket. A
//! killed server leaves its socket and loses what it recorded since its last write; clients
//! then run requests themselves.

use super::args::Args;
use super::cache::{self, Cache};
use super::protocol::{self, Frame};
use super::report::aggregate_results;
use super::stat::StatTable;
use super::units::{self, Units};
use super::worker::{Caches, PARALLEL_ITEMS_MIN_BYTES, format_file};
use super::{collect_files, format_all, invalid_combination, report::finish, run_stdin};
use crate::ewriteln;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufReader, BufWriter, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// File name of the socket in the cache directory.
pub const SOCKET_NAME: &str = "chloro.sock";

/// How long after the last request that changed a cache the server writes it.
pub const SAVE_DELAY: Duration = Duration::from_secs(2);

/// Interval of the server's checks for idleness and for a changed executable.
pub const TICK: Duration = Duration::from_secs(1);

/// How long the server waits for a request's bytes once a client connected.
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Whether the command line is `chloro serve ...`.
pub fn is_serve_invocation(args: &Args) -> bool {
    std::env::args_os().nth(1).is_some_and(|a| a == "serve")
        && args.sources.first().is_some_and(|s| s == "serve")
}

/// Longest socket path the platform accepts: `sun_path` (108 bytes on Linux, 104 on the
/// BSDs and macOS) less its terminating zero.
pub const MAX_SOCKET_PATH: usize = if cfg!(target_os = "linux") { 107 } else { 103 };

/// The socket named by `--socket`, else `chloro.sock` in the cache directory. When that
/// path is too long for a socket, `chloro-<hash of the cache directory>.sock` in
/// `$XDG_RUNTIME_DIR` (a directory only the user can enter), else none.
pub fn socket_path(args: &Args) -> Option<PathBuf> {
    if !args.socket.is_empty() {
        return Some(PathBuf::from(&args.socket));
    }
    let dir = cache::resolve_dir(&args.cache_dir)?;
    let path = dir.join(SOCKET_NAME);
    if path.as_os_str().len() <= MAX_SOCKET_PATH {
        return Some(path);
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty())?;
    let hash = blake3::hash(std::os::unix::ffi::OsStrExt::as_bytes(dir.as_os_str()));
    let hex: String = hash.as_bytes()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Some(PathBuf::from(runtime).join(format!("chloro-{hex}.sock")))
}

/// The caches of one configuration.
struct ConfigState {
    cache: Cache,
    units: Units,
    /// A request recorded something the files do not hold yet.
    dirty: bool,
}

/// Everything the server keeps between requests.
struct State {
    dir: Option<PathBuf>,
    identity: Vec<u8>,
    exe_path: Option<PathBuf>,
    /// Threads for a request of several files (`format_all`): `--threads`, else one per core.
    threads: usize,
    /// Threads for the items of one large file in a request of one source (a file or
    /// standard input): `--threads`, else 1. Item threads started on every request leave
    /// glibc's per-thread arenas fragmented: unpinned, the server's RSS grew 1.10-2.43x over
    /// 1,000 whole-file saves of the large fixtures with one per core, and 1.00-1.07x with one
    /// (EXP-021, w4).
    item_threads: usize,
    /// By `Config::key_bytes`; `None` when the cache cannot be opened.
    configs: HashMap<Vec<u8>, Option<ConfigState>>,
    stats: StatTable,
    last_request: Instant,
    requests: u64,
    /// Time spent answering requests, from the parsed request to the reply.
    busy: Duration,
    socket: PathBuf,
    /// The inode the server bound, to remove only its own socket.
    socket_ino: Option<u64>,
}

/// A parsed request.
#[derive(Debug, Default)]
struct Request {
    exe: Vec<u8>,
    cwd: PathBuf,
    cache_dir: Option<PathBuf>,
    config: String,
    threads: usize,
    no_cache: bool,
    unit_cache: bool,
    mode: Option<&'static str>,
    sources: Vec<String>,
    stdin: Option<Vec<u8>>,
    admin: Option<&'static str>,
}

/// A reply.
struct Reply {
    status: &'static str,
    reason: String,
    exit: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    /// The server exits after sending it.
    then_exit: bool,
    /// The server's executable identity, in the reply to `ping` (frame `exe`): what a
    /// client that is not chloro sends as its `exe`.
    identity: Option<Vec<u8>>,
}

impl Reply {
    fn ok(exit: i32, stdout: Vec<u8>, stderr: Vec<u8>) -> Reply {
        Reply {
            status: "ok",
            reason: String::new(),
            exit,
            stdout,
            stderr,
            then_exit: false,
            identity: None,
        }
    }

    fn fallback(reason: impl Into<String>) -> Reply {
        Reply {
            status: "fallback",
            reason: reason.into(),
            exit: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            then_exit: false,
            identity: None,
        }
    }

    fn frames(&self) -> Vec<Frame> {
        let mut frames = vec![Frame::new("status", self.status)];
        if !self.reason.is_empty() {
            frames.push(Frame::new("reason", self.reason.as_str()));
        }
        frames.push(Frame::new("exit", self.exit.to_string()));
        frames.push(Frame::new("stdout", self.stdout.clone()));
        frames.push(Frame::new("stderr", self.stderr.clone()));
        if let Some(identity) = &self.identity {
            frames.push(Frame::new("exe", identity.clone()));
        }
        frames
    }
}

fn unix_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

/// Parses the frames of a request; `Err` with the reason to fall back.
fn parse_request(frames: Vec<Frame>) -> Result<Request, String> {
    let mut frames = frames.into_iter();
    match frames.next() {
        Some(f) if f.name == "chloro" && f.value == protocol::VERSION => {}
        Some(f) if f.name == "chloro" => {
            return Err(format!(
                "protocol version {} is not supported",
                String::from_utf8_lossy(&f.value)
            ));
        }
        _ => return Err("the request does not start with `chloro`".to_string()),
    }
    let mut request = Request::default();
    let text = |f: &Frame| f.text().map(str::to_string).map_err(|e| e.to_string());
    for frame in frames {
        let mode = match frame.name.as_str() {
            "exe" => {
                request.exe = frame.value;
                continue;
            }
            "cwd" => {
                request.cwd = unix_path(&frame.value);
                continue;
            }
            "cache-dir" => {
                request.cache_dir = (!frame.value.is_empty()).then(|| unix_path(&frame.value));
                continue;
            }
            "config" => {
                request.config = text(&frame)?;
                continue;
            }
            "threads" => {
                request.threads = text(&frame)?
                    .parse()
                    .map_err(|_| "`threads` is not a number".to_string())?;
                continue;
            }
            "flag" => {
                match frame.value.as_slice() {
                    b"no-cache" => request.no_cache = true,
                    b"unit-cache" => request.unit_cache = true,
                    other => {
                        return Err(format!("unknown flag `{}`", String::from_utf8_lossy(other)));
                    }
                }
                continue;
            }
            "ping" | "status" | "stop" => {
                if request.admin.is_some() || request.mode.is_some() {
                    return Err("an administrative request comes alone".to_string());
                }
                request.admin = Some(match frame.name.as_str() {
                    "ping" => "ping",
                    "status" => "status",
                    _ => "stop",
                });
                continue;
            }
            "check" => "check",
            "write" => "write",
            "stdin" => "stdin",
            "stdin-check" => "stdin-check",
            other => return Err(format!("unknown frame `{other}`")),
        };
        if request.admin.is_some() || request.mode.is_some_and(|m| m != mode) {
            return Err("a request has one mode".to_string());
        }
        request.mode = Some(mode);
        if mode.starts_with("stdin") {
            if request.stdin.is_some() {
                return Err("a request has one standard input".to_string());
            }
            request.stdin = Some(frame.value);
        } else {
            request.sources.push(text(&frame)?);
        }
    }
    if request.admin.is_none() && request.mode.is_none() {
        return Err("the request names nothing to do".to_string());
    }
    Ok(request)
}

/// `VmRSS` and `VmHWM` of this process, as `/proc/self/status` gives them (Linux).
fn memory_lines() -> String {
    fs::read_to_string("/proc/self/status")
        .map(|status| {
            status
                .lines()
                .filter(|l| l.starts_with("VmRSS:") || l.starts_with("VmHWM:"))
                .map(|l| format!("{}\n", l.split_whitespace().collect::<Vec<_>>().join(" ")))
                .collect()
        })
        .unwrap_or_default()
}

impl State {
    /// Whether the file of the server's executable is no longer the one it started from.
    fn executable_replaced(&self) -> bool {
        match &self.exe_path {
            Some(path) => cache::identity_at(path).as_deref() != Some(&self.identity[..]),
            None => false,
        }
    }

    /// Writes every cache a request changed since the last write. Errors are ignored: a
    /// cache that cannot be written costs a later process a format.
    fn save(&mut self) {
        for state in self.configs.values_mut().flatten() {
            if state.dirty {
                let _ = state.units.write_kept(state.cache.executable_unchanged());
                let _ = state.cache.write();
                state.dirty = false;
            }
        }
    }

    fn dirty(&self) -> bool {
        self.configs.values().flatten().any(|s| s.dirty)
    }

    /// Removes the socket if it is still the one this server bound.
    fn remove_socket(&self) {
        let ours = fs::symlink_metadata(&self.socket)
            .ok()
            .is_some_and(|m| Some(m.ino()) == self.socket_ino);
        if ours {
            let _ = fs::remove_file(&self.socket);
        }
    }

    fn status(&self) -> Vec<u8> {
        let mut out = format!(
            "pid {}\nrequests {}\nbusy-us {}\nthreads {}\nitem-threads {}\nconfigs {}\npaths {}\n",
            std::process::id(),
            self.requests,
            self.busy.as_micros(),
            self.threads,
            self.item_threads,
            self.configs.len(),
            self.stats.len()
        );
        for state in self.configs.values().flatten() {
            let units = state.units.stats().unwrap_or_default();
            out.push_str(&format!(
                "cache {} entries {} units {} unit-bytes {} unit-lookups {} unit-hits {} unit-stores {}\n",
                state.cache.path().display(),
                state.cache.len(),
                units.entries,
                state.units.held_bytes(),
                units.lookups,
                units.hits,
                units.stores
            ));
        }
        out.push_str(&memory_lines());
        out.into_bytes()
    }

    /// Answers one request.
    fn answer(&mut self, request: Request) -> Reply {
        match request.admin {
            Some("ping") => {
                let mut reply = Reply::ok(
                    0,
                    format!("{}\n", std::process::id()).into_bytes(),
                    Vec::new(),
                );
                reply.identity = Some(self.identity.clone());
                return reply;
            }
            Some("status") => return Reply::ok(0, self.status(), Vec::new()),
            Some(_) => {
                let mut reply = Reply::ok(0, Vec::new(), Vec::new());
                reply.then_exit = true;
                return reply;
            }
            None => {}
        }
        if request.exe != self.identity {
            let mut reply = Reply::fallback("the server runs another chloro executable");
            reply.then_exit = self.executable_replaced();
            return reply;
        }
        if !request.no_cache && request.cache_dir != self.dir {
            return Reply::fallback("the server keeps another cache directory");
        }
        let mode = request.mode.unwrap_or_default();
        let args = Args {
            sources: request.sources,
            check: mode == "check" || mode == "stdin-check",
            write: mode == "write",
            verbose: false,
            config: request.config,
            no_cache: request.no_cache,
            unit_cache: request.unit_cache,
            cache_dir: String::new(),
            threads: if request.threads > 0 {
                request.threads
            } else if request.stdin.is_some() {
                self.item_threads
            } else {
                self.threads
            },
            help: false,
            stdin: request.stdin.is_some(),
            via_server: false,
            socket: String::new(),
            idle_timeout: 0,
        };
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let single = if request.threads > 0 {
            request.threads
        } else {
            self.item_threads
        };
        match self.run(
            &args,
            single,
            &request.cwd,
            request.stdin,
            &mut stdout,
            &mut stderr,
        ) {
            Ok(code) => Reply::ok(code, stdout, stderr),
            // As `main` returning the error: the runtime prints it and exits with 1.
            Err(e) => {
                ewriteln!(stderr, "Error: {e:?}");
                Reply::ok(1, stdout, stderr)
            }
        }
    }

    /// The CLI's run (`cli::run`) against the server's caches; a single file's items are
    /// formatted on `single` threads when it is large enough.
    fn run(
        &mut self,
        args: &Args,
        single: usize,
        cwd: &Path,
        stdin: Option<Vec<u8>>,
        out: &mut Vec<u8>,
        err: &mut Vec<u8>,
    ) -> io::Result<i32> {
        let config = match args.formatting_config() {
            Ok(config) => config,
            Err(e) => {
                ewriteln!(err, "Error: invalid --config: {e}");
                return Ok(1);
            }
        };
        if let Some(message) = invalid_combination(args) {
            ewriteln!(err, "Error: {message}");
            return Ok(1);
        }
        let stats = &self.stats;
        // The caches of this configuration, opened on its first request.
        let mut state = if args.no_cache {
            None
        } else {
            let dir = self.dir.as_deref();
            let configs = &mut self.configs;
            dir.and_then(|dir| {
                configs
                    .entry(config.key_bytes())
                    .or_insert_with(|| {
                        Cache::open(dir, &config).map(|cache| ConfigState {
                            units: Units::new(&cache),
                            cache,
                            dirty: false,
                        })
                    })
                    .as_mut()
            })
        };
        if args.unit_cache
            && let Some(state) = &state
        {
            state.units.next_generation();
        }
        let caches = caches_of(state.as_deref(), args.unit_cache, stats);

        let agg = if let Some(bytes) = stdin {
            run_stdin(Ok(bytes), args, caches, out)
        } else {
            let files = match collect_files(args, Some(cwd), err)? {
                Ok(files) => files,
                Err(code) => return Ok(code),
            };
            if files.is_empty() {
                return Ok(0);
            }
            let results = match &files[..] {
                // One file (an editor's save) on the request's thread: no worker thread to
                // start. A panic here is caught by the request, which falls back, and the
                // client's run reports it as the CLI would.
                [file] => {
                    let threads = if file.len >= PARALLEL_ITEMS_MIN_BYTES as u64 {
                        single
                    } else {
                        1
                    };
                    vec![format_file(&file.path, args, caches, threads)]
                }
                _ => format_all(&files, args, caches),
            };
            aggregate_results(results)
        };

        if let Some(state) = &mut state {
            state.cache.record(agg.fixed_points.iter().copied());
            state.cache.bound();
            state.units.trim(units::MAX_BYTES);
            state.dirty = true;
        }
        Ok(finish(&agg, args, err))
    }
}

/// The caches a request uses: none without a configuration state (`--no-cache`, or no
/// cache directory), else the fixed-point cache, the stamps and, with `--unit-cache`, the
/// unit cache.
fn caches_of<'a>(
    state: Option<&'a ConfigState>,
    unit_cache: bool,
    stats: &'a StatTable,
) -> Caches<'a> {
    match state {
        Some(state) => Caches {
            fixed: Some(&state.cache),
            units: unit_cache.then_some(&state.units),
            stats: Some(stats),
        },
        None => Caches::default(),
    }
}

/// Locks the state, whatever a panicking request left in it.
fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sends a `ping` to the socket at `path`: whether a server answers.
pub fn ping(path: &Path) -> bool {
    let Ok(stream) = UnixStream::connect(path) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let request = [
        Frame::new("chloro", protocol::VERSION),
        Frame::new("ping", ""),
    ];
    protocol::write_message(&mut BufWriter::new(&stream), &request).is_ok()
        && protocol::read_message(&mut BufReader::new(&stream)).is_ok()
}

/// Binds the socket at `path`, replacing a socket no server answers on. `Ok(None)` when a
/// server answers on it.
fn bind(path: &Path) -> io::Result<Option<UnixListener>> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    match UnixListener::bind(path) {
        Ok(listener) => Ok(Some(listener)),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            if ping(path) {
                return Ok(None);
            }
            // Left by a server that was killed. Two servers starting at once can both get
            // here; the second to bind fails, and clients reach the first.
            fs::remove_file(path)?;
            UnixListener::bind(path).map(Some)
        }
        Err(e) => Err(e),
    }
}

/// Reads one request from `stream` and answers it.
fn serve_connection(stream: &UnixStream, state: &Mutex<State>) -> bool {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let reply = match protocol::read_message(&mut BufReader::new(stream)) {
        Err(e) => {
            let mut reply = Reply::fallback(format!("unreadable request: {e}"));
            reply.status = "error";
            reply
        }
        Ok(frames) => match parse_request(frames) {
            Err(reason) => Reply::fallback(reason),
            Ok(request) => {
                let mut state = lock(state);
                let start = Instant::now();
                state.requests += 1;
                let answered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    state.answer(request)
                }));
                let end = Instant::now();
                state.last_request = end;
                state.busy += end - start;
                answered.unwrap_or_else(|_| Reply::fallback("the request panicked in the server"))
            }
        },
    };
    let _ = protocol::write_message(&mut BufWriter::new(stream), &reply.frames());
    reply.then_exit
}

/// `chloro serve`: the exit code.
///
/// # Errors
///
/// The error binding the socket.
pub fn main(args: &Args) -> io::Result<i32> {
    let mut err = io::stderr();
    let refused = [
        (args.check, "--check"),
        (args.write, "--write"),
        (args.stdin, "--stdin"),
        (args.via_server, "--via-server"),
        (args.no_cache, "--no-cache"),
        (args.unit_cache, "--unit-cache"),
        (!args.config.is_empty(), "--config"),
    ];
    if let Some((_, option)) = refused.iter().find(|(given, _)| *given) {
        ewriteln!(err, "Error: `chloro serve` does not take {option}");
        return Ok(1);
    }
    if args.sources.len() > 1 {
        ewriteln!(err, "Error: `chloro serve` takes no paths");
        return Ok(1);
    }
    let Some((identity, exe_path)) = cache::executable_identity() else {
        ewriteln!(err, "Error: cannot identify the running chloro executable");
        return Ok(1);
    };
    let dir = cache::resolve_dir(&args.cache_dir);
    let Some(socket) = socket_path(args) else {
        ewriteln!(
            err,
            "Error: no socket path (set --socket, --cache-dir or CHLORO_CACHE_DIR)"
        );
        return Ok(1);
    };
    let listener = match bind(&socket) {
        Ok(Some(listener)) => listener,
        Ok(None) => {
            ewriteln!(
                err,
                "Error: a server already answers on {}",
                socket.display()
            );
            return Ok(1);
        }
        Err(e) => {
            ewriteln!(err, "Error: cannot listen on {}: {e}", socket.display());
            return Ok(1);
        }
    };
    // Only this user may connect.
    let _ = fs::set_permissions(&socket, fs::Permissions::from_mode(0o600));
    let socket_ino = fs::symlink_metadata(&socket).ok().map(|m| m.ino());

    let state = Arc::new(Mutex::new(State {
        dir,
        identity,
        exe_path,
        threads: args.thread_count(),
        item_threads: args.threads.max(1),
        configs: HashMap::new(),
        stats: StatTable::default(),
        last_request: Instant::now(),
        requests: 0,
        busy: Duration::ZERO,
        socket: socket.clone(),
        socket_ino,
    }));
    if args.verbose {
        eprintln!(
            "chloro serve: pid {}, socket {}, idle timeout {} s",
            std::process::id(),
            socket.display(),
            args.idle_timeout
        );
    }

    // Idle writes and exits, and the executable check, between requests.
    let idle_timeout = Duration::from_secs(args.idle_timeout);
    let ticker = Arc::clone(&state);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            let mut state = lock(&ticker);
            if state.executable_replaced() {
                // The caches belong to the old executable; its writes would be refused.
                state.remove_socket();
                std::process::exit(0);
            }
            let idle = state.last_request.elapsed();
            if state.dirty() && idle >= SAVE_DELAY {
                state.save();
            }
            if !idle_timeout.is_zero() && idle >= idle_timeout {
                state.save();
                state.remove_socket();
                std::process::exit(0);
            }
        }
    });

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if serve_connection(&stream, &state) {
            drop(stream);
            let mut state = lock(&state);
            if !state.executable_replaced() {
                state.save();
            }
            state.remove_socket();
            return Ok(0);
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(frames: &[(&str, &[u8])]) -> Result<Request, String> {
        let mut all = vec![Frame::new("chloro", protocol::VERSION)];
        all.extend(frames.iter().map(|(n, v)| Frame::new(n, *v)));
        parse_request(all)
    }

    #[test]
    fn a_request_is_parsed() {
        let r = request(&[
            ("exe", b"id"),
            ("cwd", b"/tmp/x"),
            ("cache-dir", b"/tmp/c"),
            ("config", b"max_width=80"),
            ("threads", b"3"),
            ("flag", b"unit-cache"),
            ("check", b"a.rs"),
            ("check", b"src"),
        ])
        .unwrap();
        assert_eq!(r.exe, b"id");
        assert_eq!(r.cwd, Path::new("/tmp/x"));
        assert_eq!(r.cache_dir.as_deref(), Some(Path::new("/tmp/c")));
        assert_eq!((r.config.as_str(), r.threads), ("max_width=80", 3));
        assert!(r.unit_cache && !r.no_cache);
        assert_eq!((r.mode, r.sources.len()), (Some("check"), 2));

        let r = request(&[("stdin-check", b"fn a(){}"), ("cache-dir", b"")]).unwrap();
        assert_eq!(r.stdin.as_deref(), Some(&b"fn a(){}"[..]));
        assert_eq!(r.cache_dir, None);
        assert_eq!(request(&[("ping", b"")]).unwrap().admin, Some("ping"));
    }

    #[test]
    fn malformed_requests_fall_back() {
        assert!(parse_request(vec![Frame::new("check", "a.rs")]).is_err());
        assert!(parse_request(vec![Frame::new("chloro", "2"), Frame::new("ping", "")]).is_err());
        assert!(request(&[]).is_err());
        assert!(request(&[("check", b"a.rs"), ("write", b"b.rs")]).is_err());
        assert!(request(&[("stdin", b"a"), ("stdin", b"b")]).is_err());
        assert!(request(&[("ping", b""), ("check", b"a.rs")]).is_err());
        assert!(request(&[("check", b"a.rs"), ("stop", b"")]).is_err());
        assert!(request(&[("flag", b"verbose"), ("check", b"a.rs")]).is_err());
        assert!(request(&[("threads", b"x"), ("check", b"a.rs")]).is_err());
        assert!(request(&[("print", b"a.rs")]).is_err());
    }
}
