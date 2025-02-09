use std::path::PathBuf;

use colored::Colorize;

use crate::{
	api::context::context,
	cli::{commands::CabinCommand, RunningContext},
	STDLIB,
};

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct RunCommand {
	path: Option<String>,
}

impl CabinCommand for RunCommand {
	fn execute(self) {
		let path = self.path.map_or_else(|| std::env::current_dir().unwrap(), PathBuf::from);
		context().running_context = RunningContext::try_from(&path).unwrap_or_else(|error| {
			eprintln!("{} Error running file: {error}", "Error:".bold().red());
			std::process::exit(1);
		});

		let errors = crate::check(STDLIB);
		for error in errors {
			println!("{error}");
		}
	}
}
