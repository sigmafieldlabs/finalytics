//! Runs in a WebAssembly runtime with `wasm-bindgen-test`.

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn version_is_exposed() {
    assert_eq!(finalytics_wasm::version(), finalytics::VERSION);
}
