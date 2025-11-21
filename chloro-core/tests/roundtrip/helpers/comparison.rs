use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use std::fs;
use std::path::PathBuf;

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
        // Reversed: rustfmt -> chloro (shows what chloro is doing wrong)
        let input = InternedInput::new(&self.rustfmt[..], &self.chloro[..]);
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        if diff.count_additions() == 0 && diff.count_removals() == 0 {
            eprintln!("\n=== [IDENTICAL] ===\n");
        } else {
            eprintln!("\n=== DIFF (- rustfmt, + chloro) ===");

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

        // Reversed: rustfmt -> chloro (shows what chloro is doing wrong)
        let input = InternedInput::new(&self.rustfmt[..], &self.chloro[..]);
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        if diff.count_additions() == 0 && diff.count_removals() == 0 {
            output.push_str("✓ Outputs are IDENTICAL\n");
        } else {
            output.push_str("✗ Outputs DIFFER\n\n");
            output.push_str("--- DIFF (- rustfmt, + chloro) ---\n");

            let config = UnifiedDiffConfig::default();
            let printer = BasicLineDiffPrinter(&input.interner);
            let unified = diff.unified_diff(&printer, config, &input);

            output.push_str(&format!("{}", unified));
        }

        output
    }
}
