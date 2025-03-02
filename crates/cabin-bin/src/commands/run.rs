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

		// Compile-time evaluation
		println!("{} {}...", "\nRunning".bold().green(), project.config().information().name().bold());
		println!("    {} compile-time code...", "Running".bold().green());

		// Check diagnostics
		let diagnostics = project.run_compile_time_code().to_owned();
		let one_error = diagnostics.errors().len() == 1;
		if !diagnostics.errors().is_empty() {
			eprintln!("\n{}\n", "-".repeat(80));
			for diagnostic in diagnostics.into_iter() {
				if let DiagnosticInfo::Error(error) = &diagnostic.info {
					eprintln!("{} {error}\n", "Error:".bold().red());
					show_snippet::<CatppuccinMocha>(&diagnostic);
					let (line, _) = diagnostic.start_line_column().unwrap();
					eprintln!(
						"In {} on line {}\n",
						format!("{}", pathdiff::diff_paths(diagnostic.file, project.root_directory()).unwrap().display())
							.bold()
							.cyan(),
						line.to_string().bold().cyan()
					);
					eprintln!("{}\n", "-".repeat(80));
				}
			}

			eprintln!("{} due to the {} above.\n", "Cancelling".bold().red(), if one_error { "error" } else { "errors" });
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
