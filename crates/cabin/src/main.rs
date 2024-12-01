use clap::Parser as _;

use crate::cli::commands::{CabinCommand as _, SubCommand};

pub mod api;
pub mod cli;
pub mod compiler;
pub mod comptime;
pub mod lexer;
pub mod parser;
pub mod transpiler;

pub const STDLIB: &str = include_str!("../std/stdlib.cabin");
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");

/// A `clap::Parser` for the arguments passed at the command line. This is called from the main entry point, and
/// delegates work to whatever subcommand was used.
#[derive(clap::Parser)]
pub struct CabinCompilerArguments {
	/// The subcommand to run.
	#[command(subcommand)]
	pub command: SubCommand,
}

/// The main entry point for the Cabin executable. All this does is delegate the work to one of the various
/// subcommands.
fn main() {
	CabinCompilerArguments::parse().command.execute();
}
