use super::*;

#[test]
fn format_struct_literal_multiline() {
    let input = "fn module(&self, _db: &dyn DefDatabase) -> ModuleId { ModuleId { krate: self.krate, block: None, local_id: DefMap::ROOT } }";
    let output = format_source(input);

    println!("{}", output);

    // Should expand struct literal to multiple lines
    assert!(output.contains("ModuleId {\n"));
    assert!(output.contains("    krate: self.krate,\n"));
    assert!(output.contains("    block: None,\n"));
    assert!(output.contains("    local_id: DefMap::ROOT,\n"));
}

#[test]
fn format_struct_literal_single_field_inline() {
    let input = "fn foo() -> Point { Point { x: 1 } }";
    let output = format_source(input);

    println!("{}", output);

    // Single field should stay inline
    assert!(output.contains("Point { x: 1 }"));
}
// #[test]
// fn debug_struct_literal_ast() {
//     let input = "fn module(&self, _db: &dyn DefDatabase) -> ModuleId { ModuleId { krate: self.krate, block: None, local_id: DefMap::ROOT } }";
//     let parse = ra_ap_syntax::SourceFile::parse(input, ra_ap_syntax::Edition::CURRENT);
//     let root = parse.syntax_node();
//
//     // Walk the tree
//     fn walk(node: &ra_ap_syntax::SyntaxNode, depth: usize) {
//         println!("{:indent$}{:?}", "", node.kind(), indent = depth * 2);
//         for child in node.children() {
//             walk(&child, depth + 1);
//         }
//     }
//
//     walk(&root, 0);
//     assert!(false);
// }
