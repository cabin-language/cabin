use super::CabinCommand;

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct RunCommand {}

impl CabinCommand for RunCommand {
	fn execute(self) {
		cabin::check_program(include_str!("../../../cabin/tests/dev/src/main.cabin"));
	}
}
