use super::*;
use insta::assert_snapshot;

#[test]
fn format_simple_macro_call() {
    let input = r#"println!("hello world");"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"println!("hello world");"#);
}

#[test]
fn format_macro_call_with_braces() {
    let input = r#"vec! { 1, 2, 3 };"#;
    let output = format_source(input);
    assert_snapshot!(output, @"vec! { 1, 2, 3 }");
}

#[test]
fn format_config_data_macro() {
    let input = r#"config_data! {
    /// Docs for the config
    global: struct GlobalDefaultConfigData <- GlobalConfigInput -> {
        /// Warm up caches on project load.
        cachePriming_enable: bool = true,

        /// How many worker threads.
        cachePriming_numThreads: NumThreads = NumThreads::Physical,
    }
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    config_data! {
        /// Docs for the config
        global: struct GlobalDefaultConfigData <- GlobalConfigInput -> {
            /// Warm up caches on project load.
            cachePriming_enable: bool = true,

            /// How many worker threads.
            cachePriming_numThreads: NumThreads = NumThreads::Physical,
        }
    }
    ");
}

#[test]
fn format_macro_call_preserves_content() {
    // Macro internals should be preserved exactly
    let input = r#"macro_with_custom_syntax! {
    $foo => $bar,
    @special => @other,
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    macro_with_custom_syntax! {
        $foo => $bar,
        @special => @other,
    }
    ");
}

#[test]
fn format_macro_call_with_doc_comments() {
    let input = r#"/// This macro does something important
/// It has multiple doc comment lines
config_data! {
    field: Type = default,
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    /// This macro does something important
    /// It has multiple doc comment lines
    config_data! {
        field: Type = default,
    }
    "#);
}

#[test]
fn format_macro_call_with_attributes() {
    let input = r#"#[cfg(feature = "foo")]
#[allow(unused)]
some_macro! {
    content here
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    #[cfg(feature = "foo")]
    #[allow(unused)]
    some_macro! {
        content here
    }
    "#);
}

#[test]
fn format_multiple_macro_calls_with_blank_lines() {
    let input = r#"first_macro! { a }

second_macro! { b }"#;
    let output = format_source(input);
    // Should preserve blank line between macro calls
    assert_snapshot!(output, @r"
    first_macro! { a }

    second_macro! { b }
    ");
}

#[test]
fn format_macro_call_after_function() {
    let input = r#"fn foo() {}
some_macro! { content }"#;
    let output = format_source(input);
    // Should have blank line between function and macro call
    assert_snapshot!(output, @r"
    fn foo() {
    }

    some_macro! { content }
    ");
}

#[test]
fn format_function_after_macro_call() {
    let input = r#"some_macro! { content }
fn foo() {}"#;
    let output = format_source(input);
    // Should have blank line between macro call and function
    assert_snapshot!(output, @r"
    some_macro! { content }

    fn foo() {
    }
    ");
}

#[test]
fn format_macro_rules_preserved() {
    let input = r#"macro_rules! my_macro {
    ($x:expr) => {
        println!("{}", $x);
    };
}"#;
    let output = format_source(input);
    assert_snapshot!(output, @r#"
    macro_rules! my_macro {
        ($x:expr) => {
            println!("{}", $x);
        };
    }
    "#);
}

#[test]
fn format_mixed_macros_and_items() {
    let input = r#"use std::io;

config_data! {
    /// Global configs
    global: struct GlobalConfig {
        field: bool = true,
    }
}

fn helper() {}

config_data! {
    /// Local configs
    local: struct LocalConfig {
        other: i32 = 42,
    }
}

struct Foo;"#;
    let output = format_source(input);
    assert_snapshot!(output, @r"
    use std::io;

    config_data! {
        /// Global configs
        global: struct GlobalConfig {
            field: bool = true,
        }
    }

    fn helper() {
    }

    config_data! {
        /// Local configs
        local: struct LocalConfig {
            other: i32 = 42,
        }
    }

    struct Foo;
    ");
}
// // This would be a good test case:
//
// config_data! {
//     /// Docs for the config
//     global : struct GlobalDefaultConfigData <- GlobalConfigInput -> {
//         /// Warm up caches
//         cachePriming_enable: bool = true,
//
//         /// How many worker threads.
//         cachePriming_numThreads: NumThreads = NumThreads::Physical,
//     }
// }
// config_data![];
// config_data! {}
// config_data!();
