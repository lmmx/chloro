// chloro-core/src/formatter/printer.rs
//! A trait-based printer to reduce repetitive formatting patterns.

use ra_ap_syntax::ast::{AstNode, AstToken, HasAttrs, HasDocComments, HasVisibility};

use crate::formatter::write_indent;

/// Extension trait for String buffers used in formatting.
pub trait Printer {
    /// Write indentation followed by text and a newline.
    fn line(&mut self, indent: usize, text: &str);

    /// Write indentation only.
    fn indent(&mut self, indent: usize);

    /// Write text followed by a newline.
    fn newline(&mut self, text: &str);

    /// Write a blank line.
    fn blank(&mut self);

    /// Write doc comments from a node that implements HasDocComments.
    fn doc_comments<T: HasDocComments>(&mut self, item: &T, indent: usize);

    /// Write attributes from a node that implements HasAttrs.
    fn attrs<T: HasAttrs>(&mut self, item: &T, indent: usize);

    /// Write visibility if present, followed by a space.
    fn visibility<T: HasVisibility>(&mut self, item: &T);

    /// Write doc comments, attributes, indentation, and visibility - the common preamble.
    fn item_preamble<T: HasDocComments + HasAttrs + HasVisibility>(
        &mut self,
        item: &T,
        indent: usize,
    );

    /// Open a brace block: ` {\n`
    fn open_brace(&mut self);

    /// Open a brace block on new line (after where clause): `\n` + indent + `{\n`
    fn open_brace_newline(&mut self, indent: usize);

    /// Close a brace block: indent + `}`
    fn close_brace(&mut self, indent: usize);

    /// Close a brace block with newline: indent + `}\n`
    fn close_brace_ln(&mut self, indent: usize);
}

impl Printer for String {
    fn line(&mut self, indent: usize, text: &str) {
        write_indent(self, indent);
        self.push_str(text);
        self.push('\n');
    }

    fn indent(&mut self, indent: usize) {
        write_indent(self, indent);
    }

    fn newline(&mut self, text: &str) {
        self.push_str(text);
        self.push('\n');
    }

    fn blank(&mut self) {
        self.push('\n');
    }

    fn doc_comments<T: HasDocComments>(&mut self, item: &T, indent: usize) {
        for doc in item.doc_comments() {
            self.line(indent, doc.text().trim());
        }
    }

    fn attrs<T: HasAttrs>(&mut self, item: &T, indent: usize) {
        for attr in item.attrs() {
            self.line(indent, &attr.syntax().text().to_string());
        }
    }

    fn visibility<T: HasVisibility>(&mut self, item: &T) {
        if let Some(vis) = item.visibility() {
            self.push_str(&vis.syntax().text().to_string());
            self.push(' ');
        }
    }

    fn item_preamble<T: HasDocComments + HasAttrs + HasVisibility>(
        &mut self,
        item: &T,
        indent: usize,
    ) {
        self.doc_comments(item, indent);
        self.attrs(item, indent);
        write_indent(self, indent);
        self.visibility(item);
    }

    fn open_brace(&mut self) {
        self.push_str(" {\n");
    }

    fn open_brace_newline(&mut self, indent: usize) {
        self.push('\n');
        write_indent(self, indent);
        self.push_str("{\n");
    }

    fn close_brace(&mut self, indent: usize) {
        write_indent(self, indent);
        self.push('}');
    }

    fn close_brace_ln(&mut self, indent: usize) {
        write_indent(self, indent);
        self.push_str("}\n");
    }
}

/// Collect attributes as a prefix string (for expressions).
pub fn expr_attrs_prefix<T: HasAttrs>(node: &T) -> String {
    let mut result = String::new();
    for attr in node.attrs() {
        result.push_str(&attr.syntax().text().to_string());
        result.push(' ');
    }
    result
}
