use chloro_core::format_source;
use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
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
    /// Write comparison files and show diff
    pub fn write_comparison_files(&self, fixture_name: &str) {
        let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("roundtrip")
            .join("output");

        fs::create_dir_all(&output_dir).unwrap();

        // Sanitize fixture name for filename
        let safe_name = fixture_name.replace('/', "_");

        // Write chloro output
        let chloro_path = output_dir.join(format!("{}.chloro.rs", safe_name));
        fs::write(&chloro_path, &self.chloro).unwrap();
        eprintln!("Wrote: {}", chloro_path.display());

        // Write rustfmt output
        let rustfmt_path = output_dir.join(format!("{}.rustfmt.rs", safe_name));
        fs::write(&rustfmt_path, &self.rustfmt).unwrap();
        eprintln!("Wrote: {}", rustfmt_path.display());

        // Generate and write diff
        let diff_content = self.generate_diff_content();
        let diff_path = output_dir.join(format!("{}.diff", safe_name));
        fs::write(&diff_path, &diff_content).unwrap();
        eprintln!("Wrote: {}", diff_path.display());

        // Show diff in terminal
        self.show_diff();
    }

    fn show_diff(&self) {
        let input = InternedInput::new(&self.chloro[..], &self.rustfmt[..]);
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        if diff.count_additions() == 0 && diff.count_removals() == 0 {
            eprintln!("\n=== [IDENTICAL] ===\n");
        } else {
            eprintln!("\n=== DIFF (chloro vs rustfmt) ===");

            let config = UnifiedDiffConfig::default();
            let printer = BasicLineDiffPrinter(&input.interner);
            let unified = diff.unified_diff(&printer, config, &input);

            eprintln!("{}", unified);
            eprintln!("=== END DIFF ===\n");
        }
    }

    fn generate_diff_content(&self) -> String {
        let mut output = String::new();

        output.push_str("COMPARISON DIFF\n");
        output.push_str("============================================================\n\n");

        output.push_str(&format!("Original size: {} bytes\n", self.original.len()));
        output.push_str(&format!("Chloro size:   {} bytes\n", self.chloro.len()));
        output.push_str(&format!("Rustfmt size:  {} bytes\n\n", self.rustfmt.len()));

        let input = InternedInput::new(&self.chloro[..], &self.rustfmt[..]);
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        if diff.count_additions() == 0 && diff.count_removals() == 0 {
            output.push_str("✓ Outputs are IDENTICAL\n");
        } else {
            output.push_str("✗ Outputs DIFFER\n\n");

            let config = UnifiedDiffConfig::default();
            let printer = BasicLineDiffPrinter(&input.interner);
            let unified = diff.unified_diff(&printer, config, &input);

            output.push_str(&format!("{}", unified));
        }

        output
    }
}

pub fn compare_with_rustfmt(code: &str, name: &str) -> ComparisonResult {
    eprintln!();
    eprintln!("============================================================");
    eprintln!("COMPARING: {}", name);
    eprintln!("============================================================");

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

    result.write_comparison_files(name);
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
