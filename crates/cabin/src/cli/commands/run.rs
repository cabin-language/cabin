use std::{collections::HashMap, path::PathBuf};

use colored::Colorize;

use crate::{
	api::{
		context::{context, Phase},
		scope::ScopeType,
	},
	cli::{
		commands::{start, step, CabinCommand},
		RunningContext,
	},
	comptime::CompileTime as _,
	debug_start,
	lexer::{tokenize, tokenize_main, tokenize_without_prelude, Span},
	parser::{
		expressions::{
			field_access::FieldAccessType,
			function_call::FunctionCall,
			name::Name,
			object::{Field, ObjectConstructor},
			Expression,
		},
		parse,
		statements::tag::TagList,
		Module,
		TokenQueue,
	},
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
