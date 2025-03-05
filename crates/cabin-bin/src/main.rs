use clap::Parser as _;
use commands::{CabinCommand as _, SubCommand};

mod commands;
mod snippet;

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

pub fn wrap(mut text: &str, max_line_length: usize) -> String {
	let mut result = Vec::new();

	while !text.is_empty() {
		let splindex = if text.len() <= max_line_length {
			text.len()
		} else {
			max_line_length - text[..max_line_length].chars().rev().position(|c| c == ' ').unwrap()
		};
		result.push(text.get(0..splindex).unwrap());
		text = text.get(splindex..).unwrap();
	}

	result.join("\n")
}
