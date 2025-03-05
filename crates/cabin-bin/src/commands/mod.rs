use new::NewCommand;
use run::RunCommand;

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
}

#[macro_export]
macro_rules! check_errors {
	($diagnostics: expr) => {
		if !$diagnostics.errors().is_empty() {
			eprintln!();

			for (error, _span) in $diagnostics.errors() {
				eprintln!("{} {error}\n", "Error:".bold().red());
			}

			eprintln!("{} due to the errors above.\n", "Cancelling".bold().red());
			return;
		}
	};
}
