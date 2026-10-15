# EXP-021: a resident process (`chloro serve`)

Experiment E21 of the [autoresearch programme](../../2026-10-14-autoresearch.md); procedure
and gates in [protocol.md](../protocol.md). It builds trick T4 of
[../../2026-10-15-unmined-tricks.md](../../2026-10-15-unmined-tricks.md) §5 on the fixed
costs measured in §2 of
[../../2026-10-15-extensions-theory/incremental-and-process.md](../../2026-10-15-extensions-theory/incremental-and-process.md),
the executable identity of [EXP-012](EXP-012-cli-waste.md) and the unit cache of
[EXP-015](EXP-015-item-cache.md). Labels: **measured**, **read**, **derived**.

## Identity

| field | value |
|---|---|
| branch | `20261015-resident`, worktree `/home/user/wt/resident` |
| base | `8ebd3bb` (integration branch `claude/chloro-perf-research-swarm-nrscj8`; formatter code of the best-known-good `e640fbf`) |
| commits | `326615c` (server, client, protocol, `--stdin`, shared run, tests, driver), `a7a3b7a` (in-memory bound of the unit store, unit tests, interleaved driver mode), `9be1290` (invalidation tests, editor-style client mode), `0eadd9d` (one item thread per single-source request by default), `93d7d3a` (a test retries spawning a freshly copied binary), the commit of this record and the ledger row |
| scope | `chloro/src/cli*`, `chloro/tests/cli_serve.rs`, `chloro/examples/serve_driver.rs`, `chloro/Cargo.toml` (the example); no `chloro-core` change, no new dependency |
| toolchain | `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`; workspace release profile (fat LTO, one codegen unit); `cargo build --release -p chloro --features cli` (and `--examples`), `cargo build --release -p chloro-core --examples` |
| binaries | `chloro-base` (`8ebd3bb`, 2,147,536 B); `chloro-e21` (`326615c`, 2,290,328 B, +142,792 B), used for w1 and the identity runs; `chloro-e21b` (`a7a3b7a`, 2,290,328 B; `9be1290` changes no source of the binary), used for w2, w3, w4 and the start-up runs; `chloro-e21c` (`0eadd9d`, 2,291,240 B), used for w5; copies under the scratchpad's `e21/bin/` |
| host | 4 vCPU "Intel(R) Xeon(R) Processor @ 2.80GHz", Linux 6.18.44, shared with three other agents' builds and measurements: 1-minute load 2.1–15.0 during the windows, recorded per row. Absolute times are **measured, provisional** (protocol §6) and are not quiet-host figures; the claims rest on modes measured in the same window, interleaved from w2 on |
| rustfmt | `rustfmt 1.9.0-stable (2d8144b788 2026-07-07)` |

## Hypothesis and falsification

**Hypothesis** (T4). A resident chloro process answering requests over a Unix-domain socket
brings a median-file editor save from the measured 4.3–6.0 ms (about 1.8 ms process start-up,
then cache open, read and whole-file format) to under 1 ms, and a one-item edit in a large
file to its in-process unit-cache cost (0.26–0.47 of the file's format, EXP-015) instead of
1.2–2.0x of a 30 ms process, with no output difference and no memory growth across saves.

**Falsified if** (any of):

1. the median save latency on a median fixture is not below 1 ms;
2. the server's RSS after 1,000 saves is above 1.2x its RSS after 10;
3. any output or exit-code difference from the CLI on any request.

The bound to compare with (theory record § 2, **derived**): ≈ 0.31 ms per request on the
median fixture without the unit cache and ≈ 0.15 ms with it; on `lints.rs` 23–25 ms without
and 6.9–17.6 ms with.

**Added during the experiment** (the coordinator, from the external research review,
[../../2026-10-15-external-research-brief.md](../../2026-10-15-external-research-brief.md)
§1.7 and §2): the latency that decides is the **client's round trip as an editor sees it**:
a fresh client process starts, connects, sends, waits for the reply and exits, because
rust-analyzer starts the configured formatter as a new process per request. The server's
own handling time is a secondary figure. The client's start-up is measured once on its own
(hyperfine on no-op requests). Cold against warm requests are reported apart, and file
renames and configuration changes are invalidation tests. Two scoreboards below, then:
**the editor's** (a `chloro` process forwarding to the server, against the CLI and rustfmt)
decides; **the resident client's** (a process that already runs and speaks the socket,
here the driver) and the server's own time are secondary.

## Design as built (read)

**Server** (`chloro/src/cli/server.rs`, `#[cfg(unix)]`). `chloro serve [--socket PATH]
[--idle-timeout SECS] [--cache-dir D] [--threads N] [-v]`: binds a Unix-domain socket (mode
0600), answers one request per connection on one thread (the request thread formats a
single file itself; a request naming several files or a directory uses the CLI's
`format_all` and its worker threads), and keeps between requests:

| state | what | bound |
|---|---|---|
| per distinct configuration (`Config::key_bytes`) | the parsed `Config`; the fixed-point cache (`Cache::open` once: the same tool key and file as the CLI's), recorded in memory per request (`Cache::record`, a generation per request) | `MAX_ENTRIES` (131,072) in memory, trimmed at 1.25x (`Cache::bound`) |
| per distinct configuration | the unit cache (`Units`, the CLI's `HashMap` store behind the `UnitCache` trait), read from its `.chloro-units` file on the first request that uses it, then in memory only; a generation per request (`Units::next_generation`) | 64 MiB of entries counted in memory (`Units::trim`, least recently seen dropped down to three quarters) |
| per path | the file's stamp (size, modification time, device, inode, `ctime`) when its content was last read and hashed, and that digest (`stat.rs`) | 131,072 paths |
| its executable | the identity of `/proc/self/exe` (E12's identity bytes: path, size, `mtime`, device, inode, `ctime`) | — |
| threads | one worker per core for a request of several files; **one** thread for the items of a single source (file or standard input), unless `serve --threads N` or the request's `--threads` says otherwise (`0eadd9d`, see Memory) | — |

Both caches are written to their files as the CLI writes them (temporary file and rename,
only if the executable is unchanged) `SAVE_DELAY` (2 s) after the last request that changed
them, and on exit. Nothing is written in a request's path.

**Validation per request.** The request carries the client's executable identity; a
different identity (another build, another path to it) gets a fallback, and if the
server's own executable changed on disk the server exits after replying. A file named by a
`check` or `write` request is first `stat`ed: if its stamp equals the one recorded when its
content was last read, and that content is a fixed point in the request's configuration,
the file is answered without reading it. A stamp is recorded only when the file's
modification and status-change times are at least 1 s older than the stamp (git's
"racily clean" rule): a file written in the last second is read and hashed (BLAKE3) every
time, which is every request of the save loop measured below.

**Shared run.** The CLI's `main` became `run`; what it does after the configuration is
parsed is split into `collect_files` (sources, relative to a base directory), `format_all`
(unchanged, now taking a `Caches` bundle), `run_stdin`, and `finish` (summary, check-mode
error, exit code; `print_summary` writes to a given stream with `eprintln!`'s panic on a
failed write). The server calls the same functions with its own caches, the client's
working directory and in-memory output streams, so a reply carries what the CLI would have
printed and its exit code; an `io::Error` that `main` would return is printed as Rust's
runtime prints it (`Error: {e:?}`, exit 1).

**Client** (`client.rs`). `--via-server`, or `CHLORO_VIA_SERVER=1`: the CLI sends the run
to the socket (`--socket`, default `chloro.sock` in the resolved cache directory, or
`$XDG_RUNTIME_DIR/chloro-<hash>.sock` when that path exceeds `sun_path`), prints the reply's
standard output and error and exits with its code. It runs the request itself when no
server answers (no socket, a stale socket, an I/O error), when the server replies
`fallback`, and for runs the server does not take: `-v`, `--help`, print mode on paths,
`--stdin --write`. `--stdin` (new in the CLI): format standard input, print it (or, with
`--check`, exit 1 if it would change), the source named `<stdin>` in reports.

**Lifecycle.** A server that finds a live server on its socket exits with status 1; a
socket nobody answers on is replaced. The server exits on `stop`, after `--idle-timeout`
seconds without a request (default 1800, 0 for never), and when its executable changes on
disk (checked every second by a helper thread and on every refused request), removing the
socket if it is still the one it bound. A panic in a request is caught and answered with a
fallback, so the client's own run reports it as the CLI does.

### The protocol as implemented

Framing (`protocol.rs`): a message is a sequence of frames, each a header line `NAME LEN\n`
followed by exactly `LEN` bytes, ended by the frame `end 0\n`. `NAME` is 1–32 bytes of
`a-z0-9-`, `LEN` decimal without sign or leading zeros, at most 1 GiB; at most 2^20 frames.

| request frame | value |
|---|---|
| `chloro` | protocol version `1`; first |
| `exe` | the client's executable identity (bytes); compared with the server's |
| `cwd` | the client's working directory; relative paths resolve against it |
| `cache-dir` | the cache directory the client resolved; empty with `--no-cache`; a different one gets a fallback |
| `config` | the `--config` string (`key=value,...`; `edition=...` is one of its keys) |
| `threads` | `--threads` (`0`: the server's cores) |
| `flag` | `no-cache` or `unit-cache`, once each |
| `check` / `write` | a source path, once per path, one mode per request |
| `stdin` / `stdin-check` | the source text |
| `ping` / `status` / `stop` | administration, alone after `chloro` |

| reply frame | value |
|---|---|
| `status` | `ok`, `fallback` (the client runs the request itself) or `error` (unreadable request) |
| `reason` | with `fallback` and `error` |
| `exit` | the exit code, decimal |
| `stdout`, `stderr` | the bytes the CLI would have written |
| `exe` | in the reply to `ping`: the server's identity, for clients that are not chloro (the driver below) |

`status` replies with text lines: pid, requests, time spent answering (`busy-us`), paths
remembered, per configuration the fixed-point entries and the unit store's entries, bytes,
lookups, hits and stores, and `VmRSS`/`VmHWM`. The CLI's `--help` documents the command,
the options and the framing.

## Changes

| commit | change | tests |
|---|---|---|
| `326615c` | `server.rs` (`chloro serve`: socket, state, lifecycle, request handling), `client.rs` (`--via-server`, `CHLORO_VIA_SERVER=1`), `protocol.rs` (framing), `stat.rs` (per-path stamps with the settle rule); `--stdin`; `cli::main` split into `run`, `collect_files`, `run_stdin`, `report::finish`; `worker::Caches` (the fixed-point cache, the unit cache and the stamps as one argument of `format_all`/`format_file`), `worker::format_stdin`; `Cache::record`, `Cache::write`, `Cache::bound` (`save` is `record` then `write`, the same bytes); `Units::next_generation`, `write_kept`, `trim`, `held_bytes`; `cache::executable_identity`, `identity_at`; `--help` documents the server, the options and the framing; `examples/serve_driver.rs` (save loop, identity driver) | `protocol.rs` 6 (round trip with any bytes, the documented wire format, two messages on one stream, a value across buffer boundaries, 14 malformed messages refused and an endless header refused early, unwritable frames refused); `server.rs` 2 (request parsing, 10 malformed requests fall back); `stat.rs` 2 (a fresh file is not trusted and a settled one is; a same-size rewrite with the old modification time is seen through `ctime`); `tests/cli_serve.rs` 6: 22 runs through the server equal the same runs in the client process (exit code, stdout, stderr, files), among them `--check`/`--write` of files and trees, a missing path, a non-UTF-8 file, three `--config` cases, `--no-cache`, `--unit-cache` on a 167 KB fixture and on its one-item edit, `--threads`, `--stdin` in print and check mode, and the request count proves each went through the server; relative paths; no server; another executable refused (the copy's request runs in the client, the server keeps running); `stop`, a second server refused, a replaced executable ends the server; a stale socket replaced |
| `a7a3b7a` | `Units` counts an entry's in-memory size for its bound (see "What surprised"); `rounds` mode of the driver | `units.rs` 2: the store keeps its entries across requests and writes without consuming them; `trim` drops the units seen least recently |
| `9be1290` | the driver: `expect=echo` (editor-style `--stdin` clients), a default socket's cache directory derived from its path | `tests/cli_serve.rs` 1: a settled stamp is recorded and a same-size rewrite with the old modification time is still seen; a rename (formatted content under a new name, other content under the old one); configuration changes (`tab_spaces=2`, then `max_width=60`, then the default again) — each run through the server equal to the client's |
| `0eadd9d` | `chloro serve` formats the items of a single source (one file, or standard input) on one thread by default (`State::item_threads`); requests of several files keep one worker per core; `serve --threads N` or a client's `--threads` choose otherwise; `status` reports `item-threads` | `cli_serve.rs` (`--threads 2` case) and the w5 measurement |
| `93d7d3a` | — | `cli_serve.rs` retries spawning a freshly copied binary while it is busy (`ETXTBSY`): under the parallel workspace run, another test's fork can hold the copy open for writing (one failure seen in the first head run) |
| this commit | this record; the ledger row | — |

The CLI's own path is unchanged in behaviour: the same messages in the same order, the
same exit codes, the same cache files (`cli_cache.rs`, `cli_files.rs`, `cli_unit_cache.rs`
pass unchanged); it forwards only with `--via-server` or `CHLORO_VIA_SERVER=1`. `chloro serve`
as the first argument is a subcommand on Unix (a source directory named `serve` is given as
`./serve`); on other targets it is still a path.

## Exact commands

`$S` is the scratchpad's `e21/`; `$B` its `bin/`; `$W` its `work/` (w1) or `work2/` (w2): the
median fixture (`ide-diagnostics/src/handlers/break_outside_of_loop.rs`, 2,954 bytes: the
620th of the 1238 fixtures by size, the upper median; the 619th is 2,916 bytes) and the five
largest (`ide-db/src/generated/lints.rs` 639,771 B, `syntax/src/ast/generated/nodes.rs`
358,382 B, `hir/src/lib.rs` 236,066 B, `ide/src/hover/tests.rs` 210,967 B,
`rust-analyzer/src/config.rs` 169,196 B, sizes after formatting), copied and formatted
once with `chloro-base --write --no-cache` (the editor case: a formatted file), checked by a
SHA-256 list before every cell.

```text
# the save loop (chloro/examples/serve_driver.rs): per save, write F with three letters of its
# last string literal starting with three lowercase letters replaced by a code of the save's
# number (new content every save, still formatted), time one request, write F back.
# One untimed request on the unedited file first.

# w1: one cell per (file, mode), fresh cache directory and server per cell ($S/measure.sh)
taskset -c 3 $B/chloro-e21 serve --idle-timeout 3600 &            # CHLORO_CACHE_DIR=$S/m/<cell>
taskset -c 2 $B/serve_driver bench --file $W/F.rs --n 1000 --mode socket [--flag unit-cache] --rss 10,100,1000
taskset -c 2 $B/serve_driver bench --file $W/F.rs --n 1000 --mode exec --rss 10,100,1000 -- $B/chloro-e21 --check --via-server --unit-cache
taskset -c 3 $B/serve_driver bench --file $W/F.rs --n 1000 --mode exec -- $B/chloro-e21 --check [--unit-cache]
taskset -c 3 $B/serve_driver bench --file $W/F.rs --n 1000 --mode exec --pipe -- rustfmt --edition 2024 --check

# w2: interleaved ($S/rounds.sh): three servers per file (fixed-point only, unit cache, the
# CLI client's), each save timed once in all six modes in rotating order
CHLORO_CACHE_DIR=$R/{a,b,c} taskset -c 3 $B/chloro-e21b serve --idle-timeout 3600 &
taskset -c 2 $B/serve_driver_b rounds --file $W/F.rs --n 1000 --rss 10,100,1000 \
  --mode "label=socket;socket=$R/a/chloro.sock" \
  --mode "label=socket-units;socket=$R/b/chloro.sock;flags=unit-cache" \
  --mode "label=client-units;server=$R/c/chloro.sock;cache=$R/c;exec=$B/chloro-e21b --check --via-server --unit-cache" \
  --mode "label=cli;cache=$R/d;exec=$B/chloro-e21b --check" \
  --mode "label=cli-units;cache=$R/e;exec=$B/chloro-e21b --check --unit-cache" \
  --mode "label=rustfmt;pipe=1;exec=rustfmt --edition 2024 --check"

# w3: interleaved and unpinned ($S/rounds3.sh), five servers per file, nine modes; adds the
# editor-style clients (a fresh `chloro` per save) and their local counterparts
$B/chloro-e21b serve --idle-timeout 3600 &                    # one per server mode: a b c f g
$B/serve_driver_c rounds --file $W/F.rs --n 1000 --rss 10,100,1000 \
  --mode "label=socket;socket=$R/a/chloro.sock" \
  --mode "label=socket-units;socket=$R/b/chloro.sock;flags=unit-cache" \
  --mode "label=client-units;server=$R/c/chloro.sock;cache=$R/c;exec=$B/chloro-e21b --check --via-server --unit-cache" \
  --mode "label=client;server=$R/f/chloro.sock;cache=$R/f;exec=$B/chloro-e21b --check --via-server" \
  --mode "label=client-stdin-units;server=$R/g/chloro.sock;cache=$R/g;pipe=1;expect=echo;exec=$B/chloro-e21b --stdin --via-server --unit-cache" \
  --mode "label=cli;cache=$R/d;exec=$B/chloro-e21b --check" \
  --mode "label=cli-units;cache=$R/e;exec=$B/chloro-e21b --check --unit-cache" \
  --mode "label=cli-stdin-units;cache=$R/h;pipe=1;expect=echo;exec=$B/chloro-e21b --stdin --unit-cache" \
  --mode "label=rustfmt;pipe=1;exec=rustfmt --edition 2024 --check"

# the client's start-up on its own ($S/startup.sh): no-op requests on an empty directory,
# client pinned to core 2 and server to core 3, then both unpinned
hyperfine -N -w 30 -r 500 -n help "chloro --help" -n local-nocache "chloro --check --no-cache EMPTY" \
  -n local-cache "chloro --check EMPTY" -n client "chloro --check --via-server EMPTY" \
  -n client-no-server "chloro --check --via-server --socket NONE EMPTY" -n client-stdin-empty "chloro --stdin --via-server"

# w4 and w5: the server's item threads and its memory ($S/threads.sh, $S/threads5.sh):
# interleaved, unpinned, socket requests without the unit cache, two servers per file
#   w4 (chloro-e21b): the default (one item thread per core) against `serve --threads 1`
#   w5 (chloro-e21c, 0eadd9d): the new default (one item thread) against `serve --threads 4`
$B/serve_driver_c rounds --file $W/F.rs --n 1000 --rss 10,100,1000 \
  --mode "label=socket;socket=$R/a/chloro.sock" --mode "label=socket-threads1;socket=$R/b/chloro.sock"

# in process, same window as w2 (EXP-015's harness): warm on F, time F with the first edit
taskset -c 3 target/release/examples/unit_cache --edit F.orig.rs F.edited.rs -n 20

# identity through the socket (unit cache on), against `chloro-e21 --stdin --no-cache` and
# the CLI on a second copy of the tree
CHLORO_CACHE_DIR=$S/id $B/chloro-e21 serve --idle-timeout 3600 &
$B/serve_driver identity --chloro $B/chloro-e21 --scratch $S/idscratch
```

rustfmt reads the source on standard input (as editors call it): given a path, it resolves
`mod x;` declarations and fails on `config.rs` and `hir/src/lib.rs`, whose child modules are
not in the copy. The `socket` rows are what a client that is already running (an editor
plugin, the driver) sees: connect, send, format, reply. The `client-units` rows run the CLI as
the client (`exec`, argument parsing, connect, relay). Process rows on a pinned core have the
driver waiting on the same core (w1) or on core 2 (w2, where the process modes inherit the
driver's pinning and the servers sit on core 3).

## Results

Six measurement windows on the same host and day, under other agents' load (the 1-minute
load average is in every row of the tables):

| window | UTC | design | binary |
|---|---|---|---|
| w1 | 10:03–10:52 | one cell per (file, mode), fresh server and cache directory per cell; servers and the process modes on core 3, the driver on core 2 for the socket and client modes (on core 3, idle while it waits, for the process modes); 6 files × 6 modes × 1,000 saves | `chloro-e21` |
| w2 | 10:52–11:21 | interleaved (`rounds`): every save timed once in each of six modes, rotating order; the three servers on core 3, the driver and the process modes on core 2 | `chloro-e21b` |
| w3 | 11:21–11:56 | interleaved, nine modes (adds the editor-style clients), **unpinned** | `chloro-e21b` |
| start-up | 11:56–11:57 | hyperfine on no-op requests, pinned and unpinned | `chloro-e21b` |
| w4 | 11:57–12:03 | interleaved, unpinned: the server's item threads (default against `serve --threads 1`) | `chloro-e21b` |
| w5 | 12:04–12:08 | the same with the new default (`0eadd9d`) against `serve --threads 4` | `chloro-e21c` |

w3 is the fair window for comparing modes: interleaving spreads the drifting load over all
modes, and without pinning no mode is confined to the core every agent pins its
measurements to. w1 and w2 satisfy the brief's "server and driver on separate cores"; they
give the same latency verdict, and w2 favours the process modes (core 2 against the
servers' core 3). The memory verdict differs between pinned and unpinned windows (a pinned
server starts no item threads; see Memory).

### The editor's scoreboard (decides): a fresh client process per save

w3: interleaved (every save timed once in each of nine modes, rotating order), unpinned,
1,000 saves per file; ms. `first` is the request on the unedited file before the timed saves
(cold: a fresh server and empty caches); the percentiles are over the 1,000 edited saves (warm).
Load (1-minute) per file, start→end: median 3.99→2.72; lints 2.72→10.63; nodes 10.82→4.50; hir_lib 4.50→5.66; hover_tests 5.66→4.01; config 4.01→3.76.

| file | `chloro` (or rustfmt) per save | median | p90 | p99 | max | min | first |
|---|---|---|---|---|---|---|---|
| median | `--check --via-server` | 3.31 | 4.20 | 6.34 | 13.7 | 2.38 | 6.3 |
| median | `--check --via-server --unit-cache` | 3.21 | 4.10 | 6.40 | 11.6 | 2.40 | 7.5 |
| median | `--stdin --via-server --unit-cache` | 3.13 | 4.08 | 6.60 | 21.1 | 2.23 | 7.2 |
| median | `--check` | 4.88 | 6.79 | 20.96 | 84.9 | 3.28 | 6.8 |
| median | `--check --unit-cache` | 6.29 | 8.41 | 20.55 | 70.6 | 3.98 | 6.6 |
| median | `--stdin --unit-cache` | 5.50 | 7.59 | 24.48 | 69.6 | 3.42 | 5.5 |
| median | rustfmt `--check` (stdin) | 26.75 | 34.11 | 46.55 | 65.8 | 20.68 | 27.0 |
| lints | `--check --via-server` | 36.73 | 69.79 | 101.30 | 128.3 | 19.79 | 44.5 |
| lints | `--check --via-server --unit-cache` | 18.40 | 30.10 | 54.56 | 226.8 | 12.44 | 51.4 |
| lints | `--stdin --via-server --unit-cache` | 23.89 | 35.44 | 60.09 | 328.4 | 15.92 | 49.5 |
| lints | `--check` | 54.56 | 113.35 | 333.75 | 1256.2 | 31.30 | 47.3 |
| lints | `--check --unit-cache` | 36.13 | 78.02 | 413.05 | 729.6 | 20.53 | 48.2 |
| lints | `--stdin --unit-cache` | 34.11 | 64.09 | 378.74 | 1091.2 | 22.02 | 54.9 |
| lints | rustfmt `--check` (stdin) | 239.01 | 327.34 | 419.60 | 514.5 | 177.83 | 255.3 |
| nodes | `--check --via-server` | 50.37 | 89.84 | 140.69 | 191.5 | 30.00 | 123.5 |
| nodes | `--check --via-server --unit-cache` | 30.41 | 54.06 | 97.88 | 118.4 | 22.13 | 112.3 |
| nodes | `--stdin --via-server --unit-cache` | 33.76 | 54.77 | 100.98 | 152.6 | 24.69 | 177.7 |
| nodes | `--check` | 69.26 | 131.92 | 212.43 | 441.0 | 45.40 | 165.6 |
| nodes | `--check --unit-cache` | 55.70 | 104.80 | 151.69 | 192.8 | 39.33 | 215.1 |
| nodes | `--stdin --unit-cache` | 53.47 | 77.83 | 120.99 | 154.0 | 39.56 | 182.0 |
| nodes | rustfmt `--check` (stdin) | 170.38 | 242.77 | 345.59 | 443.3 | 125.00 | 351.3 |
| hir_lib | `--check --via-server` | 36.77 | 55.08 | 78.14 | 175.2 | 21.09 | 79.7 |
| hir_lib | `--check --via-server --unit-cache` | 23.65 | 36.35 | 55.98 | 80.4 | 17.59 | 64.2 |
| hir_lib | `--stdin --via-server --unit-cache` | 26.83 | 40.83 | 63.17 | 107.4 | 19.15 | 74.8 |
| hir_lib | `--check` | 49.62 | 75.91 | 130.02 | 316.2 | 31.02 | 46.3 |
| hir_lib | `--check --unit-cache` | 39.77 | 65.36 | 108.87 | 140.8 | 26.54 | 55.3 |
| hir_lib | `--stdin --unit-cache` | 38.70 | 55.89 | 74.93 | 112.1 | 25.65 | 56.8 |
| hir_lib | rustfmt `--check` (stdin) | 154.68 | 209.76 | 263.91 | 604.4 | 117.27 | 138.2 |
| hover_tests | `--check --via-server` | 12.16 | 17.23 | 25.01 | 75.1 | 7.20 | 41.0 |
| hover_tests | `--check --via-server --unit-cache` | 7.59 | 10.53 | 16.34 | 28.4 | 5.12 | 18.9 |
| hover_tests | `--stdin --via-server --unit-cache` | 9.72 | 14.27 | 22.04 | 27.9 | 6.07 | 34.3 |
| hover_tests | `--check` | 16.42 | 23.73 | 39.74 | 53.9 | 10.85 | 22.5 |
| hover_tests | `--check --unit-cache` | 14.25 | 21.74 | 32.51 | 60.8 | 8.19 | 23.9 |
| hover_tests | `--stdin --unit-cache` | 13.63 | 20.96 | 32.01 | 57.0 | 8.29 | 29.1 |
| hover_tests | rustfmt `--check` (stdin) | 69.32 | 104.43 | 145.42 | 171.2 | 45.18 | 93.2 |
| config | `--check --via-server` | 15.91 | 22.28 | 31.85 | 60.0 | 10.43 | 22.4 |
| config | `--check --via-server --unit-cache` | 11.11 | 15.15 | 23.05 | 70.9 | 8.18 | 17.8 |
| config | `--stdin --via-server --unit-cache` | 13.09 | 18.27 | 29.33 | 58.3 | 9.08 | 18.9 |
| config | `--check` | 22.07 | 30.57 | 43.78 | 76.9 | 14.45 | 19.0 |
| config | `--check --unit-cache` | 18.24 | 27.61 | 40.07 | 102.7 | 12.31 | 20.5 |
| config | `--stdin --unit-cache` | 17.65 | 27.81 | 43.55 | 54.3 | 11.95 | 16.6 |
| config | rustfmt `--check` (stdin) | 82.11 | 115.80 | 154.35 | 195.7 | 59.52 | 90.2 |

Ratios of medians, same window (w3): the plain CLI over the client, and rustfmt over the client.

| file | `--check` / `--check --via-server` | `--check --unit-cache` / `--check --via-server --unit-cache` | `--stdin --unit-cache` / `--stdin --via-server --unit-cache` | rustfmt / `--check --via-server --unit-cache` |
|---|---|---|---|---|
| median | 1.48x | 1.96x | 1.76x | 8.3x |
| lints | 1.49x | 1.96x | 1.43x | 13.0x |
| nodes | 1.38x | 1.83x | 1.58x | 5.6x |
| hir_lib | 1.35x | 1.68x | 1.44x | 6.5x |
| hover_tests | 1.35x | 1.88x | 1.40x | 9.1x |
| config | 1.39x | 1.64x | 1.35x | 7.4x |

The same client (`--check --via-server --unit-cache`) in the pinned windows, medians (p99), ms:

| file | w1 client | w1 `--check` | w1 `--check --unit-cache` | w1 rustfmt | w2 client | w2 `--check` | w2 `--check --unit-cache` | w2 rustfmt |
|---|---|---|---|---|---|---|---|---|
| median | 2.77 (8.0) | 8.29 (15.7) | 12.28 (20.5) | 26.20 (49.9) | 3.37 (12.5) | 4.29 (15.2) | 5.22 (20.1) | 24.79 (48.3) |
| lints | 20.42 (55.0) | 98.18 (195.7) | 79.70 (135.7) | 387.95 (983.5) | 27.70 (86.8) | 55.92 (123.1) | 30.91 (64.4) | 209.22 (315.4) |
| nodes | 75.63 (136.0) | 99.73 (262.0) | 87.33 (171.9) | 199.90 (539.8) | 58.27 (127.2) | 74.10 (130.7) | 48.11 (88.6) | 146.34 (229.4) |
| hir_lib | 34.14 (73.6) | 64.82 (152.6) | 64.42 (132.7) | 282.46 (558.1) | 55.63 (126.3) | 65.61 (197.6) | 39.52 (177.0) | 141.34 (239.5) |
| hover_tests | 11.20 (32.8) | 28.41 (56.4) | 24.23 (40.2) | 106.70 (171.8) | 9.08 (32.5) | 14.65 (30.1) | 10.44 (21.6) | 54.79 (93.5) |
| config | 12.52 (34.9) | 63.87 (115.8) | 47.83 (72.3) | 143.94 (313.6) | 11.82 (26.7) | 25.05 (47.5) | 17.29 (36.0) | 71.72 (114.6) |

### The client's start-up, measured on its own

hyperfine 2.0.0, `-N -w 30 -r 500`, no-op requests (an empty directory: nothing to format;
`--stdin` with empty input). ms.

pinned (client core 2, server core 3; load 3.70 4.76 5.48, load 3.97 4.79 5.48):

| command | mean ± σ | median | p90 | p99 | min | max |
|---|---|---|---|---|---|---|
| `chloro --help` (process start, print, exit) | 2.076 ± 0.491 | 2.099 | 2.474 | 3.267 | 1.538 | 9.285 |
| `chloro --check --no-cache EMPTY` (the CLI, no cache) | 2.180 ± 0.769 | 2.205 | 2.483 | 3.001 | 1.552 | 17.627 |
| `chloro --check EMPTY` (the CLI, cache opened: executable record and cache file read) | 2.349 ± 1.409 | 1.914 | 3.034 | 7.739 | 1.633 | 24.152 |
| `chloro --check --via-server EMPTY` (client, connect, server, reply) | 3.242 ± 2.010 | 2.467 | 6.694 | 9.934 | 1.767 | 15.157 |
| `chloro --check --via-server --socket NONE EMPTY` (client, failed connect, run here) | 2.301 ± 0.468 | 2.254 | 2.580 | 3.352 | 1.674 | 9.976 |
| `chloro --stdin --via-server` on empty input | 3.033 ± 1.868 | 2.327 | 6.047 | 9.729 | 1.706 | 12.929 |

unpinned (client and server unpinned; load 3.97 4.79 5.48, load 4.21 4.81 5.48):

| command | mean ± σ | median | p90 | p99 | min | max |
|---|---|---|---|---|---|---|
| `chloro --help` (process start, print, exit) | 2.471 ± 0.667 | 2.340 | 3.079 | 5.218 | 1.645 | 7.016 |
| `chloro --check --no-cache EMPTY` (the CLI, no cache) | 2.848 ± 1.275 | 2.569 | 3.487 | 8.583 | 1.778 | 16.343 |
| `chloro --check EMPTY` (the CLI, cache opened: executable record and cache file read) | 3.150 ± 1.378 | 2.901 | 3.918 | 7.681 | 1.867 | 23.814 |
| `chloro --check --via-server EMPTY` (client, connect, server, reply) | 3.936 ± 2.162 | 3.064 | 6.827 | 12.020 | 1.909 | 17.805 |
| `chloro --check --via-server --socket NONE EMPTY` (client, failed connect, run here) | 4.012 ± 1.909 | 3.717 | 6.881 | 9.882 | 1.809 | 15.520 |
| `chloro --stdin --via-server` on empty input | 4.791 ± 2.820 | 3.897 | 8.078 | 14.649 | 1.942 | 22.905 |

### The resident client's scoreboard and the server's own time (secondary)

A client that already runs and speaks the socket (the driver: connect, request, reply). Medians (p99), ms;
`first` (cold) in brackets after the window. Server time is `busy-us / requests` from its `status`
(the mean over the 1,000 saves, the cold request and the administrative ones).

| file | w1 socket | w1 socket `--unit-cache` | w2 socket | w2 socket `--unit-cache` | w3 socket | w3 socket `--unit-cache` |
|---|---|---|---|---|---|---|
| median | 0.62 (5.6) [9.4] | 0.39 (4.4) [7.8] | 0.88 (8.7) [4.2] | 0.69 (10.2) [7.7] | 0.83 (2.9) [4.3] | 0.62 (1.9) [4.3] |
| lints | 42.63 (99.7) [74.8] | 13.94 (35.0) [45.7] | 58.56 (150.0) [277.9] | 20.14 (54.5) [250.5] | 31.91 (82.8) [49.9] | 14.10 (31.5) [41.0] |
| nodes | 91.53 (171.4) [116.0] | 51.74 (121.6) [127.0] | 86.48 (167.7) [63.4] | 41.44 (79.8) [68.3] | 45.14 (107.3) [114.6] | 26.58 (72.0) [145.8] |
| hir_lib | 52.72 (104.1) [110.9] | 29.59 (57.5) [95.3] | 85.97 (185.6) [116.1] | 38.55 (88.5) [107.8] | 31.96 (64.7) [49.5] | 20.54 (44.0) [59.6] |
| hover_tests | 20.51 (43.1) [48.2] | 8.25 (23.7) [46.0] | 19.15 (49.7) [73.4] | 7.32 (22.1) [47.3] | 8.82 (20.7) [11.6] | 3.87 (10.3) [12.6] |
| config | 30.53 (62.0) [63.0] | 8.38 (19.4) [34.8] | 19.79 (39.1) [30.1] | 8.52 (18.1) [23.0] | 12.80 (28.4) [17.4] | 7.43 (18.3) [19.4] |

w1: server time per request (busy-us / requests, ms), requests

| file | socket | socket-units | client-units |
|---|---|---|---|
| median | 0.461 (1003) | 0.256 (1003) | 0.378 (1003) |
| lints | 45.704 (1003) | 15.160 (1003) | 20.562 (1003) |
| nodes | 93.359 (1003) | 53.895 (1003) | 72.789 (1003) |
| hir_lib | 56.145 (1003) | 29.528 (1003) | 30.844 (1003) |
| hover_tests | 19.736 (1003) | 6.264 (1003) | 6.234 (1003) |
| config | 28.925 (1003) | 8.399 (1003) | 9.952 (1003) |

w3: server time per request (busy-us / requests, ms), requests

| file | socket | socket-units | client | client-units | client-stdin-units |
|---|---|---|---|---|---|
| median | 0.551 (1003) | 0.373 (1003) | 0.504 (1003) | 0.399 (1003) | 0.360 (1003) |
| lints | 34.687 (1003) | 15.186 (1003) | 38.273 (1003) | 16.398 (1003) | 14.982 (1003) |
| nodes | 49.312 (1003) | 29.451 (1003) | 52.815 (1003) | 31.455 (1003) | 30.010 (1003) |
| hir_lib | 33.586 (1003) | 21.813 (1003) | 35.061 (1003) | 22.141 (1003) | 21.860 (1003) |
| hover_tests | 9.241 (1003) | 4.062 (1003) | 9.019 (1003) | 4.027 (1003) | 3.861 (1003) |
| config | 13.320 (1003) | 7.828 (1003) | 13.137 (1003) | 7.956 (1003) | 7.973 (1003) |

In process (the EXP-015 harness `unit_cache --edit`, best of 20, `taskset -c 3`, during w2, load 8.3):

| file | `format_source_with_config` | unit cache, one unit edited | ratio |
|---|---|---|---|
| median | 0.16 | 0.12 | 0.751 |
| lints | 51.36 | 10.39 | 0.202 |
| nodes | 74.24 | 36.69 | 0.494 |
| hir_lib | 64.96 | 31.56 | 0.486 |
| hover_tests | 14.83 | 2.21 | 0.149 |
| config | 17.22 | 6.67 | 0.388 |

Floors, unpinned: the same harness run unpinned after the windows (load 14.43, 14.73), against the fastest
of the 1,000 saves in w3. The server's floor is the in-process unit-cache time plus reading and hashing
the file and the exchange; the client's adds the process.

| file | in process: whole file / unit cache | socket `--unit-cache`, min | client `--check --via-server --unit-cache`, min | `chloro --check --unit-cache`, min |
|---|---|---|---|---|
| median | 0.16 / 0.12 | 0.36 | 2.40 | 3.98 |
| lints | 34.97 / 9.75 | 10.45 | 12.44 | 20.53 |
| nodes | 48.52 / 23.45 | 20.02 | 22.13 | 39.33 |
| hir_lib | 41.63 / 14.84 | 15.41 | 17.59 | 26.54 |
| hover_tests | 6.75 / 2.16 | 2.94 | 5.12 | 8.19 |
| config | 13.03 / 4.97 | 5.87 | 8.18 | 12.31 |

### Memory

w1: the server `VmRSS`/`VmHWM` (KiB, from `/proc/<pid>/status`) after 10, 100 and 1,000 saves:

| file | mode | VmRSS / VmHWM after 10 saves (KiB) | after 100 | after 1000 | 1000 / 10 (VmRSS) |
|---|---|---|---|---|---|
| median | socket, fixed-point cache | 4036/5136 | 4044/5136 | 4148/5136 | 1.028 |
| median | socket, `--unit-cache` | 3988/5144 | 4028/5144 | 4692/5144 | 1.177 |
| median | `chloro --check --via-server --unit-cache` (process) | 4036/5260 | 4076/5260 | 4744/5260 | 1.175 |
| lints | socket, fixed-point cache | 14032/14032 | 14036/14036 | 14036/14036 | 1.000 |
| lints | socket, `--unit-cache` | 13572/13572 | 13572/13572 | 13904/13904 | 1.024 |
| lints | `chloro --check --via-server --unit-cache` (process) | 13568/13568 | 13568/13568 | 13900/13900 | 1.024 |
| nodes | socket, fixed-point cache | 15572/15572 | 15696/15696 | 16572/16572 | 1.064 |
| nodes | socket, `--unit-cache` | 16932/16932 | 16936/16936 | 16936/16936 | 1.000 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 16932/16932 | 16936/16936 | 16936/16936 | 1.000 |
| hir_lib | socket, fixed-point cache | 10688/10688 | 10988/10988 | 12012/12012 | 1.124 |
| hir_lib | socket, `--unit-cache` | 11296/11296 | 11444/11444 | 12512/12512 | 1.108 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 11364/11364 | 11516/11516 | 12580/12580 | 1.107 |
| hover_tests | socket, fixed-point cache | 6264/6264 | 6268/6268 | 6268/6268 | 1.001 |
| hover_tests | socket, `--unit-cache` | 6568/6568 | 6572/6572 | 7248/7248 | 1.104 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 6612/6612 | 6616/6616 | 7292/7292 | 1.103 |
| config | socket, fixed-point cache | 7268/7268 | 7276/7276 | 7276/7276 | 1.001 |
| config | socket, `--unit-cache` | 7408/7408 | 7408/7408 | 7412/7412 | 1.001 |
| config | `chloro --check --via-server --unit-cache` (process) | 7368/7368 | 7368/7368 | 7372/7372 | 1.001 |

w2: the server `VmRSS`/`VmHWM` (KiB, from `/proc/<pid>/status`) after 10, 100 and 1,000 saves:

| file | mode | VmRSS / VmHWM after 10 saves (KiB) | after 100 | after 1000 | 1000 / 10 (VmRSS) |
|---|---|---|---|---|---|
| median | socket, fixed-point cache | 4008/5252 | 4036/5252 | 4140/5252 | 1.033 |
| median | socket, `--unit-cache` | 4036/5252 | 4084/5252 | 4748/5252 | 1.176 |
| median | `chloro --check --via-server --unit-cache` (process) | 3980/5156 | 4028/5156 | 4692/5156 | 1.179 |
| lints | socket, fixed-point cache | 14004/14004 | 14004/14004 | 14004/14004 | 1.000 |
| lints | socket, `--unit-cache` | 13624/13624 | 13624/13624 | 13920/13920 | 1.022 |
| lints | `chloro --check --via-server --unit-cache` (process) | 13624/13624 | 13624/13624 | 13920/13920 | 1.022 |
| nodes | socket, fixed-point cache | 15588/15588 | 15660/15660 | 16680/16680 | 1.070 |
| nodes | socket, `--unit-cache` | 16936/16936 | 16940/16940 | 16940/16940 | 1.000 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 16896/16896 | 16900/16900 | 16900/16900 | 1.000 |
| hir_lib | socket, fixed-point cache | 10720/10720 | 10848/10848 | 11736/11736 | 1.095 |
| hir_lib | socket, `--unit-cache` | 11324/11324 | 11324/11324 | 12412/12412 | 1.096 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 11328/11328 | 11328/11328 | 12416/12416 | 1.096 |
| hover_tests | socket, fixed-point cache | 6216/6216 | 6224/6224 | 6224/6224 | 1.001 |
| hover_tests | socket, `--unit-cache` | 6644/6644 | 6648/6648 | 7312/7312 | 1.101 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 6708/6708 | 6712/6712 | 7376/7376 | 1.100 |
| config | socket, fixed-point cache | 7272/7272 | 7280/7280 | 7280/7280 | 1.001 |
| config | socket, `--unit-cache` | 7368/7368 | 7376/7376 | 7380/7380 | 1.002 |
| config | `chloro --check --via-server --unit-cache` (process) | 7368/7368 | 7376/7376 | 7380/7380 | 1.002 |

w3: the server `VmRSS`/`VmHWM` (KiB, from `/proc/<pid>/status`) after 10, 100 and 1,000 saves:

| file | mode | VmRSS / VmHWM after 10 saves (KiB) | after 100 | after 1000 | 1000 / 10 (VmRSS) |
|---|---|---|---|---|---|
| median | socket, fixed-point cache | 4088/5332 | 4116/5332 | 4220/5332 | 1.032 |
| median | socket, `--unit-cache` | 3988/5244 | 4036/5244 | 4700/5244 | 1.179 |
| median | `chloro --check --via-server --unit-cache` (process) | 3992/5172 | 4040/5172 | 4704/5172 | 1.178 |
| median | `chloro --check --via-server` (process) | 4028/5208 | 4056/5208 | 4160/5208 | 1.033 |
| median | `chloro --stdin --via-server --unit-cache` (process) | 3988/5176 | 4024/5176 | 4684/5176 | 1.175 |
| lints | socket, fixed-point cache | 20096/20096 | 26252/26252 | 36548/36548 | 1.819 |
| lints | socket, `--unit-cache` | 19836/19836 | 19840/19840 | 20692/20692 | 1.043 |
| lints | `chloro --check --via-server --unit-cache` (process) | 19352/19352 | 19360/19360 | 19520/19520 | 1.009 |
| lints | `chloro --check --via-server` (process) | 19940/19940 | 24792/24792 | 36788/36788 | 1.845 |
| lints | `chloro --stdin --via-server --unit-cache` (process) | 19824/19824 | 20032/20032 | 22412/22412 | 1.131 |
| nodes | socket, fixed-point cache | 43844/43844 | 62608/62608 | 65072/65072 | 1.484 |
| nodes | socket, `--unit-cache` | 21092/21092 | 21512/21512 | 21516/21516 | 1.020 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 20320/20320 | 20876/20876 | 20996/20996 | 1.033 |
| nodes | `chloro --check --via-server` (process) | 51564/51564 | 63412/63412 | 64764/64764 | 1.256 |
| nodes | `chloro --stdin --via-server --unit-cache` (process) | 22740/22740 | 22748/22748 | 22752/22752 | 1.001 |
| hir_lib | socket, fixed-point cache | 18744/18744 | 32428/32428 | 33564/33700 | 1.791 |
| hir_lib | socket, `--unit-cache` | 13720/13720 | 13860/13860 | 14960/14960 | 1.090 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 13792/13792 | 13796/13796 | 14924/14924 | 1.082 |
| hir_lib | `chloro --check --via-server` (process) | 22196/22196 | 30088/30088 | 33992/33992 | 1.531 |
| hir_lib | `chloro --stdin --via-server --unit-cache` (process) | 17416/17416 | 19044/19044 | 19300/19300 | 1.108 |
| hover_tests | socket, fixed-point cache | 7656/7656 | 8612/8612 | 9524/9524 | 1.244 |
| hover_tests | socket, `--unit-cache` | 7108/7108 | 7116/7116 | 8228/8228 | 1.158 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 7284/7284 | 7292/7292 | 8000/8000 | 1.098 |
| hover_tests | `chloro --check --via-server` (process) | 7692/7692 | 8360/8360 | 10200/10200 | 1.326 |
| hover_tests | `chloro --stdin --via-server --unit-cache` (process) | 6896/6896 | 6900/6900 | 8020/8020 | 1.163 |
| config | socket, fixed-point cache | 14156/14156 | 15012/15012 | 15420/15420 | 1.089 |
| config | socket, `--unit-cache` | 8616/8616 | 8620/8620 | 8624/8624 | 1.001 |
| config | `chloro --check --via-server --unit-cache` (process) | 8524/8524 | 8532/8532 | 8772/8772 | 1.029 |
| config | `chloro --check --via-server` (process) | 13892/13892 | 15052/15052 | 15232/15232 | 1.096 |
| config | `chloro --stdin --via-server --unit-cache` (process) | 9872/9872 | 9876/9876 | 11176/11176 | 1.132 |

The server's item threads (w4, w5: interleaved, unpinned, socket requests without the unit cache, 1,000 saves):

w4, `chloro-e21b`: the default of that build (one item thread per core) against `serve --threads 1`:

| file | mode | VmRSS / VmHWM after 10 saves (KiB) | after 100 | after 1000 | 1000 / 10 (VmRSS) |
|---|---|---|---|---|---|
| lints | socket, fixed-point cache | 22096/22096 | 25348/25348 | 33492/33492 | 1.516 |
| lints | socket, server `--threads 1` | 13928/13928 | 13932/13932 | 14212/14212 | 1.020 |
| nodes | socket, fixed-point cache | 46796/46796 | 59688/59688 | 64272/64272 | 1.373 |
| nodes | socket, server `--threads 1` | 15552/15552 | 15672/15672 | 16120/16120 | 1.037 |
| hir_lib | socket, fixed-point cache | 14100/14100 | 29800/29800 | 34224/34248 | 2.427 |
| hir_lib | socket, server `--threads 1` | 10940/10940 | 11092/11092 | 11732/11732 | 1.072 |
| hover_tests | socket, fixed-point cache | 7188/7188 | 8908/8908 | 9712/9712 | 1.351 |
| hover_tests | socket, server `--threads 1` | 6156/6156 | 6156/6156 | 6164/6164 | 1.001 |
| config | socket, fixed-point cache | 14012/14012 | 15040/15040 | 15348/15348 | 1.095 |
| config | socket, server `--threads 1` | 7152/7152 | 7156/7156 | 7156/7156 | 1.001 |

| file | mode | first | median | p90 | p99 | max | min | load (1-min, start → end) |
|---|---|---|---|---|---|---|---|---|
| lints | socket, fixed-point cache | 108.48 | 48.609 | 69.322 | 91.535 | 192.41 | 22.022 | 5.37 → 8.92 |
| lints | socket, server `--threads 1` | 83.27 | 45.292 | 66.429 | 85.928 | 138.49 | 29.004 | 5.37 → 8.92 |
| nodes | socket, fixed-point cache | 78.13 | 50.704 | 79.969 | 105.029 | 123.82 | 31.172 | 8.92 → 6.30 |
| nodes | socket, server `--threads 1` | 102.63 | 56.464 | 85.763 | 122.316 | 191.12 | 40.213 | 8.92 → 6.30 |
| hir_lib | socket, fixed-point cache | 45.46 | 30.692 | 43.183 | 66.896 | 94.63 | 19.972 | 6.30 → 5.07 |
| hir_lib | socket, server `--threads 1` | 75.17 | 42.724 | 61.212 | 78.589 | 124.86 | 33.229 | 6.30 → 5.07 |
| hover_tests | socket, fixed-point cache | 13.74 | 8.677 | 11.369 | 17.557 | 39.16 | 5.883 | 5.07 → 4.84 |
| hover_tests | socket, server `--threads 1` | 17.86 | 10.171 | 14.216 | 22.164 | 34.93 | 7.228 | 5.07 → 4.84 |
| config | socket, fixed-point cache | 21.61 | 12.074 | 16.270 | 23.189 | 33.34 | 8.261 | 4.84 → 4.21 |
| config | socket, server `--threads 1` | 26.70 | 15.530 | 23.805 | 30.266 | 38.39 | 12.884 | 4.84 → 4.21 |

w5, `chloro-e21c` (`0eadd9d`): the new default (one item thread) against `serve --threads 4`:

| file | mode | VmRSS / VmHWM after 10 saves (KiB) | after 100 | after 1000 | 1000 / 10 (VmRSS) |
|---|---|---|---|---|---|
| lints | socket, fixed-point cache | 14000/14000 | 14004/14004 | 14004/14004 | 1.000 |
| lints | socket, server `--threads 4` | 21764/21764 | 25064/25064 | 32016/32016 | 1.471 |
| hir_lib | socket, fixed-point cache | 10840/10840 | 10972/10972 | 12120/12120 | 1.118 |
| hir_lib | socket, server `--threads 4` | 13732/13748 | 29896/30004 | 31528/31720 | 2.296 |

| file | mode | first | median | p90 | p99 | max | min | load (1-min, start → end) |
|---|---|---|---|---|---|---|---|---|
| lints | socket, fixed-point cache | 61.98 | 35.484 | 56.601 | 63.667 | 68.22 | 28.942 | 2.14 → 2.80 |
| lints | socket, server `--threads 4` | 46.63 | 25.768 | 34.323 | 42.563 | 74.79 | 16.967 | 2.14 → 2.80 |
| hir_lib | socket, fixed-point cache | 57.34 | 42.588 | 62.363 | 76.745 | 104.24 | 33.280 | 2.80 → 3.38 |
| hir_lib | socket, server `--threads 4` | 38.69 | 29.161 | 39.254 | 52.051 | 84.47 | 18.165 | 2.80 → 3.38 |

### Every cell

w1 (sequential cells, fresh server and cache per cell, server and process modes on core 3, driver on core 2 for the socket and client modes); ms:

| file | mode | first | median | p90 | p99 | max | min | load (1-min, start → end) |
|---|---|---|---|---|---|---|---|---|
| median | socket, fixed-point cache | 9.45 | 0.625 | 1.108 | 5.588 | 12.31 | 0.370 | 5.80 → 5.80 |
| median | socket, `--unit-cache` | 7.82 | 0.391 | 0.589 | 4.405 | 7.67 | 0.290 | 5.80 → 5.80 |
| median | `chloro --check --via-server --unit-cache` (process) | 13.87 | 2.770 | 5.719 | 7.954 | 14.38 | 1.890 | 5.80 → 5.57 |
| median | `chloro --check` (process) | 20.49 | 8.286 | 12.388 | 15.658 | 25.68 | 3.390 | 5.57 → 4.84 |
| median | `chloro --check --unit-cache` (process) | 14.00 | 12.275 | 16.399 | 20.456 | 28.30 | 3.306 | 4.84 → 4.59 |
| median | `rustfmt --edition 2024 --check` (stdin, process) | 26.40 | 26.205 | 35.327 | 49.910 | 63.97 | 19.290 | 4.59 → 5.50 |
| lints | socket, fixed-point cache | 74.80 | 42.626 | 69.362 | 99.685 | 143.45 | 28.679 | 5.50 → 5.40 |
| lints | socket, `--unit-cache` | 45.72 | 13.937 | 23.709 | 34.999 | 43.59 | 10.329 | 5.40 → 5.53 |
| lints | `chloro --check --via-server --unit-cache` (process) | 47.88 | 20.422 | 43.710 | 54.989 | 64.03 | 12.681 | 5.53 → 5.00 |
| lints | `chloro --check` (process) | 58.09 | 98.184 | 154.423 | 195.709 | 231.38 | 38.398 | 5.00 → 5.68 |
| lints | `chloro --check --unit-cache` (process) | 177.63 | 79.699 | 104.605 | 135.711 | 198.87 | 34.845 | 5.68 → 5.85 |
| lints | `rustfmt --edition 2024 --check` (stdin, process) | 589.83 | 387.948 | 731.664 | 983.484 | 1215.69 | 182.506 | 5.85 → 14.96 |
| nodes | socket, fixed-point cache | 116.00 | 91.526 | 134.927 | 171.423 | 196.06 | 47.282 | 14.96 → 15.01 |
| nodes | socket, `--unit-cache` | 127.02 | 51.742 | 82.057 | 121.598 | 362.32 | 21.760 | 15.01 → 12.83 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 361.85 | 75.633 | 106.595 | 135.973 | 161.32 | 31.944 | 12.83 → 12.78 |
| nodes | `chloro --check` (process) | 157.53 | 99.734 | 157.983 | 261.978 | 545.62 | 61.525 | 12.78 → 13.88 |
| nodes | `chloro --check --unit-cache` (process) | 98.81 | 87.332 | 129.687 | 171.858 | 547.11 | 40.286 | 13.88 → 11.99 |
| nodes | `rustfmt --edition 2024 --check` (stdin, process) | 277.30 | 199.903 | 367.699 | 539.753 | 692.30 | 122.749 | 11.99 → 9.13 |
| hir_lib | socket, fixed-point cache | 110.86 | 52.724 | 76.284 | 104.087 | 130.63 | 34.317 | 9.13 → 9.61 |
| hir_lib | socket, `--unit-cache` | 95.31 | 29.588 | 42.015 | 57.492 | 90.97 | 16.477 | 9.61 → 10.84 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 77.31 | 34.137 | 52.813 | 73.604 | 87.63 | 18.006 | 10.84 → 9.12 |
| hir_lib | `chloro --check` (process) | 49.72 | 64.816 | 114.660 | 152.621 | 181.21 | 44.420 | 9.12 → 7.36 |
| hir_lib | `chloro --check --unit-cache` (process) | 113.10 | 64.415 | 92.411 | 132.658 | 163.70 | 28.550 | 7.36 → 7.27 |
| hir_lib | `rustfmt --edition 2024 --check` (stdin, process) | 175.75 | 282.458 | 432.750 | 558.100 | 708.28 | 122.510 | 7.27 → 7.79 |
| hover_tests | socket, fixed-point cache | 48.21 | 20.512 | 29.787 | 43.091 | 55.82 | 7.917 | 7.97 → 7.98 |
| hover_tests | socket, `--unit-cache` | 46.00 | 8.251 | 16.736 | 23.719 | 55.82 | 3.096 | 7.98 → 9.76 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 63.52 | 11.203 | 21.386 | 32.758 | 64.00 | 4.967 | 9.76 → 10.39 |
| hover_tests | `chloro --check` (process) | 32.55 | 28.405 | 40.502 | 56.436 | 184.50 | 17.160 | 10.39 → 10.03 |
| hover_tests | `chloro --check --unit-cache` (process) | 35.31 | 24.229 | 31.636 | 40.208 | 43.80 | 8.514 | 10.03 → 9.38 |
| hover_tests | `rustfmt --edition 2024 --check` (stdin, process) | 120.34 | 106.699 | 144.619 | 171.797 | 199.81 | 65.789 | 9.38 → 8.50 |
| config | socket, fixed-point cache | 62.95 | 30.534 | 44.629 | 62.043 | 74.81 | 13.477 | 8.50 → 8.00 |
| config | socket, `--unit-cache` | 34.82 | 8.379 | 12.695 | 19.368 | 36.38 | 6.005 | 8.00 → 7.54 |
| config | `chloro --check --via-server --unit-cache` (process) | 31.30 | 12.522 | 19.948 | 34.921 | 47.35 | 8.030 | 7.54 → 6.92 |
| config | `chloro --check` (process) | 52.34 | 63.865 | 95.665 | 115.765 | 168.43 | 19.168 | 6.92 → 5.27 |
| config | `chloro --check --unit-cache` (process) | 72.70 | 47.828 | 60.601 | 72.330 | 110.10 | 28.297 | 5.27 → 4.76 |
| config | `rustfmt --edition 2024 --check` (stdin, process) | 209.39 | 143.937 | 231.885 | 313.597 | 360.27 | 82.857 | 4.76 → 4.49 |

w2 (interleaved, servers on core 3, driver and the process modes on core 2); ms:

| file | mode | first | median | p90 | p99 | max | min | load (1-min, start → end) |
|---|---|---|---|---|---|---|---|---|
| median | socket, fixed-point cache | 4.19 | 0.877 | 4.736 | 8.655 | 14.61 | 0.488 | 4.49 → 5.85 |
| median | socket, `--unit-cache` | 7.75 | 0.687 | 4.993 | 10.220 | 84.02 | 0.339 | 4.49 → 5.85 |
| median | `chloro --check --via-server --unit-cache` (process) | 16.33 | 3.373 | 9.136 | 12.502 | 21.11 | 2.137 | 4.49 → 5.85 |
| median | `chloro --check` (process) | 6.21 | 4.294 | 8.446 | 15.208 | 35.19 | 2.945 | 4.49 → 5.85 |
| median | `chloro --check --unit-cache` (process) | 5.53 | 5.225 | 13.717 | 20.071 | 144.19 | 3.352 | 4.49 → 5.85 |
| median | `rustfmt --edition 2024 --check` (stdin, process) | 30.72 | 24.790 | 37.186 | 48.270 | 54.13 | 19.841 | 4.49 → 5.85 |
| lints | socket, fixed-point cache | 277.93 | 58.565 | 89.858 | 150.038 | 205.16 | 30.031 | 5.85 → 9.09 |
| lints | socket, `--unit-cache` | 250.46 | 20.143 | 32.626 | 54.495 | 76.62 | 10.979 | 5.85 → 9.09 |
| lints | `chloro --check --via-server --unit-cache` (process) | 362.86 | 27.704 | 48.944 | 86.794 | 115.00 | 13.613 | 5.85 → 9.09 |
| lints | `chloro --check` (process) | 138.12 | 55.924 | 82.379 | 123.104 | 177.01 | 40.110 | 5.85 → 9.09 |
| lints | `chloro --check --unit-cache` (process) | 136.74 | 30.912 | 44.515 | 64.406 | 109.33 | 20.116 | 5.85 → 9.09 |
| lints | `rustfmt --edition 2024 --check` (stdin, process) | 414.41 | 209.222 | 265.551 | 315.397 | 391.10 | 174.187 | 5.85 → 9.09 |
| nodes | socket, fixed-point cache | 63.43 | 86.479 | 115.994 | 167.749 | 210.43 | 41.220 | 9.09 → 2.28 |
| nodes | socket, `--unit-cache` | 68.26 | 41.436 | 56.492 | 79.825 | 122.63 | 20.330 | 9.09 → 2.28 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 68.55 | 58.266 | 82.141 | 127.221 | 169.21 | 22.112 | 9.09 → 2.28 |
| nodes | `chloro --check` (process) | 85.35 | 74.104 | 101.737 | 130.721 | 252.17 | 55.213 | 9.09 → 2.28 |
| nodes | `chloro --check --unit-cache` (process) | 72.12 | 48.111 | 65.558 | 88.594 | 170.79 | 36.779 | 9.09 → 2.28 |
| nodes | `rustfmt --edition 2024 --check` (stdin, process) | 164.39 | 146.343 | 186.926 | 229.365 | 269.00 | 122.904 | 9.09 → 2.28 |
| hir_lib | socket, fixed-point cache | 116.06 | 85.971 | 133.144 | 185.565 | 218.36 | 37.716 | 2.28 → 6.77 |
| hir_lib | socket, `--unit-cache` | 107.84 | 38.551 | 61.497 | 88.462 | 112.71 | 16.154 | 2.28 → 6.77 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 129.94 | 55.629 | 88.074 | 126.279 | 147.27 | 18.995 | 2.28 → 6.77 |
| hir_lib | `chloro --check` (process) | 51.27 | 65.605 | 100.510 | 197.579 | 526.82 | 44.874 | 2.28 → 6.77 |
| hir_lib | `chloro --check --unit-cache` (process) | 53.21 | 39.521 | 56.907 | 177.039 | 496.97 | 25.990 | 2.28 → 6.77 |
| hir_lib | `rustfmt --edition 2024 --check` (stdin, process) | 144.72 | 141.345 | 189.689 | 239.531 | 360.00 | 113.230 | 2.28 → 6.77 |
| hover_tests | socket, fixed-point cache | 73.38 | 19.149 | 34.766 | 49.663 | 59.13 | 7.787 | 6.77 → 3.04 |
| hover_tests | socket, `--unit-cache` | 47.31 | 7.315 | 13.650 | 22.095 | 30.54 | 3.138 | 6.77 → 3.04 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 64.68 | 9.082 | 18.696 | 32.536 | 42.24 | 5.195 | 6.77 → 3.04 |
| hover_tests | `chloro --check` (process) | 17.80 | 14.650 | 21.828 | 30.103 | 40.50 | 11.897 | 6.77 → 3.04 |
| hover_tests | `chloro --check --unit-cache` (process) | 17.80 | 10.441 | 15.021 | 21.640 | 28.37 | 7.557 | 6.77 → 3.04 |
| hover_tests | `rustfmt --edition 2024 --check` (stdin, process) | 52.36 | 54.793 | 76.977 | 93.529 | 151.91 | 44.406 | 6.77 → 3.04 |
| config | socket, fixed-point cache | 30.14 | 19.793 | 29.404 | 39.065 | 64.27 | 13.495 | 3.04 → 3.99 |
| config | socket, `--unit-cache` | 22.96 | 8.515 | 13.086 | 18.053 | 28.16 | 5.840 | 3.04 → 3.99 |
| config | `chloro --check --via-server --unit-cache` (process) | 23.27 | 11.825 | 17.656 | 26.714 | 46.61 | 8.167 | 3.04 → 3.99 |
| config | `chloro --check` (process) | 21.04 | 25.052 | 36.009 | 47.522 | 74.70 | 18.797 | 3.04 → 3.99 |
| config | `chloro --check --unit-cache` (process) | 23.11 | 17.289 | 24.878 | 36.010 | 71.16 | 11.763 | 3.04 → 3.99 |
| config | `rustfmt --edition 2024 --check` (stdin, process) | 66.93 | 71.720 | 95.876 | 114.566 | 151.62 | 58.431 | 3.04 → 3.99 |

w3 (interleaved, unpinned); ms:

| file | mode | first | median | p90 | p99 | max | min | load (1-min, start → end) |
|---|---|---|---|---|---|---|---|---|
| median | socket, fixed-point cache | 4.28 | 0.832 | 1.122 | 2.910 | 10.48 | 0.515 | 3.99 → 2.72 |
| median | socket, `--unit-cache` | 4.29 | 0.625 | 0.883 | 1.927 | 11.71 | 0.360 | 3.99 → 2.72 |
| median | `chloro --check --via-server --unit-cache` (process) | 7.54 | 3.206 | 4.101 | 6.402 | 11.65 | 2.400 | 3.99 → 2.72 |
| median | `chloro --check --via-server` (process) | 6.30 | 3.306 | 4.202 | 6.337 | 13.71 | 2.380 | 3.99 → 2.72 |
| median | `chloro --stdin --via-server --unit-cache` (process) | 7.21 | 3.135 | 4.080 | 6.600 | 21.05 | 2.230 | 3.99 → 2.72 |
| median | `chloro --check` (process) | 6.84 | 4.882 | 6.787 | 20.964 | 84.89 | 3.283 | 3.99 → 2.72 |
| median | `chloro --check --unit-cache` (process) | 6.60 | 6.288 | 8.414 | 20.547 | 70.59 | 3.979 | 3.99 → 2.72 |
| median | `chloro --stdin --unit-cache` (process) | 5.49 | 5.502 | 7.587 | 24.475 | 69.61 | 3.420 | 3.99 → 2.72 |
| median | `rustfmt --edition 2024 --check` (stdin, process) | 26.96 | 26.745 | 34.111 | 46.548 | 65.82 | 20.677 | 3.99 → 2.72 |
| lints | socket, fixed-point cache | 49.85 | 31.910 | 53.335 | 82.790 | 109.33 | 17.790 | 2.72 → 10.63 |
| lints | socket, `--unit-cache` | 41.00 | 14.101 | 22.488 | 31.505 | 45.62 | 10.453 | 2.72 → 10.63 |
| lints | `chloro --check --via-server --unit-cache` (process) | 51.36 | 18.401 | 30.104 | 54.559 | 226.81 | 12.442 | 2.72 → 10.63 |
| lints | `chloro --check --via-server` (process) | 44.47 | 36.732 | 69.792 | 101.295 | 128.34 | 19.792 | 2.72 → 10.63 |
| lints | `chloro --stdin --via-server --unit-cache` (process) | 49.52 | 23.893 | 35.441 | 60.089 | 328.44 | 15.918 | 2.72 → 10.63 |
| lints | `chloro --check` (process) | 47.33 | 54.564 | 113.349 | 333.746 | 1256.24 | 31.297 | 2.72 → 10.63 |
| lints | `chloro --check --unit-cache` (process) | 48.25 | 36.134 | 78.015 | 413.047 | 729.64 | 20.534 | 2.72 → 10.63 |
| lints | `chloro --stdin --unit-cache` (process) | 54.85 | 34.109 | 64.089 | 378.735 | 1091.15 | 22.021 | 2.72 → 10.63 |
| lints | `rustfmt --edition 2024 --check` (stdin, process) | 255.27 | 239.010 | 327.337 | 419.604 | 514.46 | 177.831 | 2.72 → 10.63 |
| nodes | socket, fixed-point cache | 114.62 | 45.143 | 71.999 | 107.253 | 161.99 | 27.488 | 10.82 → 4.50 |
| nodes | socket, `--unit-cache` | 145.79 | 26.581 | 42.981 | 72.018 | 96.73 | 20.020 | 10.82 → 4.50 |
| nodes | `chloro --check --via-server --unit-cache` (process) | 112.27 | 30.414 | 54.056 | 97.878 | 118.40 | 22.127 | 10.82 → 4.50 |
| nodes | `chloro --check --via-server` (process) | 123.45 | 50.367 | 89.837 | 140.685 | 191.51 | 30.001 | 10.82 → 4.50 |
| nodes | `chloro --stdin --via-server --unit-cache` (process) | 177.73 | 33.755 | 54.768 | 100.978 | 152.64 | 24.686 | 10.82 → 4.50 |
| nodes | `chloro --check` (process) | 165.62 | 69.264 | 131.921 | 212.429 | 440.99 | 45.399 | 10.82 → 4.50 |
| nodes | `chloro --check --unit-cache` (process) | 215.06 | 55.705 | 104.803 | 151.692 | 192.77 | 39.326 | 10.82 → 4.50 |
| nodes | `chloro --stdin --unit-cache` (process) | 182.00 | 53.465 | 77.830 | 120.994 | 154.00 | 39.562 | 10.82 → 4.50 |
| nodes | `rustfmt --edition 2024 --check` (stdin, process) | 351.26 | 170.375 | 242.766 | 345.592 | 443.29 | 125.004 | 10.82 → 4.50 |
| hir_lib | socket, fixed-point cache | 49.46 | 31.961 | 46.995 | 64.675 | 107.92 | 19.035 | 4.50 → 5.66 |
| hir_lib | socket, `--unit-cache` | 59.58 | 20.544 | 31.052 | 44.011 | 70.14 | 15.411 | 4.50 → 5.66 |
| hir_lib | `chloro --check --via-server --unit-cache` (process) | 64.17 | 23.654 | 36.346 | 55.976 | 80.44 | 17.588 | 4.50 → 5.66 |
| hir_lib | `chloro --check --via-server` (process) | 79.71 | 36.773 | 55.083 | 78.145 | 175.22 | 21.089 | 4.50 → 5.66 |
| hir_lib | `chloro --stdin --via-server --unit-cache` (process) | 74.77 | 26.827 | 40.825 | 63.173 | 107.37 | 19.154 | 4.50 → 5.66 |
| hir_lib | `chloro --check` (process) | 46.30 | 49.617 | 75.906 | 130.024 | 316.21 | 31.018 | 4.50 → 5.66 |
| hir_lib | `chloro --check --unit-cache` (process) | 55.25 | 39.765 | 65.364 | 108.874 | 140.75 | 26.535 | 4.50 → 5.66 |
| hir_lib | `chloro --stdin --unit-cache` (process) | 56.77 | 38.702 | 55.891 | 74.928 | 112.06 | 25.648 | 4.50 → 5.66 |
| hir_lib | `rustfmt --edition 2024 --check` (stdin, process) | 138.17 | 154.683 | 209.759 | 263.914 | 604.43 | 117.265 | 4.50 → 5.66 |
| hover_tests | socket, fixed-point cache | 11.57 | 8.816 | 14.165 | 20.723 | 46.98 | 4.916 | 5.66 → 4.01 |
| hover_tests | socket, `--unit-cache` | 12.64 | 3.874 | 6.008 | 10.259 | 23.98 | 2.944 | 5.66 → 4.01 |
| hover_tests | `chloro --check --via-server --unit-cache` (process) | 18.92 | 7.588 | 10.533 | 16.341 | 28.45 | 5.117 | 5.66 → 4.01 |
| hover_tests | `chloro --check --via-server` (process) | 41.00 | 12.164 | 17.228 | 25.013 | 75.12 | 7.203 | 5.66 → 4.01 |
| hover_tests | `chloro --stdin --via-server --unit-cache` (process) | 34.30 | 9.725 | 14.268 | 22.044 | 27.88 | 6.066 | 5.66 → 4.01 |
| hover_tests | `chloro --check` (process) | 22.47 | 16.421 | 23.732 | 39.741 | 53.88 | 10.853 | 5.66 → 4.01 |
| hover_tests | `chloro --check --unit-cache` (process) | 23.87 | 14.248 | 21.736 | 32.509 | 60.80 | 8.193 | 5.66 → 4.01 |
| hover_tests | `chloro --stdin --unit-cache` (process) | 29.13 | 13.628 | 20.965 | 32.008 | 57.02 | 8.289 | 5.66 → 4.01 |
| hover_tests | `rustfmt --edition 2024 --check` (stdin, process) | 93.25 | 69.323 | 104.429 | 145.424 | 171.22 | 45.184 | 5.66 → 4.01 |
| config | socket, fixed-point cache | 17.36 | 12.805 | 18.641 | 28.411 | 155.97 | 8.414 | 4.01 → 3.76 |
| config | socket, `--unit-cache` | 19.35 | 7.426 | 11.097 | 18.328 | 23.70 | 5.870 | 4.01 → 3.76 |
| config | `chloro --check --via-server --unit-cache` (process) | 17.80 | 11.109 | 15.147 | 23.047 | 70.86 | 8.184 | 4.01 → 3.76 |
| config | `chloro --check --via-server` (process) | 22.42 | 15.914 | 22.284 | 31.854 | 60.02 | 10.427 | 4.01 → 3.76 |
| config | `chloro --stdin --via-server --unit-cache` (process) | 18.88 | 13.092 | 18.272 | 29.334 | 58.31 | 9.080 | 4.01 → 3.76 |
| config | `chloro --check` (process) | 19.04 | 22.072 | 30.572 | 43.783 | 76.94 | 14.446 | 4.01 → 3.76 |
| config | `chloro --check --unit-cache` (process) | 20.49 | 18.241 | 27.611 | 40.074 | 102.65 | 12.310 | 4.01 → 3.76 |
| config | `chloro --stdin --unit-cache` (process) | 16.61 | 17.651 | 27.808 | 43.550 | 54.32 | 11.950 | 4.01 → 3.76 |
| config | `rustfmt --edition 2024 --check` (stdin, process) | 90.18 | 82.112 | 115.796 | 154.349 | 195.68 | 59.515 | 4.01 → 3.76 |

### Identity through the socket

`serve_driver identity` against one server (`chloro-e21`, unit cache on in every request),
every reply compared byte for byte (exit code, standard output, standard error) with
`chloro-e21 --stdin --no-cache [--check]` run as a process on the same input, then with the
CLI on a second copy of the tree; log `$S/identity-fixtures.log`.

| scenario (EXP-015's, through the socket) | requests | result |
|---|---|---|
| cold: each of the 1238 fixtures as `stdin` (the unit cache empty for it, or holding equal units of earlier files) | 1,238 | 0 differences |
| warm: the same again (every unit found) | 1,238 | 0 differences |
| `stdin-check` of each fixture | 1,238 | 0 differences (exit code and the "need formatting" line) |
| shifted: a comment line inserted at the start (every unit moves a line; hits rebased) | 1,227 (no BOM, `#!` or front matter) | 0 differences |
| blanked: for every unit of every file without `\r` or a BOM, the file with every byte outside the unit replaced by a space | 13,644 | 0 differences |
| per file, `check PATH` on copy A against `chloro --check --unit-cache` on copy B | 1,238 | 0 differences |
| per file, `write PATH` on A against `chloro --write --unit-cache` on B, and the written file | 1,238 | 0 differences, 0 files differ |
| the tree, `check DIR` before the writes (exit 1, "Error: N file(s) need formatting") and after (exit 0) | 2 | 0 differences |

The server's counts after the stdin scenarios: 62,929 unit lookups, 49,649 hits, 14,064
stores, 13,280 entries (EXP-015's cold count: 13,280 stored); after the path scenarios
96,175 lookups and 82,895 hits, 1,219 paths remembered by stamp, 1,748 fixed-point entries.
18,585 + 2,478 requests in 145.0 s + 117.9 s, every one answered `ok` (a fallback counts as a
difference). The save loops checked 126,000 more replies (36,000 in w1, 36,000 in w2,
54,000 in w3; 66,000 of them through a server): every save of every file in every mode
returned exit 0 with empty output, or the input for the `--stdin` modes (`failures=0` in
every row).

The integration tests compare the server with the client process on the error paths the
loops do not reach (a missing path, an invalid or unknown `--config`, a non-UTF-8 file,
`--stdin` with invalid UTF-8, `--stdin --write`), on several sources in one request, on
relative paths, and on renames, configuration changes and a same-size rewrite under a
known stamp.

**Differences that remain by construction** (none reached by any test): a panic in the
formatter on a worker thread prints its message on the server's standard error, not the
client's (the CLI's own quirk of dropping that file's result is reproduced; a panic on the
request's thread falls back and the client's own run prints it); and the client writes the
reply's standard output and then its standard error, where the CLI interleaves them in time
(each stream's bytes are the same).

## Gates

`chloro-core` has no change on the branch, so the formatter gates run the base's formatter;
they were run on `326615c` and again on `0eadd9d` (the results below are the second run;
the first gave the same counts and `Ir` 248,199,435). `93d7d3a` changes one test only;
format, lint and the workspace tests ran on it. A first workspace run on `0eadd9d` failed
one `cli_serve` test on `ETXTBSY` (see Changes) and stopped before `chloro-core`'s tests.

| gate | command | result |
|---|---|---|
| format | `cargo fmt --all --check` | clean |
| lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| tests | `cargo test --workspace --all-features` | 309 passed, 1 ignored (pre-existing), 0 failed: `chloro` 36 unit (`protocol` 6, `server` 2, `stat` 2, `units` 2 new), `cli_cache` 11, `cli_files` 5, `cli_serve` 7 (new), `cli_unit_cache` 3; `chloro-core` 196 unit and its integration and doc tests, unchanged |
| goldens, fixtures | `target/release/examples/bench -n 1 --check …/golden/fixtures-e8d87c2.tsv --root /home/user/chloro/chloro-core/tests/conformance/fixtures` | 0 of 1238 outputs changed |
| goldens, fixtures, 4 threads | the same with `--threads 4` | 0 of 1238 differ between 4 threads and `format_source`; 0 of 1238 changed |
| goldens, registry | `bench -n 1 --check …/golden/registry-e8d87c2.tsv --root /root/.cargo/registry/src` | 0 of 25,024 outputs changed (4,174 newer files not recorded) |
| conformance | `target/release/examples/conform -i`; `conform -i -V 1.8.0` | 1220/1220 identical, 0 not idempotent; 1220/1220, 0 not idempotent |
| instructions | `benchmarks/profile.sh --core 3 --bench target/release/examples/bench` | 248,199,637 (248,199,435 on `326615c`) against 248,202,094 (the formatter is untouched) |
| a non-Unix target | `cargo check --target wasm32-wasip1 -p chloro --features cli --bins` | compiles; no warning from `chloro` (the server, client and protocol are not compiled there); one warning from `chloro-core` (`unused_mut` at `formatter/comment.rs:795`) that only this target shows, not from this change |

## What surprised

- **The client's start-up is the editor path's floor, not the server.** A server answers a
  median-file save in 0.26 ms of its own time (`busy-us` / requests, w1) and 0.39–0.69 ms at
  a resident client; a fresh `chloro` process that forwards the same request takes 2.8–3.4
  ms (w1, w2), of which the process start is most (hyperfine, below). rust-analyzer starts the
  formatter as a process per request, so that, not the socket round trip, is the latency its
  users see.
- **A new process on a loaded pinned core waits for the scheduler.** The pinned process
  rows of w1 cluster at multiples of 4 ms (the tick): on the median file 522 of 1,000 plain
  CLI runs took 8.0–8.99 ms and 252 took 12.0–12.99 ms, against a minimum of 3.39 ms. Every
  agent on this host pins its measurements to core 3, where w1 ran everything; w2 put the
  servers there and the process modes on core 2, which biases w2 against the server; w3 is
  unpinned. The comparisons that decide are taken within one window, interleaved.
- **The fixed-point cache answers a repeated edit.** The first probe re-used one server for
  two modes, and the second mode's "saves" were cache hits (0.9 ms on `lints.rs`): every
  edit the first mode made was recorded as a fixed point. Every cell and every mode has its
  own server and cache directory since, and the edit codes differ per save.
- **One worker thread per request cost 0.15 ms.** `format_all` starts a scoped worker even
  for one file; under load that start and join cost about 0.15 ms of a 0.5–0.9 ms request
  (A/B on the median file, 500 saves, two rounds). The server formats a request's single file
  on its own thread (a panic there is caught and answered with a fallback); the CLI keeps
  its worker.
- **The default socket path did not fit `sun_path`.** The scratchpad's cache directory plus
  `chloro.sock` exceeded 107 bytes. The default now falls back to
  `$XDG_RUNTIME_DIR/chloro-<hash of the cache directory>.sock` and the measurements use short
  cache directories.
- **A verbatim unit record is small in the file and not in memory.** 1,003 records of an
  already formatted unit count 40 bytes each in the file layout, while RSS rose about
  0.7 KiB per save on the median fixture (the map entry, its table's doubling, the
  fixed-point entry of the same save). `Units::trim` counted the file bytes, so the 64 MiB
  bound would have let the store reach several times that in memory; `a7a3b7a` counts the
  entry's in-memory size.
- **Item threads per request grow the server.** Pinned to one core (w1, w2) the server
  starts no item threads and its RSS stayed within 1.18x. Unpinned (w3), a whole-file
  format of a file of 128 KiB or more started one item thread per core on every save, and
  the server without the unit cache grew 1.09–1.85x over 1,000 saves, four of the five
  large files past the 1.2x falsifier (`hir_lib.rs` 18.3 → 32.8 MiB); w4 isolated it (default against `serve --threads 1`,
  interleaved: 1.10–2.43x against 1.00–1.07x), and `0eadd9d` makes one item thread the
  default for single-source requests; w5 confirms it on the new binary (1.000x and 1.118x
  against 1.471x and 2.296x with `--threads 4`), at +38–46% latency for a whole-file format
  without the unit cache and none with it (one unit, the request's thread).
- **Multi-threaded requests leave memory behind.** After the identity run's whole-tree
  requests (four worker threads) the server's RSS was 78 MiB against 36 MiB after 18,585
  single-source requests: glibc keeps per-thread arenas. The save path (one file, the
  request's thread) does not touch them.

## What was not built, and why

| not built | why | consequence |
|---|---|---|
| starting the server from the client | the brief keeps the server opt-in; a client that forks a daemon needs a lifecycle decision (who owns it, where its errors go) | `chloro serve` is started by hand or by an editor integration |
| a separate small client (`fmt-client`) | the CLI is the client (`--via-server`); a smaller binary would still pay `exec` | the CLI as client costs a process start (the `client-units` rows); only a client that already runs (an editor plugin speaking the socket, the driver here) sees the socket rows |
| an LSP `textDocument/formatting` front end | out of scope for the prototype | follow-up |
| Windows (named pipes) | `#[cfg(unix)]`: on other targets the server, the client and the driver are not compiled and `--via-server` runs the request in the process; the non-Unix build was not compiled here (no other target installed) | follow-up |
| concurrent requests | one request at a time keeps the state single-owner; a request for a whole tree blocks saves for its duration | a pool, or a request queue per path, if editors share a server with CI-sized requests |
| signal handling | no dependency for it (`libc`, `signal-hook`) | a killed server loses what it recorded since its last write (2 s after the last request) and leaves its socket, which the next server replaces |
| returning freed memory after multi-threaded requests | needs `malloc_trim` through FFI (`unsafe`) | a whole-tree request leaves glibc's per-thread arenas holding memory (36 → 78 MiB in the identity run); the single-file save path does not touch them |
| a configuration file (`rustfmt.toml`) | the CLI has none; the protocol carries the `--config` string (`edition=...` included) | follows the CLI when it gains one |
| keeping the parsed tree per path (incremental reparse, the theory's last column) | the flat tree has no reparse; the unit cache saves the format of unchanged units, not the parse | the parse of the whole file stays in every request |
| print mode on paths, `-v` | the CLI prints files in completion order and logs cache paths; neither is a save | run in the client process |
| a peer-credential check | `SO_PEERCRED` needs FFI; the socket is created mode 0600 in the user's cache directory | a process that can write the user's cache directory can already add cache entries |

## Decision

**Refuted as stated at the latency that decides; not shipped; kept on its branch.**

1. *Median save latency on the median fixture below 1 ms.* At the editor's path (a fresh
   `chloro` per save that forwards to the server, the criterion added during the
   experiment): 3.13–3.31 ms median in w3 (p99 6.3–6.6 ms), 2.77 ms in w1 and 3.37 ms in
   w2. **Not below 1 ms: falsified.** The process alone costs 2.1–2.3 ms median before it
   does anything (`chloro --help`, hyperfine), and a no-op request through the server
   2.5–3.1 ms, so no server can bring a process-per-request editor under 1 ms. For a client
   that already runs (the secondary scoreboard) the same save is 0.39–0.88 ms median in the
   three windows (0.62–0.83 ms in w3), and the server's own time 0.26–0.55 ms: that part of
   the hypothesis holds.
2. *RSS after 1,000 saves within 1.2x of after 10.* Held in the pinned windows (at most
   1.179x). **Failed unpinned** for whole-file formats of large files without the unit cache
   (w3: 1.09–1.85x, four of five large files past 1.2x), from item threads started on every
   request; fixed in `0eadd9d` (one item thread for a single source: 1.000x and 1.118x in
   w5 against 1.471x and 2.296x with four), at +38–46% latency for that path. With the unit
   cache every window stays within 1.18x. RSS still grows by about 0.7 KiB per save of new
   content (one fixed-point entry and one unit record) up to the caches' bounds: the median
   fixture's 1.175–1.179x would pass 1.2x after roughly 1,200 such saves.
3. *Any output or exit-code difference from the CLI.* None: 0 differences in 21,063 identity
   requests and in 126,000 checked save replies (66,000 through a server). Held.

The second half of the hypothesis holds where the first does, at a resident client: the
fastest unit-cache save of a large file through the socket is the in-process unit-cache time
plus 0.2–0.9 ms (reading and hashing the file, the exchange), and a client process adds a
steady 2.0–2.3 ms to it (w3 minima against the unpinned harness; nodes.rs's harness figure was
taken under more load than its w3 run).

What the server buys at the editor's path, same window (w3, medians): 1.35–1.49x against
`chloro --check`, 1.64–1.96x against `chloro --check --unit-cache`, 1.35–1.76x for the
rust-analyzer form (`--stdin`), and 5.6–13x against rustfmt. On a one-item edit of a large
file the client with the unit cache answers in 7.6–30.4 ms against 14.3–55.7 ms for the CLI
with its unit cache file and 69.3–239.0 ms for rustfmt.

T4's own rule reads "if the median is not below 1 ms or RSS grows, the mode is not
shipped": the median is not below 1 ms where it decides, and RSS grows with every save of
new content until the caches' bounds. The branch stays as it is, for the integrator: exact
(identity, all gates), opt-in, and the server half of an editor integration whose other
half (a client that is already running: an LSP front end, or an extension that speaks the
socket) is what would reach the sub-millisecond figures. Nothing in it is the default, and
the CLI's own path is unchanged.

## Follow-ups

1. **The client's start-up** (the deciding floor). Measure what a minimal forwarding client
   costs: a separate small binary without the formatter, BLAKE3 or facet (`exec`, connect,
   write, read), statically linked; the hyperfine rows above bound what it can save (the
   `--help` row is the current process floor). An editor that keeps its own process (an LSP
   `textDocument/formatting` front end, or an extension that speaks the socket) is the only
   way to the resident client's numbers.
2. **Lifecycle.** Starting the server from the first `--via-server` (detached, its errors
   logged in the cache directory), `SIGTERM` handling that writes the caches, systemd/launchd
   socket activation, the socket in `$XDG_RUNTIME_DIR` by default.
3. **Windows.** Named pipes (`\\.\pipe\chloro-<hash>`), the same framing.
4. **Editor integration.** rust-analyzer's `rustfmt.overrideCommand` with `chloro --stdin
   --via-server --unit-cache` works today (the `client-stdin-units` rows); a documented
   recipe, then an LSP front end.
5. **Memory.** An age-out of unit records not seen for some generations (a superseded edit
   is never looked up again), a smaller in-memory bound for the server than the file's
   64 MiB, and `malloc_trim` after multi-threaded requests.
6. **Concurrency.** A worker pool or a queue per path, so a whole-tree request does not hold
   a save.
7. **Keeping the parse.** The tree per path and an item-level reparse (the theory's last
   column) would remove the whole-file parse that every request still pays.
