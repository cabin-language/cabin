use cabin::diagnostics::DiagnosticInfo;
use colored::Colorize as _;

use super::CabinCommand;
use crate::{snippet::show_snippet, theme::CatppuccinMocha};

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
		let program = std::fs::read_to_string(project.root_directory().join("src").join("main.cabin")).unwrap();

		// Compile-time evaluation
		println!("{} {}...", "\nRunning".bold().green(), project.config().information().name().bold());
		println!("    {} compile-time code...", "Running".bold().green());
		let diagnostics = project.run_compile_time_code().to_owned();
		if !diagnostics.errors().is_empty() {
			eprintln!("\n{}\n", "-".repeat(80));
			for diagnostic in diagnostics.into_iter() {
				if let DiagnosticInfo::Error(error) = &diagnostic.info {
					eprintln!("{} {error}\n", "Error:".bold().red());
					show_snippet::<CatppuccinMocha>(&program, &diagnostic);
					eprintln!("{}\n", "-".repeat(80));
				}
			}

			eprintln!("{} due to the errors above.\n", "Cancelling".bold().red());
			return;
		}

		// Compilation
		println!("    {} compile-time evaluated code...", "Compiling".bold().green());
		let c = project.transpile().unwrap();
		std::fs::write("./output.c", c).unwrap();

		// Running
		println!("    {} runtime code...", "Running".bold().green());
	}
}
