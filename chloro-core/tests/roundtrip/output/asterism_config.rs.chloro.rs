//! Configuration to acknowledge developer preferences as well as set defaults.
//!
//! Specifically, we try to find an asterism.toml, and if present we load settings from there.
//! This provides wrapping width and file extension preferences.
use facet::Facet;

use std::fs;

pub struct Config {
    pub wrap_width: usize,
    pub file_extensions: Vec<String>,
}

impl Config {
    #[must_use]
    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string("asterism.toml") {
            if let Ok(config) = facet_toml::from_str::<Self>(&contents) {
                return config;
            }
        }
        facet_toml::from_str::<Self>("").unwrap()
    }
}
