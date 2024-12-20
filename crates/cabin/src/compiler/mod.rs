use std::{env, path::PathBuf};

pub fn compile(c_code: &str) -> anyhow::Result<PathBuf> {
	let c_path = env::temp_dir().join("cabin_transpiled.c");
	std::fs::write(c_path, c_code)?;
	let _ = std::process::Command::new("clang")
		.arg("-ferror-limit=0")
		.arg("-w")
		.arg("-o")
		.arg("cabin_output")
		.arg("cabin_transpiled.c")
		.current_dir(env::temp_dir())
		.spawn()?;
	Ok(env::temp_dir().join("cabin_output"))
}

pub fn run_native_executable(path: PathBuf) -> anyhow::Result<()> {
	let _ = std::process::Command::new(path).spawn()?;
	Ok(())
}
