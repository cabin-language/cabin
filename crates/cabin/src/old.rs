use clap::Parser as _;

use crate::cli::commands::{CabinCommand as _, SubCommand};

/// The `api` module. This module holds various utilities and abstractions within the Cabin
/// compiler.
pub mod api;

/// The `cli` module. This module handles various Cabin CLI jobs such as the various commands
/// available from the command line.
pub mod cli;

/// The `compiler` module. This module compiling transpiled C code into a native binary executable.
pub mod compiler;

/// The `comptime` module. This module handles the core API for compile-time evaluation.
pub mod comptime;

/// The lexer module. This module handles tokenizing source code strings into token streams before
/// they're passed off to the parser for parsing.
pub mod lexer;

/// The parser module. This module handles parsing token streams into abstract syntax trees.
pub mod parser;

/// The transpiler module. This module handles transpiling abstract syntax trees into C code.
pub mod transpiler;

/// The Cabin standard library. This is a Cabin file that's automatically imported into every Cabin
/// project or file. It contains definitions for all of the built-in types and objects, such as
/// `Text`, `Number`, `terminal`, etc. See `/std/stdlib.cabin` for its contents.
pub const STDLIB: &str = include_str!("../std/stdlib.cabin");

/// The Cabin prelude. This is a Cabin file that's automatically prepended to all Cabin files
/// written by the user. It just brings some useful items into scope from the standard library.
/// See `/std/prelude.cabin` for its contents.
pub const PRELUDE: &str = include_str!("../std/prelude.cabin");

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
