#![allow(unreachable_pub)]
//! Grammar for the command-line arguments.

use std::{path::PathBuf, str::FromStr};

use ide_ssr::{SsrPattern, SsrRule};

use crate::cli::Verbosity;

/// LSP server for the Rust programming language.
///
/// Subcommands and their flags do not provide any stability guarantees and may be removed or
/// changed without notice. Top-level flags that are not marked as [Unstable] provide
/// backwards-compatibility and may be relied on.
/// Verbosity level, can be repeated multiple times.
/// Verbosity level.
/// Log to the specified file instead of stderr.
/// Flush log records to the file immediately.
/// [Unstable] Wait until a debugger is attached to (requires debug build).
/// Print version.
/// Dump a LSP config JSON schema.
/// Parse stdin.
/// Suppress printing.
/// Parse stdin and print the list of symbols.
/// Highlight stdin as html.
/// Enable rainbow highlighting of identifiers.
/// Batch typecheck project and print summary statistics
/// Directory with Cargo.toml or rust-project.json.
/// Randomize order in which crates, modules, and items are processed.
/// Run type inference in parallel.
/// Only analyze items matching this path.
/// Also analyze all dependencies.
/// Don't load sysroot crates (`std`, `core` & friends).
/// Don't set #[cfg(test)].
/// Don't run build scripts or load `OUT_DIR` values by running `cargo check` before analysis.
/// Don't expand proc macros.
/// Run the proc-macro-srv binary at the specified path.
/// Skip body lowering.
/// Skip type inference.
/// Skip lowering to mir
/// Skip data layout calculation
/// Skip const evaluation
/// Runs several IDE features after analysis, including semantics highlighting, diagnostics
/// and annotations. This is useful for benchmarking the memory usage on a project that has
/// been worked on for a bit in a longer running session.
/// Run term search on all the tail expressions (of functions, block, if statements etc.)
/// Validate term search by running `cargo check` on every response.
/// Note that this also temporarily modifies the files on disk, use with caution!
/// Run unit tests of the project using mir interpreter
/// Directory with Cargo.toml or rust-project.json.
/// Run unit tests of the project using mir interpreter
/// Directory with Cargo.toml.
/// Only run tests with filter as substring
/// Directory with Cargo.toml or rust-project.json.
/// Don't run build scripts or load `OUT_DIR` values by running `cargo check` before analysis.
/// Don't expand proc macros.
/// Run the proc-macro-srv binary at the specified path.
/// The minimum severity.
/// Report unresolved references
/// Directory with Cargo.toml or rust-project.json.
/// Don't run build scripts or load `OUT_DIR` values by running `cargo check` before analysis.
/// Don't expand proc macros.
/// Run the proc-macro-srv binary at the specified path.
/// Prime caches, as rust-analyzer does typically at startup in interactive sessions.
/// Directory with Cargo.toml or rust-project.json.
/// Don't run build scripts or load `OUT_DIR` values by running `cargo check` before analysis.
/// Don't expand proc macros.
/// Run the proc-macro-srv binary at the specified path.
/// The number of threads to use. Defaults to the number of physical cores.
/// A structured search replace rule (`$a.foo($b) ==>> bar($a, $b)`)
/// A structured search replace pattern (`$a.foo($b)`)
/// Prints debug information for any nodes with source exactly equal to snippet.
/// Exclude code from vendored libraries from the resulting index.
/// The output path where the SCIP file will be written to. Defaults to `index.scip`.
/// A path to an json configuration file that can be used to customize cargo behavior.
/// Exclude code from vendored libraries from the resulting index.
#[derive(Debug)]
pub struct RustAnalyzer {
    pub verbose: u32,
    pub quiet: bool,
    pub log_file: Option<PathBuf>,
    pub no_log_buffering: bool,
    pub wait_dbg: bool,
    pub subcommand: RustAnalyzerCmd,
}

#[derive(Debug)]
pub enum RustAnalyzerCmd {
    LspServer(LspServer),
    Parse(Parse),
    Symbols(Symbols),
    Highlight(Highlight),
    AnalysisStats(AnalysisStats),
    RunTests(RunTests),
    RustcTests(RustcTests),
    Diagnostics(Diagnostics),
    UnresolvedReferences(UnresolvedReferences),
    PrimeCaches(PrimeCaches),
    Ssr(Ssr),
    Search(Search),
    Lsif(Lsif),
    Scip(Scip),
}

#[derive(Debug)]
pub struct LspServer {
    pub version: bool,
    pub print_config_schema: bool,
}

#[derive(Debug)]
pub struct Parse {
    pub no_dump: bool,
}

#[derive(Debug)]
pub struct Symbols;

#[derive(Debug)]
pub struct Highlight {
    pub rainbow: bool,
}

#[derive(Debug)]
pub struct AnalysisStats {
    pub path: PathBuf,
    pub output: Option<OutputFormat>,
    pub randomize: bool,
    pub parallel: bool,
    pub only: Option<String>,
    pub with_deps: bool,
    pub no_sysroot: bool,
    pub no_test: bool,
    pub disable_build_scripts: bool,
    pub disable_proc_macros: bool,
    pub proc_macro_srv: Option<PathBuf>,
    pub skip_lowering: bool,
    pub skip_inference: bool,
    pub skip_mir_stats: bool,
    pub skip_data_layout: bool,
    pub skip_const_eval: bool,
    pub run_all_ide_things: bool,
    pub run_term_search: bool,
    pub validate_term_search: bool,
}

#[derive(Debug)]
pub struct RunTests {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct RustcTests {
    pub rustc_repo: PathBuf,
    pub filter: Option<String>,
}

#[derive(Debug)]
pub struct Diagnostics {
    pub path: PathBuf,
    pub disable_build_scripts: bool,
    pub disable_proc_macros: bool,
    pub proc_macro_srv: Option<PathBuf>,
    pub severity: Option<Severity>,
}

#[derive(Debug)]
pub struct UnresolvedReferences {
    pub path: PathBuf,
    pub disable_build_scripts: bool,
    pub disable_proc_macros: bool,
    pub proc_macro_srv: Option<PathBuf>,
}

#[derive(Debug)]
pub struct PrimeCaches {
    pub path: PathBuf,
    pub disable_build_scripts: bool,
    pub disable_proc_macros: bool,
    pub proc_macro_srv: Option<PathBuf>,
    pub num_threads: Option<usize>,
}

#[derive(Debug)]
pub struct Ssr {
    pub rule: Vec<SsrRule>,
}

#[derive(Debug)]
pub struct Search {
    pub pattern: Vec<SsrPattern>,
    pub debug: Option<String>,
}

#[derive(Debug)]
pub struct Lsif {
    pub path: PathBuf,
    pub exclude_vendored_libraries: bool,
}

#[derive(Debug)]
pub struct Scip {
    pub path: PathBuf,
    pub output: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub exclude_vendored_libraries: bool,
}

impl RustAnalyzer {
    #[allow(dead_code)]
    pub fn from_env_or_exit() -> Self {
        Self::from_env_or_exit_()
    }

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Csv,
}

impl RustAnalyzer {
    pub fn verbosity(&self) -> Verbosity {
        if self.quiet {
            return Verbosity::Quiet;
        }
        match self.verbose {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Spammy,
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "csv" => Ok(Self::Csv),
            _ => Err(format!("unknown output format `{s}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Weak,
    Warning,
    Error,
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &*s.to_ascii_lowercase() {
            "weak" => Ok(Self::Weak),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(format!("unknown severity `{s}`")),
        }
    }
}
