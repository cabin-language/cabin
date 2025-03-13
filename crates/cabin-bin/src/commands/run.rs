use colored::Colorize as _;

use crate::commands::{check_errors, CabinCommand};

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct RunCommand {}

impl CabinCommand for RunCommand {
	fn execute(self) {
		let mut project = match cabin::Project::from_child(std::env::current_dir().unwrap()) {
			Ok(project) => project,
			Err(error) => {
				eprintln!("\n{} {error}\n", "Error:".bold().red());
				return;
			},
		};

		println!("{} {}...", "\nRunning".bold().green(), project.config().information.name.bold());

		// Checking
		println!("{} syntax and types...", "\tChecking".bold().green());
		if !check_errors(project.check().to_owned(), &mut project, true, true) {
			return;
		}

		// Compile-time evaluation
		println!("    {} compile-time code...", "Running".bold().green());
		if !check_errors(project.run_compile_time_code().to_owned(), &mut project, false, true) {
			return;
		}
		if project.printed() {
			println!();
		}

		// Compilation
		println!("    {} compile-time evaluated code...", "Compiling".bold().green());
		let c = project.transpile().unwrap();
		std::fs::write("./output.c", c).unwrap();

		// Running
		println!("    {} runtime code...", "Running".bold().green());
	}
}
