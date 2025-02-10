use colored::Colorize as _;

use super::CabinCommand;

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct RunCommand {}

impl CabinCommand for RunCommand {
	fn execute(self) {
		let program = std::fs::read_to_string("./src/main.cabin").unwrap_or_else(|_| {
			println!("{} No main file found.", "Error:".bold().red());
			std::process::exit(1);
		});

		let errors = cabin::check_program(&program);

		for (error, _span) in errors.errors() {
			println!("{} {error}", "Error:".bold().red());
		}
	}
}
