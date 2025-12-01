use ra_ap_syntax::{AstNode, SyntaxNode, ast};

use crate::formatter::printer::Printer;

pub fn format_macro_call(node: &SyntaxNode, buf: &mut String, indent: usize) {
    let Some(macro_call) = ast::MacroCall::cast(node.clone()) else {
        return;
    };

    buf.doc_comments(&macro_call, indent);
    buf.attrs(&macro_call, indent);
    buf.indent(indent);

    if let Some(path) = macro_call.path() {
        let offset = usize::from(path.syntax().text_range().start() - node.text_range().start());
        buf.push_str(&node.text().to_string()[offset..]);
    } else {
        buf.push_str(&node.text().to_string());
    }
    buf.push('\n');
}
