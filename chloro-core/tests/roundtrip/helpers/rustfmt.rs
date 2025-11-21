use std::process::Command;

pub fn format_with_rustfmt(code: &str) -> Result<String, String> {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn rustfmt: {}", e))?;

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
