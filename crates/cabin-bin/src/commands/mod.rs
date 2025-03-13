use std::path::PathBuf;

use cabin::{
	diagnostics::{DiagnosticInfo, Diagnostics},
	theme::CatppuccinMocha,
};
use check::CheckCommand;
use colored::Colorize as _;
use interactive::InteractiveCommand;
use new::NewCommand;
use run::RunCommand;

use crate::{snippet::show_snippet, wrap};

mod check;
mod interactive;
mod new;
mod run;

#[enum_dispatch::enum_dispatch]
pub trait CabinCommand {
	/// Executes this subcommand.
	fn execute(self);
}

#[derive(clap::Subcommand)]
#[enum_dispatch::enum_dispatch(CabinCommand)]
pub enum SubCommand {
	Run(RunCommand),
	New(NewCommand),
	Check(CheckCommand),
	Interactive(InteractiveCommand),
}

pub fn check_errors(diagnostics: Diagnostics, project: &mut cabin::Project, show_warnings: bool, cancel_on_errors: bool) -> bool {
	let has_errors = !diagnostics.errors().is_empty();
	let one_error = diagnostics.errors().len() == 1;
	let max_columns = 100;

	if has_errors || (show_warnings && !diagnostics.warnings().is_empty()) {
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

	if has_errors && cancel_on_errors {
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
