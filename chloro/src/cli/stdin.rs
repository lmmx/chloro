//! `--stdin`: format standard input to standard output, for editors, as `rustfmt` does with
//! no file. The configuration is that of the file `--stdin-filepath` names, or, as rustfmt
//! finds it, of the current directory. Neither cache is used.

use super::args::Args;
use super::project::Project;
use super::report::render_parse_error;
use super::worker::PARALLEL_ITEMS_MIN_BYTES;
use chloro_core::{
    is_unchanged, try_format_source_with_config, try_format_source_with_config_parallel,
};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Formats standard input under `args`, and returns the exit status: 0 when it is formatted
/// (and, with `--check`, when it was already); 1 when it does not parse (the error is on
/// standard error and nothing is written to standard output, as rustfmt does), with
/// `--check` when it needs formatting, and on a usage or I/O error.
pub fn format_stdin(args: &Args, project: &mut Project) -> i32 {
    let fail = |message: String| {
        eprintln!("Error: {message}");
        1
    };
    if !args.stdin {
        return fail("--stdin-filepath is given without --stdin".to_owned());
    }
    if args.write {
        return fail("--write cannot be used with --stdin: there is no file to write".to_owned());
    }
    if args.sources != ["."] {
        return fail("--stdin takes no sources".to_owned());
    }
    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        return fail(format!("cannot read standard input: {e}"));
    }
    let (path, label) = match args.stdin_filepath.as_str() {
        "" => match std::env::current_dir().and_then(|dir| dir.canonicalize()) {
            // The configuration of a file in the current directory.
            Ok(dir) => (dir.join("stdin.rs"), "<stdin>".to_owned()),
            Err(e) => return fail(format!("cannot read the current directory: {e}")),
        },
        given => (absolute(Path::new(given)), given.to_owned()),
    };
    let config = match project.config_of(&path) {
        Ok(id) => &project.configs()[id],
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let threads = args.thread_count();
    let formatted = if threads > 1 && input.len() >= PARALLEL_ITEMS_MIN_BYTES {
        try_format_source_with_config_parallel(&input, config, threads)
    } else {
        try_format_source_with_config(&input, config)
    };
    let formatted = match formatted {
        Ok(formatted) => formatted,
        Err(e) => {
            eprint!("{}", render_parse_error(&label, &input, &e));
            return 1;
        }
    };
    if args.check {
        if is_unchanged(&input, &formatted, config) {
            return 0;
        }
        eprintln!("Error: {label} needs formatting");
        return 1;
    }
    match io::stdout().write_all(formatted.as_bytes()) {
        Ok(()) => 0,
        Err(e) => fail(format!("cannot write standard output: {e}")),
    }
}

/// `path` made absolute against the current directory, with its directory canonical when
/// it exists: the path need not name an existing file (a new editor buffer).
fn absolute(path: &Path) -> PathBuf {
    let path = std::env::current_dir().map_or_else(|_| path.to_path_buf(), |dir| dir.join(path));
    match (
        path.parent().and_then(|dir| dir.canonicalize().ok()),
        path.file_name(),
    ) {
        (Some(dir), Some(name)) => dir.join(name),
        _ => path,
    }
}
