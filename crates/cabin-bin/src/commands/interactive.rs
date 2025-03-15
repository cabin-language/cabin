use std::io::Write;

use colored::Colorize as _;

use crate::commands::CabinCommand;

/// Run a cabin file or project.
#[derive(clap::Parser)]
pub struct InteractiveCommand {}

impl CabinCommand for InteractiveCommand {
	fn execute(self) {
		println!("Cabin interpreter {}\n", "(Press Ctrl+C to exit)".dimmed());
		let mut context = cabin::context::StandardContext::interactive();
		loop {
			print!("> ");
			std::io::stdout().flush().unwrap();

			let mut line = String::new();
			std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();

			cabin::interpret(&line, &mut context);

			for diagnostic in context.diagnostics() {
				match &diagnostic.info {
					cabin::diagnostics::DiagnosticInfo::Error(error) => eprintln!("{} {error}", "Error:".bold().red()),
					cabin::diagnostics::DiagnosticInfo::Warning(warning) => eprintln!("{} {warning}", "Warning:".bold().yellow()),
					cabin::diagnostics::DiagnosticInfo::Info(info) => eprintln!("{} {info}", "Info:".bold().cyan()),
				}
			}

			context.clear_diagnostics();
		}
	}
}
