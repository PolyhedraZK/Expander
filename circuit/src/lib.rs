#![no_std]

mod ecc_circuit;
pub use ecc_circuit::*;

mod layered;
pub use layered::*;

mod witness;
pub use witness::*;

mod serde;
pub use serde::*;

use wasm_bindgen::prelude::wasm_bindgen;use ark_std::string::ToString;
// Import the `console.log` function from the `console` object
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
// Define a macro to make console.log easier to use
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
