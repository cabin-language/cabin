use std::{collections::HashMap, path::Path};

use crate::{
	api::{
		diagnostics::{Diagnostic, DiagnosticInfo},
		scope::ScopeData,
	},
	ast::expressions::{name::Name, Expression},
	comptime::memory::{VirtualMemory, VirtualPointer},
	Diagnostics,
	Span,
	STDLIB,
};

pub struct Context {
	// Publicly mutable
	pub(crate) scope_tree: ScopeData,
	pub(crate) virtual_memory: VirtualMemory,
	pub(crate) libraries: HashMap<Name, VirtualPointer>,

	// Privately mutable
	side_effects_stack: Vec<bool>,
	diagnostics: Diagnostics,
}

impl Default for Context {
	fn default() -> Self {
		let mut context = Context {
			scope_tree: ScopeData::global(),
			virtual_memory: VirtualMemory::empty(),
			side_effects_stack: Vec::new(),
			diagnostics: Diagnostics::empty(),
			libraries: HashMap::new(),
		};

		// Add stdlib
		let library = crate::parse_library(STDLIB, &mut context).to_pointer(&mut context);
		let _ = context.libraries.insert("builtin".into(), library);
		if let Err(error) = context.scope_tree.declare_new_variable("builtin", Expression::Pointer(library)) {
			context.add_diagnostic(Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(error),
			});
		}

		context
	}
}

impl Context {
	pub fn add_library<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), std::io::Error> {
		let library = crate::parse_library_file(path, self)?.to_pointer(self);
		let _ = self.libraries.insert(name.into(), library);
		if let Err(error) = self.scope_tree.declare_new_variable(name, Expression::Pointer(library)) {
			self.add_diagnostic(Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(error),
			});
		}
		Ok(())
	}

	pub(crate) fn toggle_side_effects(&mut self, side_effects: bool) {
		self.side_effects_stack.push(side_effects);
	}

	pub(crate) fn untoggle_side_effects(&mut self) {
		let _ = self.side_effects_stack.pop();
	}

	pub fn diagnostics(&self) -> &Diagnostics {
		&self.diagnostics
	}

	pub fn has_side_effects(&self) -> bool {
		self.side_effects_stack.last().copied().unwrap_or(true)
	}

	pub fn add_diagnostic(&mut self, error: Diagnostic) {
		self.diagnostics.push(error);
	}
}
