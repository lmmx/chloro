//! Measurement and identity driver for `chloro serve` (experiment E21,
//! `docs/journal/2026-10-14-autoresearch/experiments/EXP-021-resident.md`).
//!
//! ```text
//! serve_driver bench --file F --mode socket [--flag unit-cache] [--n 1000] [--rss 10,100,1000]
//! serve_driver bench --file F --mode exec [--n 1000] -- chloro --check --unit-cache
//! serve_driver identity --chloro BIN [--root DIR]... [--every N] [--no-blank] [--no-paths]
//! ```
//!
//! `bench` performs `--n` saves of `F`: each writes `F` with a one-token edit (three letters
//! of the last string literal that starts with three lowercase ASCII letters replaced by a
//! code of the save's number, so every save is new content and the file stays formatted;
//! `--edit dirty` also appends a newline the formatter removes), times one request, and
//! writes the original back. `--mode socket` sends the request on the socket
//! (`--socket`, default `$CHLORO_CACHE_DIR/chloro.sock`) itself, as an editor integration
//! would; `--mode exec` runs the command after `--` with `F` appended (with `--pipe`, the
//! file's content on standard input instead, as editors call rustfmt). One untimed request
//! on the unedited file comes first. With `--rss`, the server's `VmRSS` and `VmHWM` are read
//! from `/proc/<pid>/status` after the given saves. Prints one summary line (milliseconds)
//! and, with `--tsv`, writes every latency.
//!
//! `identity` drives the scenarios of `chloro-core/examples/unit_cache.rs` through the
//! socket with the unit cache on and compares every reply (exit code, standard output,
//! standard error) with `BIN --stdin --no-cache` run as a process on the same input: for
//! every file, the file (cold, then warm), the file with a comment line inserted at its
//! start (shifted), `--check` of the file, and for every unit the file with every byte
//! outside the unit blanked. Then, on two copies of the tree, `--check` and `--write` of
//! every file and of the tree through the socket against the CLI on the other copy, and the
//! files written. Exits with status 1 on any difference or fallback.

#[cfg(unix)]
#[allow(dead_code)]
#[path = "../src/cli/protocol.rs"]
mod protocol;

#[cfg(not(unix))]
fn main() {
    eprintln!("serve_driver needs a Unix platform");
    std::process::exit(1);
}

#[cfg(unix)]
fn main() {
    unix::main();
}

#[cfg(unix)]
mod unix {
    use super::protocol::{self, Frame};
    use chloro_core::{CachedUnit, Config, UnitCache, UnitKey};
    use std::cell::RefCell;
    use std::fs;
    use std::io::{BufReader, BufWriter, Write};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::Instant;

    /// What a run returned: exit code, standard output, standard error.
    type Outcome = (i32, Vec<u8>, Vec<u8>);

    struct Client {
        socket: PathBuf,
        identity: Vec<u8>,
        pid: u32,
        cwd: PathBuf,
        cache_dir: Vec<u8>,
    }

    impl Client {
        fn connect(socket: PathBuf) -> Client {
            let reply = exchange(&socket, &[Frame::new("ping", "")]).expect("no server answers");
            let get = |name: &str| {
                reply
                    .iter()
                    .find(|f| f.name == name)
                    .map(|f| f.value.clone())
            };
            let pid = String::from_utf8(get("stdout").unwrap()).unwrap();
            Client {
                socket,
                identity: get("exe").expect("the ping reply carries no identity"),
                pid: pid.trim().parse().unwrap(),
                cwd: std::env::current_dir().unwrap(),
                cache_dir: std::env::var_os("CHLORO_CACHE_DIR")
                    .map(|d| d.as_bytes().to_vec())
                    .unwrap_or_default(),
            }
        }

        /// The reply to a request of `sources` (frames such as `check PATH`) with `flags`.
        fn request(&self, flags: &[&str], sources: Vec<Frame>) -> Result<Outcome, String> {
            let mut frames = vec![
                Frame::new("exe", self.identity.clone()),
                Frame::new("cwd", self.cwd.as_os_str().as_bytes()),
                Frame::new("cache-dir", self.cache_dir.clone()),
                Frame::new("config", ""),
                Frame::new("threads", "0"),
            ];
            frames.extend(flags.iter().map(|f| Frame::new("flag", *f)));
            frames.extend(sources);
            let reply = exchange(&self.socket, &frames).ok_or("the exchange failed")?;
            let get = |name: &str| {
                reply
                    .iter()
                    .find(|f| f.name == name)
                    .map(|f| f.value.clone())
            };
            if get("status").as_deref() != Some(b"ok") {
                return Err(format!(
                    "status {:?}: {}",
                    get("status").map(|s| String::from_utf8_lossy(&s).into_owned()),
                    String::from_utf8_lossy(&get("reason").unwrap_or_default())
                ));
            }
            let exit = String::from_utf8(get("exit").unwrap()).unwrap();
            Ok((
                exit.parse().unwrap(),
                get("stdout").unwrap_or_default(),
                get("stderr").unwrap_or_default(),
            ))
        }

        fn status(&self) -> String {
            let reply = exchange(&self.socket, &[Frame::new("status", "")]).unwrap();
            let out = reply.into_iter().find(|f| f.name == "stdout").unwrap();
            String::from_utf8(out.value).unwrap()
        }
    }

    /// Sends `chloro 1` and `frames`; the reply's frames.
    fn exchange(socket: &Path, frames: &[Frame]) -> Option<Vec<Frame>> {
        let stream = UnixStream::connect(socket).ok()?;
        let mut all = vec![Frame::new("chloro", protocol::VERSION)];
        all.extend_from_slice(frames);
        protocol::write_message(&mut BufWriter::new(&stream), &all).ok()?;
        protocol::read_message(&mut BufReader::new(&stream)).ok()
    }

    /// Runs `argv` with `stdin` piped in.
    fn process(argv: &[String], stdin: Option<&[u8]>) -> Outcome {
        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            });
        let mut child = command.spawn().unwrap();
        if let Some(bytes) = stdin {
            let mut pipe = child.stdin.take().unwrap();
            let bytes = bytes.to_vec();
            // Written from a thread: a large output could otherwise fill its pipe first.
            std::thread::spawn(move || {
                let _ = pipe.write_all(&bytes);
            });
        }
        let out = child.wait_with_output().unwrap();
        (out.status.code().unwrap_or(-1), out.stdout, out.stderr)
    }

    fn loadavg() -> String {
        fs::read_to_string("/proc/loadavg")
            .map(|l| l.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    }

    fn memory(pid: u32) -> (u64, u64) {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
        let field = |name: &str| {
            status
                .lines()
                .find_map(|l| l.strip_prefix(name))
                .and_then(|v| v.split_whitespace().next()?.parse().ok())
                .unwrap_or(0)
        };
        (field("VmRSS:"), field("VmHWM:"))
    }

    /// Where the edit goes: the first of three lowercase ASCII letters after the last `"`
    /// that is followed by three.
    fn edit_site(source: &str) -> usize {
        let bytes = source.as_bytes();
        source
            .match_indices('"')
            .map(|(i, _)| i + 1)
            .rfind(|&i| (i..i + 3).all(|j| bytes.get(j).is_some_and(u8::is_ascii_lowercase)))
            .expect("no string literal starting with three lowercase letters")
    }

    /// Three letters for save `n`, never the original's.
    fn code(n: usize, original: &[u8]) -> [u8; 3] {
        let letters = |n: usize| {
            [
                b'a' + (n / 676 % 26) as u8,
                b'a' + (n / 26 % 26) as u8,
                b'a' + (n % 26) as u8,
            ]
        };
        let code = letters(n);
        if code[..] == original[..3] {
            letters(n + 17_576 / 2)
        } else {
            code
        }
    }

    fn percentile(sorted: &[f64], q: f64) -> f64 {
        let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
        sorted[rank - 1]
    }

    pub fn main() {
        let mut args: Vec<String> = std::env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            Some("bench") => bench(args.split_off(1)),
            Some("identity") => identity(args.split_off(1)),
            _ => {
                eprintln!("usage: serve_driver bench|identity ...");
                std::process::exit(2);
            }
        }
    }

    fn default_socket() -> PathBuf {
        PathBuf::from(std::env::var_os("CHLORO_CACHE_DIR").expect("set CHLORO_CACHE_DIR"))
            .join("chloro.sock")
    }

    fn bench(args: Vec<String>) {
        let mut file = None;
        let mut mode = String::from("socket");
        let mut n = 1000;
        let mut socket = None;
        let mut flags: Vec<String> = Vec::new();
        let mut request = String::from("check");
        let mut dirty = false;
        let mut rss_at: Vec<usize> = Vec::new();
        let mut label = String::new();
        let mut tsv = None;
        let mut pipe = false;
        let mut exec: Vec<String> = Vec::new();
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            let mut value = || it.next().expect("option takes a value");
            match arg.as_str() {
                "--file" => file = Some(PathBuf::from(value())),
                "--mode" => mode = value(),
                "--n" => n = value().parse().unwrap(),
                "--socket" => socket = Some(PathBuf::from(value())),
                "--flag" => flags.push(value()),
                "--request" => request = value(),
                "--edit" => dirty = value() == "dirty",
                "--rss" => rss_at = value().split(',').map(|v| v.parse().unwrap()).collect(),
                "--label" => label = value(),
                "--tsv" => tsv = Some(PathBuf::from(value())),
                "--pipe" => pipe = true,
                "--" => {
                    exec = it.by_ref().collect();
                    break;
                }
                other => panic!("unknown option {other}"),
            }
        }
        let file = fs::canonicalize(file.expect("--file")).unwrap();
        let original = fs::read_to_string(&file).unwrap();
        let site = edit_site(&original);
        let socket_mode = mode == "socket";
        let client = (socket_mode || !rss_at.is_empty())
            .then(|| Client::connect(socket.clone().unwrap_or_else(default_socket)));
        let flag_refs: Vec<&str> = flags.iter().map(String::as_str).collect();
        let mut argv = exec.clone();
        if !pipe {
            argv.push(file.to_string_lossy().into_owned());
        }

        let expected_exit = i32::from(dirty && request == "check");
        // `content` is what the file holds: piped to the command with `--pipe`.
        let run_once = |content: &[u8]| -> Result<Outcome, String> {
            if socket_mode {
                let path = file.to_string_lossy().into_owned();
                client
                    .as_ref()
                    .unwrap()
                    .request(&flag_refs, vec![Frame::new(&request, path)])
            } else {
                Ok(process(&argv, pipe.then_some(content)))
            }
        };

        let load_start = loadavg();
        let t = Instant::now();
        let first = run_once(original.as_bytes());
        let first_ms = t.elapsed().as_secs_f64() * 1e3;
        let mut failures = 0;
        if first.as_ref().map(|o| o.0) != Ok(0) {
            eprintln!("warm-up request: {first:?}");
            failures += 1;
        }
        let mut times = Vec::with_capacity(n);
        let mut rss = Vec::new();
        for i in 1..=n {
            let mut edited = original.clone().into_bytes();
            let letters = code(i, &edited[site..site + 3]);
            edited[site..site + 3].copy_from_slice(&letters);
            if dirty {
                edited.push(b'\n');
            }
            fs::write(&file, &edited).unwrap();
            let t = Instant::now();
            let outcome = run_once(&edited);
            times.push(t.elapsed().as_secs_f64() * 1e3);
            match &outcome {
                Ok((code, out, err)) if *code == expected_exit && out.is_empty() => {
                    // Check mode on a file needing formatting prints the count.
                    if expected_exit == 0 && !err.is_empty() {
                        failures += 1;
                    }
                }
                other => {
                    failures += 1;
                    if failures < 5 {
                        eprintln!("save {i}: {other:?}");
                    }
                }
            }
            fs::write(&file, &original).unwrap();
            if rss_at.contains(&i) {
                let pid = client.as_ref().unwrap().pid;
                rss.push((i, memory(pid)));
            }
        }
        let load_end = loadavg();
        if let Some(path) = tsv {
            let lines: String = times.iter().map(|t| format!("{t:.4}\n")).collect();
            fs::write(path, lines).unwrap();
        }
        let mut sorted = times.clone();
        sorted.sort_by(f64::total_cmp);
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        let rss: Vec<String> = rss
            .iter()
            .map(|(i, (r, h))| format!("{i}:{r}/{h}"))
            .collect();
        println!(
            "{label}\t{}\tn={n}\tfirst={first_ms:.3}\tmedian={:.3}\tp90={:.3}\tp99={:.3}\tmax={:.3}\tmean={mean:.3}\tmin={:.3}\tfailures={failures}\tload={load_start} -> {load_end}\trss_kB(VmRSS/VmHWM)={}",
            file.file_name().unwrap().to_string_lossy(),
            percentile(&sorted, 0.5),
            percentile(&sorted, 0.9),
            percentile(&sorted, 0.99),
            sorted[sorted.len() - 1],
            sorted[0],
            rss.join(" ")
        );
        if failures > 0 {
            std::process::exit(1);
        }
    }

    /// Records the text of every unit a format stores (and finds none).
    #[derive(Default)]
    struct UnitTexts(RefCell<Vec<String>>);

    impl UnitCache for UnitTexts {
        fn get(&self, _: &UnitKey<'_>) -> Option<CachedUnit> {
            None
        }
        fn insert(&self, key: &UnitKey<'_>, _: CachedUnit) {
            self.0.borrow_mut().push(key.text().to_owned());
        }
    }

    fn files_under(root: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => files_under(&path, out),
                Ok(t) if t.is_file() && path.extension().is_some_and(|e| e == "rs") => {
                    out.push(path)
                }
                _ => {}
            }
        }
    }

    /// `src` with every byte outside `lo..hi` replaced by a space, newlines kept.
    fn blanked(src: &str, lo: usize, hi: usize) -> String {
        let mut out = String::with_capacity(src.len());
        for (i, c) in src.char_indices() {
            if (lo..hi).contains(&i) || c == '\n' {
                out.push(c);
            } else {
                for _ in 0..c.len_utf8() {
                    out.push(' ');
                }
            }
        }
        out
    }

    fn copy_tree(from: &Path, to: &Path) {
        let mut files = Vec::new();
        files_under(from, &mut files);
        for file in files {
            let target = to.join(file.strip_prefix(from).unwrap());
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(&file, &target).unwrap();
        }
    }

    fn identity(args: Vec<String>) {
        let mut roots = Vec::new();
        let mut chloro = None;
        let mut socket = None;
        let mut every = 1;
        let mut blank = true;
        let mut paths = true;
        let mut scratch = None;
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            let mut value = || it.next().expect("option takes a value");
            match arg.as_str() {
                "--root" => roots.push(PathBuf::from(value())),
                "--chloro" => chloro = Some(value()),
                "--socket" => socket = Some(PathBuf::from(value())),
                "--every" => every = value().parse().unwrap(),
                "--no-blank" => blank = false,
                "--no-paths" => paths = false,
                "--scratch" => scratch = Some(PathBuf::from(value())),
                other => panic!("unknown option {other}"),
            }
        }
        let chloro = chloro.expect("--chloro BIN");
        if roots.is_empty() {
            roots.push(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../chloro-core/tests/conformance/fixtures"),
            );
        }
        let client = Client::connect(socket.unwrap_or_else(default_socket));
        let mut files = Vec::new();
        for root in &roots {
            files_under(root, &mut files);
        }
        files.sort();
        let files: Vec<PathBuf> = files.into_iter().step_by(every).collect();
        let reference = |input: &[u8], check: bool| {
            let mut argv = vec![chloro.clone(), "--stdin".into(), "--no-cache".into()];
            if check {
                argv.push("--check".into());
            }
            process(&argv, Some(input))
        };
        let differences = std::cell::Cell::new(0usize);
        let compare = |what: &str, path: &Path, served: Result<Outcome, String>, local: Outcome| {
            match served {
                Ok(served) if served == local => {}
                Ok(served) => {
                    differences.set(differences.get() + 1);
                    println!(
                        "{what} differs: {} (exit {} vs {}, stdout {} vs {} bytes, stderr {:?} vs {:?})",
                        path.display(),
                        served.0,
                        local.0,
                        served.1.len(),
                        local.1.len(),
                        String::from_utf8_lossy(&served.2),
                        String::from_utf8_lossy(&local.2)
                    );
                }
                Err(e) => {
                    differences.set(differences.get() + 1);
                    println!("{what} not answered: {}: {e}", path.display());
                }
            }
        };
        let unit = ["unit-cache"];
        let config = Config::default();
        let started = Instant::now();
        let (mut requests, mut blanked_units) = (0usize, 0usize);
        for path in &files {
            let bytes = fs::read(path).unwrap();
            let stdin = |b: &[u8]| vec![Frame::new("stdin", b.to_vec())];
            let local = reference(&bytes, false);
            compare(
                "cold",
                path,
                client.request(&unit, stdin(&bytes)),
                local.clone(),
            );
            compare("warm", path, client.request(&unit, stdin(&bytes)), local);
            compare(
                "check",
                path,
                client.request(&unit, vec![Frame::new("stdin-check", bytes.clone())]),
                reference(&bytes, true),
            );
            requests += 3;
            let Ok(src) = String::from_utf8(bytes) else {
                continue;
            };
            if !(src.starts_with('\u{feff}') || src.starts_with("#!") || src.starts_with("---")) {
                let shifted = format!("// unit cache probe\n{src}");
                let local = reference(shifted.as_bytes(), false);
                compare(
                    "shifted",
                    path,
                    client.request(&unit, stdin(shifted.as_bytes())),
                    local,
                );
                requests += 1;
            }
            if blank && !src.contains('\r') && !src.starts_with('\u{feff}') {
                let texts = UnitTexts::default();
                let _ = chloro_core::format_source_with_unit_cache(&src, &config, 1, &texts);
                for text in texts.0.into_inner() {
                    let Some(lo) = src.find(&text) else { continue };
                    let variant = blanked(&src, lo, lo + text.len());
                    let local = reference(variant.as_bytes(), false);
                    compare(
                        "blanked",
                        path,
                        client.request(&unit, stdin(variant.as_bytes())),
                        local,
                    );
                    requests += 1;
                    blanked_units += 1;
                }
            }
        }
        println!(
            "stdin: {} files, {requests} requests ({blanked_units} blanked units), {:.1} s; server: {}",
            files.len(),
            started.elapsed().as_secs_f64(),
            client.status().replace('\n', "; ")
        );

        if paths {
            let scratch = scratch.expect("--scratch DIR for the path scenarios");
            let (served, local) = (scratch.join("served"), scratch.join("local"));
            let _ = fs::remove_dir_all(&served);
            let _ = fs::remove_dir_all(&local);
            for root in &roots {
                let name = root.file_name().unwrap();
                copy_tree(root, &served.join(name));
                copy_tree(root, &local.join(name));
            }
            let cache = scratch.join("local-cache");
            let cli = |args: &[&str], target: &Path| {
                let mut argv: Vec<String> = vec![chloro.clone()];
                argv.extend(args.iter().map(|a| a.to_string()));
                argv.push(target.to_string_lossy().into_owned());
                let mut command = Command::new(&argv[0]);
                command.args(&argv[1..]).env("CHLORO_CACHE_DIR", &cache);
                let out = command.output().unwrap();
                (out.status.code().unwrap_or(-1), out.stdout, out.stderr)
            };
            let mut local_files = Vec::new();
            files_under(&local, &mut local_files);
            local_files.sort();
            let started = Instant::now();
            let tree_check = || {
                let source = vec![Frame::new("check", served.to_string_lossy().into_owned())];
                (
                    client.request(&unit, source),
                    cli(&["--check", "--unit-cache"], &local),
                )
            };
            let (served_reply, local_reply) = tree_check();
            compare("tree check", &served, served_reply, local_reply);
            let mut path_requests = 1;
            for mode in ["check", "write"] {
                for file in &local_files {
                    let twin = served.join(file.strip_prefix(&local).unwrap());
                    let source = vec![Frame::new(mode, twin.to_string_lossy().into_owned())];
                    let served_reply = client.request(&unit, source);
                    let local_reply = cli(&[&format!("--{mode}"), "--unit-cache"], file);
                    compare(mode, file, served_reply, local_reply);
                    path_requests += 1;
                    if mode == "write" && fs::read(&twin).unwrap() != fs::read(file).unwrap() {
                        differences.set(differences.get() + 1);
                        println!("written file differs: {}", file.display());
                    }
                }
            }
            let (served_reply, local_reply) = tree_check();
            compare("tree check", &served, served_reply, local_reply);
            path_requests += 1;
            println!(
                "paths: {} files, {path_requests} requests, {:.1} s; server: {}",
                local_files.len(),
                started.elapsed().as_secs_f64(),
                client.status().replace('\n', "; ")
            );
        }
        let differences = differences.get();
        println!("{differences} differences between the server's replies and the CLI's");
        if differences > 0 {
            std::process::exit(1);
        }
    }
}
