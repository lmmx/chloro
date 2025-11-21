use chloro_core::format_source;

#[test]
fn test_simple_use() {
    let input = "use std::io;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_use_with_path() {
    let input = "use std::collections::HashMap;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_public_use() {
    let input = "pub use crate::utils;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_use_with_alias() {
    let input = "use std::io::Result as IoResult;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_use_glob() {
    let input = "use std::prelude::*;";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_use_multiple_items() {
    let input = "use std::io::{Read, Write, BufReader};";
    insta::assert_snapshot!(format_source(input));
}

#[test]
fn test_use_nested() {
    let input = "use std::{io, fs, collections::HashMap};";
    insta::assert_snapshot!(format_source(input));
}
