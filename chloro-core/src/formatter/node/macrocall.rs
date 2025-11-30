// chloro-core/src/formatter/node/macrocall.rs
use ra_ap_syntax::{
    AstNode, AstToken, SyntaxNode,
    ast::{self, HasAttrs, HasDocComments},
};

use crate::formatter::write_indent;

/// Format a macro call (e.g., `config_data! { ... }` or `println!("hello")`)
///
/// Macro calls are preserved as-is since we cannot safely reformat their internals
/// without understanding the macro's syntax.
pub fn format_macro_call(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let macro_call = match ast::MacroCall::cast(node.clone()) {
        Some(m) => m,
        None => return,
    };

    // Format doc comments using HasDocComments trait
    for doc_comment in macro_call.doc_comments() {
        write_indent(buf, indent);
        buf.push_str(doc_comment.text().trim());
        buf.push('\n');
    }

    // Format attributes using HasAttrs trait
    for attr in macro_call.attrs() {
        write_indent(buf, indent);
        buf.push_str(&attr.syntax().text().to_string());
        buf.push('\n');
    }

    write_indent(buf, indent);

    // Find where the actual macro call starts (after doc comments and attributes)
    // by looking at the path, which is the first "real" part of the macro call
    if let Some(path) = macro_call.path() {
        let path_start = path.syntax().text_range().start();
        let node_start = node.text_range().start();
        let offset = usize::from(path_start - node_start);

        // Get everything from the path onwards (preserves spacing before delimiters)
        let full_text = node.text().to_string();
        buf.push_str(&full_text[offset..]);
    } else {
        // Fallback: output the whole thing
        buf.push_str(&node.text().to_string());
    }

    buf.push('\n');
}
