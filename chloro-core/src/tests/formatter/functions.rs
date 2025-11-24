use super::*;

#[test]
fn format_function_params_single_line_self_and_param() {
    let input = "pub fn origin(\n    self,\n    db: &dyn HirDatabase,\n) -> CrateOrigin {}";
    let output = format_source(input);
    assert!(output.contains("pub fn origin(self, db: &dyn HirDatabase) -> CrateOrigin {"));
}

#[test]
fn format_function_params_single_line_ref_self() {
    let input =
        "pub fn cfg<'db>(\n    &self,\n    db: &'db dyn HirDatabase,\n) -> &'db CfgOptions {}";
    let output = format_source(input);
    assert!(
        output.contains("pub fn cfg<'db>(&self, db: &'db dyn HirDatabase) -> &'db CfgOptions {")
    );
}

#[test]
fn format_function_params_single_line_multiple_params() {
    let input = "pub fn canonical_path(\n    &self,\n    db: &dyn HirDatabase,\n    edition: Edition,\n) -> Option<String> {}";
    let output = format_source(input);
    assert!(output.contains(
        "pub fn canonical_path(&self, db: &dyn HirDatabase, edition: Edition) -> Option<String> {"
    ));
}

#[test]
fn format_function_params_multi_line_when_too_long() {
    // This should remain multi-line because it exceeds MAX_WIDTH (100)
    let input = "pub fn very_long_function_name_that_makes_this_exceed_width(self, first_parameter: VeryLongTypeName, second_parameter: AnotherLongType) -> Result<(), Error> {}";
    let output = format_source(input);
    // Should have newlines in parameters
    assert!(output.contains(",\n"));
}
