pub mod config;
pub(crate) mod node;
pub mod printer;

use ra_ap_syntax::{AstNode, Edition, SourceFile};

/// Format Rust source code with canonical style.
pub fn format_source(source: &str) -> String {
    let parse = SourceFile::parse(source, Edition::CURRENT);
    let root = parse.tree();

    let mut output = String::with_capacity(source.len());
    node::format_node(root.syntax(), &mut output, 0);
    output
}

/// Write indentation to buffer
pub(crate) fn write_indent(buf: &mut String, indent: usize) {
    for _ in 0..indent {
        buf.push(' ');
    }
}
