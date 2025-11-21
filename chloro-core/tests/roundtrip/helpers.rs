use chloro_core::format_source;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[ctor::ctor]
fn init_debug() {
    chloro_core::debug::set_debug(true);
}

pub struct ComparisonResult {
    pub original: String,
    pub chloro: String,
    pub rustfmt: String,
}

impl ComparisonResult {
    pub fn show_diff(&self, name: &str) {
        eprintln!();
        eprintln!("============================================================");
        eprintln!("COMPARISON: {}", name);
        eprintln!("============================================================");

        eprintln!();
        eprintln!("--- ORIGINAL ({} bytes) ---", self.original.len());
        eprintln!("{}", &self.original[..self.original.len().min(200)]);
        if self.original.len() > 200 {
            eprintln!("... ({} more bytes)", self.original.len() - 200);
        }

        eprintln!();
        eprintln!("--- CHLORO OUTPUT ({} bytes) ---", self.chloro.len());
        eprintln!("{}", self.chloro);

        eprintln!();
        eprintln!("--- RUSTFMT OUTPUT ({} bytes) ---", self.rustfmt.len());
        eprintln!("{}", self.rustfmt);

        if self.chloro != self.rustfmt {
            eprintln!();
            eprintln!("--- DIFF (chloro vs rustfmt) ---");
            let diff = similar::TextDiff::from_lines(&self.chloro, &self.rustfmt);
            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    similar::ChangeTag::Delete => "- ",
                    similar::ChangeTag::Insert => "+ ",
                    similar::ChangeTag::Equal => "  ",
                };
                eprint!("{}{}", sign, change);
            }
            eprintln!();
            eprintln!("--- END DIFF ---");
        } else {
            eprintln!();
            eprintln!("✓ IDENTICAL OUTPUT");
        }

        eprintln!();
    }
}

pub fn compare_with_rustfmt(code: &str, name: &str) -> ComparisonResult {
    let chloro = format_source(code);

    // Format with rustfmt
    let rustfmt = format_with_rustfmt(code).unwrap_or_else(|e| {
        eprintln!("Warning: rustfmt failed: {}", e);
        code.to_string()
    });

    let result = ComparisonResult {
        original: code.to_string(),
        chloro,
        rustfmt,
    };

    result.show_diff(name);
    result
}

fn format_with_rustfmt(code: &str) -> Result<String, String> {
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

pub fn load_fixture(name: &str) -> String {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("roundtrip")
        .join("fixtures")
        .join(format!("{}.rs", name));

    eprintln!("Loading fixture from: {}", fixture_path.display());

    fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "Fixture not found: {} (error: {})",
            fixture_path.display(),
            e
        )
    })
}
