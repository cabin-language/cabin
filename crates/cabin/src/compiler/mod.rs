use std::{path::Path, process::Command};

pub(crate) fn compile_c_to_native_binary<Input: AsRef<Path>, Output: AsRef<Path>>(c_file: Input, output: Output) -> Result<(), std::io::Error> {
	let _ = Command::new("clang").arg("-o").arg(output.as_ref().as_os_str()).arg(c_file.as_ref().as_os_str()).spawn()?;

	Ok(())
}
