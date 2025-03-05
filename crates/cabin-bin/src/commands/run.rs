use std::path::PathBuf;

use cabin::diagnostics::{DiagnosticInfo, Diagnostics};
use colored::Colorize as _;

use super::CabinCommand;
use crate::{snippet::show_snippet, theme::CatppuccinMocha, wrap};

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
		if !check_errors(project.check().to_owned(), &mut project, true) {
			return;
		}

		// Compile-time evaluation
		println!("    {} compile-time code...", "Running".bold().green());
		if !check_errors(project.run_compile_time_code().to_owned(), &mut project, false) {
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

fn check_errors(diagnostics: Diagnostics, project: &mut cabin::Project, show_warnings: bool) -> bool {
	let has_errors = !diagnostics.errors().is_empty();
	let one_error = diagnostics.errors().len() == 1;
	let max_columns = 100;

	if has_errors || show_warnings {
		eprintln!("\n{}\n", "-".repeat(max_columns).bold());
	}

	for diagnostic in &diagnostics {
		if let DiagnosticInfo::Error(error) = &diagnostic.info {
			eprintln!(
				"{} {}\n",
				"Error:".bold().red(),
				wrap(&format!("Error: {error}"), max_columns).trim_start_matches("Error: ")
			);
			show_snippet::<CatppuccinMocha>(&diagnostic, max_columns);
			let (line, _) = diagnostic.start_line_column();
			let path = if &diagnostic.file == &PathBuf::from("stdlib") {
				"stdlib".to_owned()
			} else {
				format!("{}", pathdiff::diff_paths(&diagnostic.file, project.root_directory()).unwrap().display())
			};
			eprintln!("In {} on line {}\n", path.bold().cyan(), (line + 1).to_string().bold().cyan());
			eprintln!("{}\n", "-".repeat(max_columns).bold());
		}
	}

	if has_errors {
		eprintln!("{} due to the {} above.\n", "Cancelling".bold().red(), if one_error { "error" } else { "errors" });
		return false;
	}
	// Warnings
	else if show_warnings {
		for diagnostic in &diagnostics {
			if let DiagnosticInfo::Warning(warning) = &diagnostic.info {
				eprintln!(
					"{} {}\n",
					"Warning:".bold().yellow(),
					wrap(&format!("Warning: {warning}"), max_columns).trim_start_matches("Warning: ")
				);
				show_snippet::<CatppuccinMocha>(&diagnostic, max_columns);
				let (line, _) = diagnostic.start_line_column();
				let path = if &diagnostic.file == &PathBuf::from("stdlib") {
					"stdlib".to_owned()
				} else {
					format!("{}", pathdiff::diff_paths(&diagnostic.file, project.root_directory()).unwrap().display())
				};
				eprintln!("In {} on line {}\n", path.bold().cyan(), (line + 1).to_string().bold().cyan());
				eprintln!("{}\n", "-".repeat(max_columns).bold());
			}
		}
	}

	true
}
