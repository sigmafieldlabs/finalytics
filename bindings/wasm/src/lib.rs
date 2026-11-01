//! The WebAssembly module behind the `finalytics` npm package.
//!
//! This layer only converts data between JavaScript and Rust; the analytics live in the
//! `finalytics` crate.

use wasm_bindgen::prelude::*;

/// Returns the version of this release of Finalytics.
#[wasm_bindgen]
pub fn version() -> String {
    finalytics::version().to_owned()
}
