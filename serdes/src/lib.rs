#![no_std]

pub mod error;
pub mod macros;
pub mod serdes;

pub use error::{SerdeError, SerdeResult};
pub use serdes::ExpSerde;
pub use serdes_derive::ExpSerde;

use ark_std::string::ToString;
use wasm_bindgen::prelude::*;
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
