use std::process::Command;

/// Formats `code` with the `rustfmt` on `PATH` (rustfmt 1.9.0, the reference release) at
/// edition 2024.
pub fn format_with_rustfmt(code: &str) -> Result<String, String> {
    format_with_rustfmt_command(code, &["rustfmt".to_owned()], "2024")
}

/// Formats `code` with the rustfmt started by `command` (program and arguments, e.g.
/// `rustup run 1.88.0 rustfmt`) at `edition`. An error carries rustfmt's stderr; rustfmt
/// writes nothing on a parse error or a panic, so the caller takes the input as the
/// expected output then.
pub fn format_with_rustfmt_command(
    code: &str,
    command: &[String],
    edition: &str,
) -> Result<String, String> {
    let (program, args) = command.split_first().ok_or("empty rustfmt command")?;
    let mut child = Command::new(program)
        .args(args)
        .arg("--edition")
        .arg(edition)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {program}: {}", e))?;

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .ok_or("Failed to open stdin")?
        .write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write to stdin: {}", e))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for rustfmt: {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from rustfmt: {}", e))
    } else {
        Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// The rustfmt releases other than 1.9.0 to generate reference output for, from
/// `CHLORO_RUSTFMT_VERSIONS`: a comma-separated list of `<version>=<toolchain>` pairs, e.g.
/// `1.8.0=1.88.0`, each run as `rustup run <toolchain> rustfmt`. Unset: empty.
pub fn rustfmt_versions() -> Vec<(String, Vec<String>)> {
    std::env::var("CHLORO_RUSTFMT_VERSIONS")
        .ok()
        .into_iter()
        .flat_map(|v| v.split(',').map(str::to_owned).collect::<Vec<_>>())
        .filter(|entry| !entry.trim().is_empty())
        .map(|entry| {
            let (version, toolchain) = entry.split_once('=').unwrap_or_else(|| {
                panic!("CHLORO_RUSTFMT_VERSIONS entry `{entry}` is not version=toolchain")
            });
            (
                version.trim().to_owned(),
                vec![
                    "rustup".to_owned(),
                    "run".to_owned(),
                    toolchain.trim().to_owned(),
                    "rustfmt".to_owned(),
                ],
            )
        })
        .collect()
}
