use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_fields::wasm_fields;

#[wasm_bindgen(js_name = "runCode")]
pub fn run_code(code: &str, io: Io) {
	let mut context = cabin::Context::with_io(io);
	cabin::interpret_with_logs(code, &mut context);
}

#[wasm_bindgen]
#[wasm_fields]
pub struct Io {
	write: js_sys::Function,
	read_line: js_sys::Function,
	error_write: js_sys::Function,
}

#[wasm_bindgen(typescript_custom_section)]
const IIO: &'static str = r#"
interface IIo {
	write: (value: string) => void,
	error_write: (value: string) => void,
	read_line: () => string,
}
"#;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(typescript_type = "IJsIo")]
	pub type IIo;
}

#[wasm_bindgen]
impl Io {
	#[wasm_bindgen(constructor)]
	pub fn new(io: IIo) -> Io {
		let value: JsValue = io.into();
		Io {
			write: js_sys::Reflect::get(&value, &"write".into()).unwrap().into(),
			error_write: js_sys::Reflect::get(&value, &"error_write".into()).unwrap().into(),
			read_line: js_sys::Reflect::get(&value, &"read_line".into()).unwrap().into(),
		}
	}
}

impl cabin::io::Io for Io {
	fn read_line(&mut self) -> String {
		self.read_line.call0(&JsValue::NULL).unwrap().as_string().unwrap()
	}

	fn write(&mut self, value: &cabin::io::StyledString) {
		self.write.call1(&JsValue::NULL, &JsValue::from_str(&value.value)).unwrap();
	}

	fn error_write(&mut self, value: &cabin::io::StyledString) {
		todo!()
	}

	fn get_environment_variable(&mut self, name: &str) -> Option<String> {
		todo!()
	}

	fn set_environment_variable(&mut self, name: &str, value: &str) {
		todo!()
	}

	fn read_file(&mut self, path: &str) -> Option<String> {
		todo!()
	}

	fn write_file(&mut self, path: &str, contents: &str) {
		todo!()
	}

	fn delete_file(&mut self, path: &str) {
		todo!()
	}
}
