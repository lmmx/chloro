//! The chloro binary for the end-to-end tests, isolated from the user's configuration: the
//! home and configuration directories, where chloro looks for a global `rustfmt.toml` as
//! rustfmt does, name a directory that does not exist.

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;

/// `chloro`, as built for the tests, with no global `rustfmt.toml` to find.
pub fn chloro() -> Command {
    let none = concat!(env!("CARGO_TARGET_TMPDIR"), "/no-home");
    let mut cmd = cargo_bin_cmd!("chloro");
    for var in ["HOME", "USERPROFILE", "XDG_CONFIG_HOME", "APPDATA"] {
        cmd.env(var, none);
    }
    cmd
}
