use colored::Colorize as _;

use super::CabinCommand;
use crate::check_errors;

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct RunCommand {}

impl CabinCommand for RunCommand {
	fn execute(self) {
		let mut project = match cabin::Project::new(std::env::current_dir().unwrap()) {
			Ok(project) => project,
			Err(error) => {
				eprintln!("\n{} {error}\n", "Error:".bold().red());
				return;
			},
		};

		// Compile-time evaluation
		println!("{} {}...", "\nRunning".bold().green(), project.config().information().name().bold());
		println!("    {} compile-time code...", "Running".bold().green());
		let diagnostics = project.run_compile_time_code();
		check_errors!(diagnostics);

		// Compilation
		println!("    {} compile-time evaluated code...", "Compiling".bold().green());
		let c = project.transpile().unwrap();
		std::fs::write("./output.c", c).unwrap();

		// Running
		println!("    {} runtime code...", "Running".bold().green());
	}
}
