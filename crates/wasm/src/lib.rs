use wasm_bindgen::prelude::*;

mod api;
mod error;
mod model;

#[wasm_bindgen(start)]
pub fn main_js() {
    console_error_panic_hook::set_once();
}
