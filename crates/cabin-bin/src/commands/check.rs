use colored::Colorize as _;

use crate::commands::{check_errors, CabinCommand};

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct CheckCommand {}

impl CabinCommand for CheckCommand {
	fn execute(self) {
		let mut project = match cabin::Project::from_child(std::env::current_dir().unwrap()) {
			Ok(project) => project,
			Err(error) => {
				eprintln!("\n{} {error}\n", "Error:".bold().red());
				return;
			},
		};

		println!("{} {}...", "\nChecking".bold().green(), project.config().information.name.bold());

		if !check_errors(project.check().to_owned(), &mut project, true, false) {
			return;
		}
	}
}
