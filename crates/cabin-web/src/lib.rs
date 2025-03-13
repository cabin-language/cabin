use cabin::{comptime::CompileTime, interpreter::Runtime};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "runCode")]
pub fn run_code(code: &str, input: js_sys::Function, output: js_sys::Function, error: js_sys::Function) {
	let mut context = cabin::Context::with_io(
		JsRunOptions {
			io: JsIo {
				input: JsReader { read: input },
				output: JsWriter { write: output },
				error: JsWriter { write: error },
			},
		}
		.io
		.into(),
	);

	context.print("Running compile-time code...\n");

	let program = cabin::parse_program(code, &mut context);
	let evaluated_program = program.evaluate_at_compile_time(&mut context);

	context.print("Running runtime code...\n");

	let _ = evaluated_program.evaluate_at_runtime(&mut context);
}

#[wasm_bindgen(js_name = "Io")]
#[derive(Clone)]
pub struct JsIo {
	input: JsReader,
	output: JsWriter,
	error: JsWriter,
}

#[wasm_bindgen()]
impl JsIo {
	#[wasm_bindgen(constructor)]
	pub fn new(input: JsReader, output: JsWriter, error: JsWriter) -> Self {
		Self { input, output, error }
	}
}

impl From<JsIo> for cabin::io::Io<JsReader, JsWriter, JsWriter> {
	fn from(io: JsIo) -> Self {
		cabin::io::Io {
			input: io.input,
			output: io.output,
			error: io.error,
		}
	}
}

#[wasm_bindgen(js_name = "RunOptions")]
pub struct JsRunOptions {
	io: JsIo,
}

#[wasm_bindgen]
impl JsRunOptions {
	#[wasm_bindgen(constructor)]
	pub fn new(io: JsIo) -> Self {
		Self { io }
	}
}

#[wasm_bindgen(js_name = "IoReader")]
#[derive(Clone)]
pub struct JsReader {
	read: js_sys::Function,
}

#[wasm_bindgen]
impl JsReader {
	#[wasm_bindgen(constructor)]
	pub fn new(read: js_sys::Function) -> Self {
		Self { read }
	}
}

impl cabin::io::IoReader for JsReader {
	fn read(&mut self) -> String {
		let output = self.read.call0(&wasm_bindgen::JsValue::NULL).unwrap();
		let Some(string) = output.as_string() else {
			return String::new();
		};
		string
	}
}

#[wasm_bindgen(js_name = "IoWriter")]
#[derive(Clone)]
pub struct JsWriter {
	write: js_sys::Function,
}

#[wasm_bindgen]
impl JsWriter {
	#[wasm_bindgen(constructor)]
	pub fn new(write: js_sys::Function) -> Self {
		Self { write }
	}
}

impl cabin::io::IoWriter for JsWriter {
	fn write(&mut self, value: &cabin::io::StyledString) {
		self.write.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(&value.value)).unwrap();
	}
}
