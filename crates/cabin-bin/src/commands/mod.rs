use run::RunCommand;

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
}
