use std::io::Write as _;

use cabin::diagnostics::Severity;
use colored::Colorize as _;

use crate::commands::CabinCommand;

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct InteractiveCommand;

impl CabinCommand for InteractiveCommand {
	fn execute(self) {
		println!("Cabin interpreter {}\n", "(Press Ctrl+C to exit)".dimmed());
		let mut context = cabin::context::Context::interactive();

		#[allow(clippy::infinite_loop, reason = "its a repl!")]
		loop {
			print!("> ");
			std::io::stdout().flush().unwrap();

			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();

			cabin::interpret(&line, &mut context);

			for diagnostic in context.diagnostics() {
				match &diagnostic.info.severity() {
					Severity::AlwaysError | Severity::ProdError => eprintln!("{} {}", "Error:".bold().red(), diagnostic.info),
					Severity::AlwaysWarn | Severity::ProdWarning => eprintln!("{} {}", "Warning:".bold().yellow(), diagnostic.info),
					Severity::AlwaysInfo | Severity::ProdInfo => eprintln!("{} {}", "Info:".bold().cyan(), diagnostic.info),
					Severity::AlwaysHint | Severity::ProdHint => eprintln!("{} {}", "Hint:".bold().cyan(), diagnostic.info),
				}
			}

			context.clear_diagnostics();
		}
	}
}
