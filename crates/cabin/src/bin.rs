use cabin::cli::commands::{CabinCommand as _, SubCommand};
use clap::Parser as _;

/// The Cabin compiler.
#[derive(clap::Parser)]
pub struct CabinCompilerArguments {
	/// The command to run.
	#[command(subcommand)]
	pub command: SubCommand,
}

/// The main entry point for the Cabin executable. All this does is delegate the work to one of the various
/// subcommands.
fn main() {
	CabinCompilerArguments::parse().command.execute();
}
