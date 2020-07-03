extern crate pulldown_cmark;
extern crate dprint_core;
#[macro_use] extern crate lazy_static;
extern crate regex;

pub mod configuration;
mod format_text;
mod parsing;

pub use format_text::{format_text};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm_plugin;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use wasm_plugin::*;