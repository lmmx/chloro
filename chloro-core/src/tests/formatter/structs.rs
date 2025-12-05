use super::*;
use insta::assert_snapshot;

#[test]
fn preserve_struct_field_default_initializer() {
    let input = r#"struct S { f: f32 = 0.0 }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    struct S {
        f: f32 = 0.0,
    }
    ");
}

#[test]
fn preserve_struct_field_default_initializer_multiline() {
    let input = r#"struct S {
    f: f32 = 0.0,
    g: i32 = 42,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    struct S {
        f: f32 = 0.0,
        g: i32 = 42,
    }
    "#);
}

#[test]
fn preserve_enum_discriminant() {
    let input = r#"enum E {
    A = 1,
    B = 92,
    C = 100,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    enum E {
        A = 1,
        B = 92,
        C = 100,
    }
    "#);
}

#[test]
fn preserve_enum_discriminant_single_variant() {
    let input = r#"enum E { B = 92 }"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    enum E {
        B = 92,
    }
    ");
}

#[test]
fn preserve_enum_mixed_discriminants() {
    let input = r#"enum E {
    A,
    B = 92,
    C,
}
"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    enum E {
        A,
        B = 92,
        C,
    }
    "#);
}
