use super::*;
use insta::assert_snapshot;

use ra_ap_syntax::ast::HasArgList;

#[test]
fn debug_method_call_arg_formatting() {
    let input = r#"fn foo() {
    acc.push(
        UnresolvedModule {
            decl: InFile::new(file_id, decl),
            candidates: candidates.clone(),
        }
        .into(),
    )
}"#;

    let parse = ra_ap_syntax::SourceFile::parse(input, ra_ap_syntax::Edition::CURRENT);
    let root = parse.syntax_node();

    use ra_ap_syntax::{AstNode, NodeOrToken, SyntaxKind, ast};

    // Find acc.push()
    for node in root.descendants() {
        if node.kind() == SyntaxKind::METHOD_CALL_EXPR {
            if let Some(method) = ast::MethodCallExpr::cast(node.clone()) {
                if let Some(name) = method.name_ref() {
                    if name.text() == "push" {
                        eprintln!("=== acc.push() ===");

                        if let Some(arg_list) = method.arg_list() {
                            // Check if there's a newline after L_PAREN
                            let mut after_lparen = false;
                            for child in arg_list.syntax().children_with_tokens() {
                                match &child {
                                    NodeOrToken::Token(t) if t.kind() == SyntaxKind::L_PAREN => {
                                        after_lparen = true;
                                    }
                                    NodeOrToken::Token(t)
                                        if after_lparen && t.kind() == SyntaxKind::WHITESPACE =>
                                    {
                                        let has_newline = t.text().contains('\n');
                                        eprintln!(
                                            "Whitespace after L_PAREN: {:?}, has_newline: {}",
                                            t.text(),
                                            has_newline
                                        );
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // What does format_method_call_expr return for this?
                        // It should return None because the receiver (acc) is a PATH_EXPR
                        // No wait, acc is PATH_EXPR which is NOT in the chain check

                        let receiver = method.receiver();
                        eprintln!(
                            "Receiver: {:?}",
                            receiver.as_ref().map(|r| r.syntax().kind())
                        );
                    }
                }
            }
        }
    }

    panic!("Debug output above");
}

#[test]
fn method_call_after_multiline_struct() {
    let input = r#"fn foo() {
    Foo {
        a: 1,
        b: 2,
    }
    .bar()
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        Foo {
            a: 1,
            b: 2,
        }
        .bar()
    }
    "#);
}

#[test]
fn method_call_after_inline_struct() {
    let input = r#"fn foo() { Foo { a: 1 }.bar() }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        Foo { a: 1 }.bar()
    }
    "#);
}

#[test]
fn into_after_multiline_struct_in_arg() {
    let input = r#"fn foo() {
    acc.push(
        UnresolvedModule {
            decl: InFile::new(file_id, decl),
            candidates: candidates.clone(),
        }
        .into(),
    )
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    fn foo() {
        acc.push(
            UnresolvedModule {
                decl: InFile::new(file_id, decl),
                candidates: candidates.clone(),
            }
            .into(),
        )
    }
    "#);
}
