use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

use crate::{
	api::{
		diagnostics::{Diagnostic, DiagnosticInfo},
		scope::ScopeTree,
	},
	ast::expressions::{name::Name, new_literal::Literal, Expression},
	comptime::memory::{ExpressionPointer, VirtualMemory},
	Diagnostics,
	Span,
	STDLIB,
};

pub struct Context {
	// Publicly mutable
	pub(crate) scope_tree: ScopeTree,
	pub(crate) virtual_memory: VirtualMemory,
	pub(crate) libraries: HashMap<Name, ExpressionPointer>,
	pub(crate) side_effects: bool,
	pub(crate) file: PathBuf,

	// Privately mutable
	diagnostics: Diagnostics,
}

impl Default for Context {
	fn default() -> Self {
		let mut context = Context {
			scope_tree: ScopeTree::global(),
			virtual_memory: VirtualMemory::empty(),
			diagnostics: Diagnostics::empty(),
			libraries: HashMap::new(),
			side_effects: true,
			file: "builtin.cabin".into(),
		};

		// Add stdlib
		let library = Expression::Literal(Literal::Object(crate::parse_library(STDLIB, &mut context).into_object(&mut context))).store_in_memory(&mut context);
		let _ = context.libraries.insert("builtin".into(), library);
		context.scope_tree.declare_new_variable("builtin", library).unwrap();

		context
	}
}

impl Context {
	pub fn diagnostics(&self) -> &Diagnostics {
		&self.diagnostics
	}

	pub fn add_diagnostic(&mut self, error: Diagnostic) {
		self.diagnostics.push(error);
	}
}
