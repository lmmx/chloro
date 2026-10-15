//! End-to-end tests of `chloro serve`: a server on a temporary socket answers `--via-server`
//! runs with the output, exit code and file contents of the same runs in the client
//! process, and its lifecycle (refused clients, `stop`, a replaced executable) holds.
#![cfg(all(feature = "cli", unix))]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_chloro");
const UNFORMATTED: &str = "fn main(){let x=1;}\n";
const FORMATTED: &str = "fn main() {\n    let x = 1;\n}\n";

/// Spawns `command`, retrying while the executable is busy (`ETXTBSY`): a test that has
/// just copied the binary can lose a race with another test's fork, whose child holds the
/// copy open for writing until it executes.
fn spawn(command: &mut Command) -> Child {
    let start = Instant::now();
    loop {
        match command.spawn() {
            Err(e) if e.raw_os_error() == Some(26) && start.elapsed() < Duration::from_secs(10) => {
                std::thread::sleep(Duration::from_millis(20));
            }
            result => return result.unwrap(),
        }
    }
}

/// A server on `<dir>/s.sock` with its cache under `<dir>/server-cache`.
struct Server {
    child: Child,
    socket: PathBuf,
}

impl Server {
    fn start(bin: &Path, dir: &Path) -> Server {
        let socket = dir.join("s.sock");
        let child = spawn(
            Command::new(bin)
                .args(["serve", "--idle-timeout", "120", "--socket"])
                .arg(&socket)
                .env("CHLORO_CACHE_DIR", dir.join("server-cache"))
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        );
        let server = Server { child, socket };
        let start = Instant::now();
        while frames(&server.socket, &[("ping", b"")]).is_none() {
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "the server did not start"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        server
    }

    /// The `requests` count of the server's `status` reply.
    fn requests(&self) -> u64 {
        self.status_count("requests ")
    }

    /// A count of the server's `status` reply, by its line's prefix.
    fn status_count(&self, prefix: &str) -> u64 {
        let reply = frames(&self.socket, &[("status", b"")]).unwrap();
        let text = String::from_utf8(value(&reply, "stdout").to_vec()).unwrap();
        text.lines()
            .find_map(|l| l.strip_prefix(prefix))
            .unwrap()
            .parse()
            .unwrap()
    }

    /// Waits for the server process to exit, up to `limit`.
    fn exits_within(&mut self, limit: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            if self.child.try_wait().unwrap().is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Sends `chloro 1` and `request` in the protocol's framing; the reply's frames.
fn frames(socket: &Path, request: &[(&str, &[u8])]) -> Option<Vec<(String, Vec<u8>)>> {
    let mut stream = UnixStream::connect(socket).ok()?;
    let mut bytes = b"chloro 1\n1".to_vec();
    for (name, v) in request {
        bytes.extend_from_slice(format!("{name} {}\n", v.len()).as_bytes());
        bytes.extend_from_slice(v);
    }
    bytes.extend_from_slice(b"end 0\n");
    stream.write_all(&bytes).ok()?;
    let mut reader = BufReader::new(stream);
    let mut reply = Vec::new();
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let (name, len) = header.trim_end().split_once(' ')?;
        if name == "end" {
            return Some(reply);
        }
        let mut v = vec![0; len.parse().ok()?];
        reader.read_exact(&mut v).ok()?;
        reply.push((name.to_string(), v));
    }
}

fn value<'a>(reply: &'a [(String, Vec<u8>)], name: &str) -> &'a [u8] {
    &reply.iter().find(|(n, _)| n == name).unwrap().1
}

/// What a run printed and returned, and the files it left.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    files: Vec<(String, String)>,
}

/// Runs `bin args` in `dir` (cache under `cache`), with `stdin` piped in.
fn run(bin: &Path, dir: &Path, cache: &Path, args: &[&str], stdin: &[u8]) -> Run {
    let mut child = spawn(
        Command::new(bin)
            .args(args)
            .current_dir(dir)
            .env("CHLORO_CACHE_DIR", cache)
            .env_remove("CHLORO_VIA_SERVER")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    );
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().unwrap();
    let mut files: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .map(|e| {
            let text = String::from_utf8_lossy(&fs::read(e.path()).unwrap()).into_owned();
            (e.file_name().to_string_lossy().into_owned(), text)
        })
        .collect();
    files.sort();
    Run {
        code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        // Paths differ between the two copies; error lines name canonical paths.
        stderr: String::from_utf8_lossy(&stderr)
            .replace(&*dir.canonicalize().unwrap().to_string_lossy(), "<dir>"),
        files,
    }
}

/// Two copies of a small project: `local` for runs in the client process, `served` for runs
/// through the server.
struct Pair {
    root: TempDir,
}

impl Pair {
    fn new(files: &[(&str, &[u8])]) -> Pair {
        let root = tempfile::tempdir().unwrap();
        for side in ["local", "served"] {
            fs::create_dir_all(root.path().join(side)).unwrap();
            for (name, text) in files {
                fs::write(root.path().join(side).join(name), text).unwrap();
            }
        }
        Pair { root }
    }

    fn side(&self, side: &str) -> PathBuf {
        self.root.path().join(side)
    }

    /// The run of `args` in the client process and through the server at `socket`; the
    /// local run has its own cache directory, so the two share nothing.
    fn both(&self, socket: &Path, server_cache: &Path, args: &[&str], stdin: &[u8]) -> (Run, Run) {
        let local = run(
            Path::new(BIN),
            &self.side("local"),
            &self.root.path().join("local-cache"),
            args,
            stdin,
        );
        let mut via = vec!["--via-server", "--socket", socket.to_str().unwrap()];
        via.extend_from_slice(args);
        let served = run(
            Path::new(BIN),
            &self.side("served"),
            server_cache,
            &via,
            stdin,
        );
        (local, served)
    }
}

#[test]
fn runs_through_the_server_equal_runs_in_the_client() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(Path::new(BIN), dir.path());
    let cache = dir.path().join("server-cache");
    let large = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../chloro-core/tests/conformance/fixtures/rust-analyzer/rust-analyzer/src/config.rs",
    ))
    .unwrap();
    let cases: &[(&[&str], &[u8])] = &[
        (&["--check", "good.rs"], b""),
        (&["--check", "bad.rs"], b""),
        (&["--check", "."], b""),
        (&["--check", "good.rs", "bad.rs", "./good.rs"], b""),
        (&["--check", "nothing.rs"], b""),
        (&["--check", "latin1.rs"], b""),
        (&["--check", "--config", "tab_spaces=2", "good.rs"], b""),
        (&["--check", "--config", "max_width=x", "good.rs"], b""),
        (&["--check", "--config", "nonsense=1", "good.rs"], b""),
        (&["--check", "--no-cache", "bad.rs"], b""),
        (&["--check", "--unit-cache", "large.rs"], b""),
        (&["--check", "--threads", "2", "large.rs"], b""),
        (&["--stdin"], UNFORMATTED.as_bytes()),
        (
            &["--stdin", "--config", "tab_spaces=2"],
            UNFORMATTED.as_bytes(),
        ),
        (&["--stdin", "--check"], UNFORMATTED.as_bytes()),
        (&["--stdin", "--check"], FORMATTED.as_bytes()),
        (&["--stdin", "--unit-cache"], &large),
        (&["--stdin"], b"\xff\xfe"),
        (&["--stdin", "--write"], UNFORMATTED.as_bytes()),
        (&["--write", "bad.rs"], b""),
        (&["--write", "."], b""),
        (&["--check", "."], b""),
    ];
    let pair = Pair::new(&[
        ("good.rs", FORMATTED.as_bytes()),
        ("bad.rs", UNFORMATTED.as_bytes()),
        ("latin1.rs", b"fn caf\xe9() {}\n"),
        ("large.rs", &large),
    ]);
    let before = server.requests();
    for (args, stdin) in cases {
        let (local, served) = pair.both(&server.socket, &cache, args, stdin);
        assert_eq!(local, served, "{args:?}");
    }
    // Every run went through the server (each run counts one request, `status` one), but
    // `--stdin --write`, which the client refuses itself.
    let forwarded = cases
        .iter()
        .filter(|(args, _)| !(args.contains(&"--stdin") && args.contains(&"--write")))
        .count();
    assert_eq!(server.requests(), before + forwarded as u64 + 1);

    // A one-item edit of the large file, through the unit cache the server holds.
    let edited = String::from_utf8(large.clone())
        .unwrap()
        .replacen("\"", "\"z", 1);
    for side in ["local", "served"] {
        fs::write(pair.side(side).join("large.rs"), &edited).unwrap();
    }
    for args in [
        &["--check", "--unit-cache", "large.rs"][..],
        &["--write", "--unit-cache", "."],
    ] {
        let (local, served) = pair.both(&server.socket, &cache, args, b"");
        assert_eq!(local, served, "{args:?}");
    }
}

#[test]
fn relative_paths_resolve_in_the_client_directory() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(Path::new(BIN), dir.path());
    let pair = Pair::new(&[("bad.rs", UNFORMATTED.as_bytes())]);
    for side in ["local", "served"] {
        fs::create_dir_all(pair.side(side).join("sub")).unwrap();
        fs::write(pair.side(side).join("sub/inner.rs"), UNFORMATTED).unwrap();
    }
    let cache = dir.path().join("server-cache");
    for args in [
        &["--check", "sub"][..],
        &["--write", "sub/../bad.rs", "sub"],
    ] {
        let (local, served) = pair.both(&server.socket, &cache, args, b"");
        assert_eq!(local, served, "{args:?}");
    }
    assert_eq!(
        fs::read_to_string(pair.side("served").join("sub/inner.rs")).unwrap(),
        FORMATTED
    );
}

#[test]
fn without_a_server_the_client_runs_the_request() {
    let dir = tempfile::tempdir().unwrap();
    let pair = Pair::new(&[("bad.rs", UNFORMATTED.as_bytes())]);
    let missing = dir.path().join("none.sock");
    let (local, served) = pair.both(&missing, &dir.path().join("c"), &["--check", "bad.rs"], b"");
    assert_eq!(local, served);
    assert_eq!(served.code, Some(1));
    // The environment variable asks for the server as the option does.
    let out = Command::new(BIN)
        .args(["--check", "--socket"])
        .arg(&missing)
        .arg(pair.side("local").join("bad.rs"))
        .env("CHLORO_VIA_SERVER", "1")
        .env("CHLORO_CACHE_DIR", dir.path().join("c"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn another_executable_is_refused_and_the_client_runs_the_request() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(Path::new(BIN), dir.path());
    // A copy is another executable (path and inode).
    let copy = dir.path().join("chloro-copy");
    fs::copy(BIN, &copy).unwrap();
    let pair = Pair::new(&[("bad.rs", UNFORMATTED.as_bytes())]);
    let before = server.requests();
    let via = [
        "--via-server",
        "--socket",
        server.socket.to_str().unwrap(),
        "--write",
        "bad.rs",
    ];
    let served = run(
        &copy,
        &pair.side("served"),
        &dir.path().join("c"),
        &via,
        b"",
    );
    let local = run(
        Path::new(BIN),
        &pair.side("local"),
        &dir.path().join("c2"),
        &via[3..],
        b"",
    );
    assert_eq!(local, served);
    assert_eq!(
        served.files,
        vec![("bad.rs".to_string(), FORMATTED.to_string())]
    );
    // The server saw the request, refused it, and keeps running for its own executable.
    assert_eq!(server.requests(), before + 2);
}

#[test]
fn stop_and_a_replaced_executable_end_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let mut server = Server::start(Path::new(BIN), dir.path());
    let reply = frames(&server.socket, &[("stop", b"")]).unwrap();
    assert_eq!(value(&reply, "status"), b"ok");
    assert!(server.exits_within(Duration::from_secs(10)));
    assert!(!server.socket.exists());

    // A server started from a copy exits when the copy is rewritten.
    let copy = dir.path().join("chloro-copy");
    fs::copy(BIN, &copy).unwrap();
    let mut server = Server::start(&copy, dir.path());
    // A second server on the same socket refuses to start.
    let second = spawn(
        Command::new(&copy)
            .args(["serve", "--socket"])
            .arg(&server.socket)
            .env("CHLORO_CACHE_DIR", dir.path().join("server-cache"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .wait_with_output()
    .unwrap();
    assert_eq!(second.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&second.stderr).contains("already answers"));
    let mut bytes = fs::read(&copy).unwrap();
    bytes.push(0);
    // Written to a new file renamed over the copy, as an install replaces a binary.
    let replacement = dir.path().join("chloro-copy.new");
    fs::write(&replacement, &bytes).unwrap();
    fs::rename(&replacement, &copy).unwrap();
    assert!(server.exits_within(Duration::from_secs(10)));
    assert!(!server.socket.exists());
}

#[test]
fn a_stale_socket_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let mut server = Server::start(Path::new(BIN), dir.path());
    // Killed: the socket stays, nobody answers on it.
    server.child.kill().unwrap();
    server.child.wait().unwrap();
    assert!(server.socket.exists());
    let pair = Pair::new(&[("bad.rs", UNFORMATTED.as_bytes())]);
    let (local, served) = pair.both(
        &server.socket,
        &dir.path().join("c"),
        &["--check", "."],
        b"",
    );
    assert_eq!(local, served);
    // A new server takes the socket over.
    let restarted = Server::start(Path::new(BIN), dir.path());
    assert!(restarted.requests() >= 1);
}

#[test]
fn renames_configuration_changes_and_rewrites_under_a_known_stamp_are_seen() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(Path::new(BIN), dir.path());
    let cache = dir.path().join("server-cache");
    let pair = Pair::new(&[("good.rs", FORMATTED.as_bytes())]);
    // Older than the settle time: the server records the stamp and answers the next
    // request on the file from it, without reading the file.
    std::thread::sleep(Duration::from_millis(1200));
    for _ in 0..2 {
        let (local, served) = pair.both(&server.socket, &cache, &["--check", "good.rs"], b"");
        assert_eq!(local, served);
        assert_eq!(served.code, Some(0));
    }
    // The settled stamp is recorded, so a request that finds the file unchanged takes it.
    assert_eq!(server.status_count("paths "), 1);
    // The same size and modification time with other content: the status-change time
    // differs, so the server reads the file again.
    let same_size = format!("{:<28}\n", "fn main(){let x=1;}");
    assert_eq!(same_size.len(), FORMATTED.len());
    for side in ["local", "served"] {
        let path = pair.side(side).join("good.rs");
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, &same_size).unwrap();
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
    }
    let (local, served) = pair.both(&server.socket, &cache, &["--check", "good.rs"], b"");
    assert_eq!(local, served);
    assert_eq!(served.code, Some(1));

    // A rename: the formatted content under a new name, other content under the old one.
    for side in ["local", "served"] {
        let side = pair.side(side);
        fs::write(side.join("good.rs"), FORMATTED).unwrap();
        fs::rename(side.join("good.rs"), side.join("moved.rs")).unwrap();
        fs::write(side.join("good.rs"), UNFORMATTED).unwrap();
    }
    for args in [
        &["--check", "moved.rs"][..],
        &["--check", "good.rs"],
        &["--check", "."],
    ] {
        let (local, served) = pair.both(&server.socket, &cache, args, b"");
        assert_eq!(local, served, "{args:?}");
    }

    // A configuration change: the same formatted file under another configuration.
    for args in [
        &["--check", "moved.rs"][..],
        &["--check", "--config", "tab_spaces=2", "moved.rs"],
        &[
            "--check",
            "--config",
            "tab_spaces=2,max_width=60",
            "moved.rs",
        ],
        &["--check", "moved.rs"],
    ] {
        let (local, served) = pair.both(&server.socket, &cache, args, b"");
        assert_eq!(local, served, "{args:?}");
    }
}
