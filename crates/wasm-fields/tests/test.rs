use wasm_bindgen::prelude::wasm_bindgen;
use wasm_fields::wasm_fields;

#[test]
pub fn test() {
	#[wasm_bindgen]
	#[wasm_fields]
	pub struct Example {
		text: String,
	}
}
