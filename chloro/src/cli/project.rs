//! The configuration of each file, found as `cargo fmt` finds it: the `rustfmt.toml` that
//! rustfmt reads for the file, the edition `cargo fmt` passes for the file's crate, and the
//! options of `--config` over both.
//!
//! - **`rustfmt.toml`.** For each file, rustfmt reads `.rustfmt.toml` or `rustfmt.toml`, in
//!   that order, from the file's directory or the nearest of its parents that has one; failing
//!   that, from the home directory, then from `rustfmt/` in the user's configuration directory
//!   (rustfmt's `get_toml_path` and `resolve_project_file`). `--config-path` names the file
//!   (or a directory holding one) for every file instead.
//! - **The edition.** `cargo fmt` runs rustfmt with `--edition` set to the edition of each
//!   package, which overrides the file's `edition`: the `[package] edition` of the nearest
//!   `Cargo.toml`, 2015 when it has none, and `[workspace.package] edition` for
//!   `edition.workspace = true`. A file whose nearest `Cargo.toml` has no `[package]` gets no
//!   edition from it.
//! - **`--config`** is applied last, and `--isolated` reads neither file.
//!
//! A `rustfmt.toml` that does not parse, or that gives an option a value it does not take,
//! is an error before any file is formatted, as rustfmt refuses it; an unstable or unknown
//! option is a warning, printed once per file. A `Cargo.toml` that cannot be read for its
//! edition is a warning, and gives no edition.

use super::args::Args;
use chloro_core::{Config, FileValue};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::de::{DeTable, DeValue};

/// rustfmt's configuration file names, in the order it looks for them in a directory.
const CONFIG_FILE_NAMES: [&str; 2] = [".rustfmt.toml", "rustfmt.toml"];

/// The configurations of a run's files, and what was read to make them.
pub struct Project {
    isolated: bool,
    /// `--config-path`, resolved to a file.
    config_path: Option<PathBuf>,
    /// The options of `--config`, applied last.
    cli: Vec<(String, String)>,
    verbose: bool,
    /// The distinct configurations, by index.
    configs: Vec<Config>,
    /// Index into `configs` by the key bytes of a configuration.
    ids: HashMap<Vec<u8>, usize>,
    /// Index into `configs` by (configuration file, edition).
    resolved: HashMap<(Option<PathBuf>, Option<String>), usize>,
    /// The nearest configuration file of a directory and of its parents.
    config_files: HashMap<PathBuf, Option<PathBuf>>,
    /// The configuration file outside the tree: the home or the configuration directory's.
    global: Option<Option<PathBuf>>,
    /// The options of each configuration file read, over the defaults.
    file_configs: HashMap<PathBuf, Config>,
    /// The nearest `Cargo.toml` of a directory and of its parents.
    manifests: HashMap<PathBuf, Option<PathBuf>>,
    /// The edition of each `Cargo.toml` read.
    editions: HashMap<PathBuf, Option<String>>,
}

impl Project {
    /// The project of a run with `args`.
    ///
    /// # Errors
    ///
    /// `--config-path` names nothing, or a directory without a configuration file, or is
    /// given with `--isolated`; or `--config` is malformed.
    pub fn new(args: &Args) -> Result<Project, String> {
        let config_path = match args.config_path.as_str() {
            "" => None,
            _ if args.isolated => {
                return Err("--isolated and --config-path cannot be used together".to_owned());
            }
            path => Some(resolve_config_path(Path::new(path))?),
        };
        Ok(Project {
            isolated: args.isolated,
            config_path,
            cli: args.config_pairs()?,
            verbose: args.verbose,
            configs: Vec::new(),
            ids: HashMap::new(),
            resolved: HashMap::new(),
            config_files: HashMap::new(),
            global: None,
            file_configs: HashMap::new(),
            manifests: HashMap::new(),
            editions: HashMap::new(),
        })
    }

    /// The distinct configurations of the files resolved so far.
    pub fn configs(&self) -> &[Config] {
        &self.configs
    }

    /// The index into [`configs`](Self::configs) of the configuration of `file`, a canonical
    /// path.
    ///
    /// # Errors
    ///
    /// The configuration file that applies to `file` does not parse or sets an invalid value.
    pub fn config_of(&mut self, file: &Path) -> Result<usize, String> {
        let dir = file.parent().unwrap_or(Path::new("/"));
        let (config_file, edition) = if self.isolated {
            (None, None)
        } else {
            let config_file = match &self.config_path {
                Some(path) => Some(path.clone()),
                None => self.config_file_for(dir),
            };
            (config_file, self.edition_for(dir))
        };
        let key = (config_file, edition);
        if let Some(&id) = self.resolved.get(&key) {
            return Ok(id);
        }
        let mut config = match &key.0 {
            Some(path) => self.file_config(path)?,
            None => Config::default(),
        };
        // As `cargo fmt`'s `--edition`, which overrides the file's; both it and `--config`
        // were checked before.
        if let Some(edition) = &key.1 {
            config.set("edition", edition)?;
        }
        for (k, v) in &self.cli {
            config.set(k, v)?;
        }
        let bytes = config.key_bytes();
        let id = match self.ids.get(&bytes) {
            Some(&id) => id,
            None => {
                self.configs.push(config);
                self.ids.insert(bytes, self.configs.len() - 1);
                self.configs.len() - 1
            }
        };
        self.resolved.insert(key, id);
        Ok(id)
    }

    /// The configuration file rustfmt reads for a file in `dir`.
    fn config_file_for(&mut self, dir: &Path) -> Option<PathBuf> {
        if let Some(found) = nearest(&mut self.config_files, dir, config_file_in) {
            return Some(found);
        }
        self.global
            .get_or_insert_with(|| {
                let home = std::env::home_dir().and_then(|home| config_file_in(&home));
                home.or_else(|| config_file_in(&user_config_dir()?.join("rustfmt")))
            })
            .clone()
    }

    /// The options of the configuration file `path` over the defaults, read once.
    fn file_config(&mut self, path: &Path) -> Result<Config, String> {
        if let Some(config) = self.file_configs.get(path) {
            return Ok(config.clone());
        }
        let failed = |detail: String| {
            format!(
                "The file `{}` failed to parse.\nError details: {detail}",
                path.display()
            )
        };
        let text = fs::read_to_string(path).map_err(|e| failed(e.to_string()))?;
        let table = DeTable::parse(&text).map_err(|e| failed(e.to_string()))?;
        let mut config = Config::default();
        for (key, value) in table.get_ref() {
            let value = file_value(value.get_ref(), &text[value.span()]);
            match config.set_from_file(key.get_ref(), value) {
                Ok(Some(warning)) => eprintln!("Warning: {warning}"),
                Ok(None) => {}
                Err(e) => return Err(failed(e)),
            }
        }
        if self.verbose {
            eprintln!("Using rustfmt config file {}", path.display());
        }
        self.file_configs.insert(path.to_path_buf(), config.clone());
        Ok(config)
    }

    /// The edition `cargo fmt` passes for a file in `dir`: that of the package whose
    /// `Cargo.toml` is nearest.
    fn edition_for(&mut self, dir: &Path) -> Option<String> {
        let manifest = nearest(&mut self.manifests, dir, |d| {
            let path = d.join("Cargo.toml");
            path.is_file().then_some(path)
        })?;
        if let Some(edition) = self.editions.get(&manifest) {
            return edition.clone();
        }
        // An edition the formatter does not take is not passed on.
        let checked = package_edition(&manifest).and_then(|edition| match edition {
            Some(e) if Config::default().set("edition", &e).is_err() => {
                Err(format!("unsupported edition `{e}`"))
            }
            edition => Ok(edition),
        });
        let edition = match checked {
            Ok(edition) => edition,
            Err(e) => {
                eprintln!(
                    "Warning: no edition read from `{}`: {e}",
                    manifest.display()
                );
                None
            }
        };
        if self.verbose
            && let Some(edition) = &edition
        {
            eprintln!("Edition {edition} from {}", manifest.display());
        }
        self.editions.insert(manifest, edition.clone());
        edition
    }
}

/// `--config-path` as rustfmt resolves it: a file, or the configuration file of a directory.
fn resolve_config_path(path: &Path) -> Result<PathBuf, String> {
    let not_found = || {
        format!(
            "unable to find a config file for the given path: `{}`",
            path.display()
        )
    };
    if !path.exists() {
        return Err(not_found());
    }
    if path.is_dir() {
        return config_file_in(path).ok_or_else(not_found);
    }
    path.canonicalize().map_err(|e| e.to_string())
}

/// The configuration file of `dir` itself, if it has one.
fn config_file_in(dir: &Path) -> Option<PathBuf> {
    CONFIG_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// What `find` finds in `dir` or the nearest of its parents, recorded in `memo` for every
/// directory on the way.
fn nearest(
    memo: &mut HashMap<PathBuf, Option<PathBuf>>,
    dir: &Path,
    find: impl Fn(&Path) -> Option<PathBuf>,
) -> Option<PathBuf> {
    let mut visited = Vec::new();
    let mut found = None;
    for d in dir.ancestors() {
        if let Some(known) = memo.get(d) {
            found = known.clone();
            break;
        }
        visited.push(d.to_path_buf());
        if let Some(path) = find(d) {
            found = Some(path);
            break;
        }
    }
    for d in visited {
        memo.insert(d, found.clone());
    }
    found
}

/// The user's configuration directory, as rustfmt's `dirs::config_dir` gives it.
fn user_config_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        Some(std::env::home_dir()?.join("Library/Application Support"))
    } else {
        var("XDG_CONFIG_HOME")
            .filter(|dir| dir.is_absolute())
            .or_else(|| Some(std::env::home_dir()?.join(".config")))
    }
}

/// A TOML value as [`Config::set_from_file`] takes it; `text` is the value as written.
fn file_value<'a>(value: &'a DeValue<'_>, text: &'a str) -> FileValue<'a> {
    match value {
        DeValue::String(s) => FileValue::String(s),
        DeValue::Boolean(b) => FileValue::Boolean(*b),
        DeValue::Integer(n) => match i64::from_str_radix(n.as_str(), n.radix()) {
            Ok(n) => FileValue::Integer(n),
            Err(_) => FileValue::Other(text),
        },
        _ => FileValue::Other(text),
    }
}

/// The edition of the package of the manifest `path`: its `[package] edition`, 2015 when it
/// has none, or its workspace's for `edition.workspace = true`; `None` for a manifest
/// without `[package]` (a virtual workspace).
fn package_edition(path: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let manifest = DeTable::parse(&text).map_err(|e| e.to_string())?;
    let Some(package) = table(manifest.get_ref(), "package") else {
        return Ok(None);
    };
    match package.get("edition").map(|v| v.get_ref()) {
        None => Ok(Some("2015".to_owned())),
        Some(DeValue::String(edition)) => Ok(Some(edition.to_string())),
        Some(DeValue::Table(t))
            if matches!(
                t.get("workspace").map(|v| v.get_ref()),
                Some(DeValue::Boolean(true))
            ) =>
        {
            let dir = path.parent().unwrap_or(Path::new("/"));
            let root = match package.get("workspace").map(|v| v.get_ref()) {
                Some(DeValue::String(root)) => Some(dir.join(root.as_ref()).join("Cargo.toml")),
                _ => dir.ancestors().map(|d| d.join("Cargo.toml")).find(|m| {
                    fs::read_to_string(m).is_ok_and(|text| {
                        DeTable::parse(&text).is_ok_and(|t| t.get_ref().contains_key("workspace"))
                    })
                }),
            };
            let root = root.ok_or("`edition.workspace = true` outside a workspace")?;
            workspace_edition(&root)
                .map(Some)
                .ok_or_else(|| format!("no `workspace.package.edition` in `{}`", root.display()))
        }
        Some(_) => Err("`package.edition` is not a string".to_owned()),
    }
}

/// `[workspace.package] edition` of the workspace manifest `root`.
fn workspace_edition(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root).ok()?;
    let manifest = DeTable::parse(&text).ok()?;
    let package = table(table(manifest.get_ref(), "workspace")?, "package")?;
    match package.get("edition")?.get_ref() {
        DeValue::String(edition) => Some(edition.to_string()),
        _ => None,
    }
}

/// The table `key` of `table`.
fn table<'a, 'i>(table: &'a DeTable<'i>, key: &str) -> Option<&'a DeTable<'i>> {
    match table.get(key)?.get_ref() {
        DeValue::Table(t) => Some(t),
        _ => None,
    }
}
