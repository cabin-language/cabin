use std::{fmt::Display, path::PathBuf};

use cabin_macros::import;

use super::io::{Io, StyledString, SystemIo};
use crate::{
	Diagnostics,
	STDLIB,
	Spanned,
	api::{diagnostics::Diagnostic, scope::ScopeTree},
	ast::expressions::{Expression, extend::Extend, function_call::FunctionCall, identifier::Identifier},
	comptime::{
		Dot as _,
		Evaluate as _,
		storage::{Pointer, Storable, Storage},
	},
	definition::VariableDeclaration,
	diagnostics::DiagnosticInfo,
	log,
	scope::ScopeRef,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DebugMode {
	Full,
	None,
}

/// Global(ish) data about the state of the compiler. The context holds the program's scope data,
/// as well as memory where expressions are stored, among some other metadata like the file path
/// being evaluated.
///
/// # Type Parameters
///
/// The `Context` takes three type parameters: `Input: Read`, `Output: Write`, and `Error: Write`.
/// These type parameters are passed up to its internal `io: Io` field, and they dictate how to
/// read and write data to and from the system. For regular programs, these are simply set to
/// `Stdin`, `Stdout`, and `Stderr` from Rust's standard library. When compiling to WebAssembly,
/// for example, input and output are handled differently.
///
/// - `Input` - The input stream to read from when calling `input()` in Cabin
/// - `Output` - The output stream to read from when calling `print()` in Cabin
/// - `Error` - The error stream to read from when printing an error in Cabin
pub struct Context {
	/// Scope information about the program. Scopes are stored in this tree, with each scope
	/// containing a map between variable names and their values (as `ExpressionPointers`)
	///
	/// This should never be reassigned, only mutated with the methods on it for declaring and
	/// reassigning variables.
	pub scope: ScopeTree,

	/// Storage for expressions. All expressions are stored in `VirtualMemory`, and can be accessed
	/// with an `ExpressionPointer`, allowing one expression to be reused and mutated globally from
	/// different places in the user's code.
	pub storage: Storage,

	pub system_io: Box<dyn Io>,

	/// Whether Cabin is currently being run as an interactive REPL.
	pub interactive: bool,

	/// Whether the AST is currently being evaluated "with side effects".
	///
	/// For example, when checking for type errors, we dont want to actually run any code that
	/// the user can see the effects of, so this is `false`.
	///
	/// Another example is branch constructs, i.e., if the condition in an `if` expression is
	/// `true`, a corresponding `otherwise` block will still be evaluated, just without side
	/// effects. This allows checking parts of code for validity without running it.
	///
	/// Certain builtin functions, such as `print`, will simply not run (or have their behavior
	/// affected) when this is `false`.
	pub side_effects: bool,

	/// The path to the file currently being acted upon (tokenized/parsed/evaluated/transpiled etc.)
	pub file: PathBuf,

	/// Whether the user has printed to stdout or stderr at compile-time. This is stored because
	/// when the first line is printed (and the first line only), an additional empty line should
	/// be printed before it. Additionally, if any lines are printed, an additional newline is
	/// printed after compile-time evaluation.
	pub has_printed: bool,

	/// Diagnostic information about the user's code, such as warnings, errors, hints, etc.
	diagnostics: Diagnostics,

	pub name_query_result: Option<Identifier>,
	pub name_query: Option<usize>,
	pub debug_mode: DebugMode,
	pub call_stack: Vec<FunctionCall>,

	pub path: Vec<String>,
}

impl Default for Context {
	fn default() -> Self {
		Context::with_io(SystemIo)
	}
}

#[derive(Debug, Clone)]
pub struct ContextSnapshot {
	scope: ScopeTree,
	virtual_memory: Storage,
}

impl Context {
	pub fn interactive() -> Context {
		Context {
			interactive: true,
			..Default::default()
		}
	}

	/// Creates a snapshot of the scope and memory of the current execution. The context
	/// can later be rolled back to this snapshot with `context.roll_back()`.
	///
	/// This allows running code such as action bodies to check for compile-time errors
	/// without performing undesired mutations by saving a snapshot, running the mutations,
	/// and then rolling back to the snapshot.
	pub fn snapshot(&self) -> ContextSnapshot {
		ContextSnapshot {
			scope: self.scope.clone(),
			virtual_memory: self.storage.clone(),
		}
	}

	pub fn roll_back(&mut self, snapshot: ContextSnapshot) {
		self.scope = snapshot.scope;
		self.storage = snapshot.virtual_memory;
	}

	pub fn with_io<System: Io + 'static>(io: System) -> Self {
		let mut context = Context {
			scope: ScopeTree::global(),
			storage: Storage::empty(),
			path: Vec::new(),
			diagnostics: Diagnostics::empty(),
			side_effects: true,
			has_printed: false,
			file: "stdlib".into(),
			name_query: None,
			name_query_result: None,
			system_io: Box::new(io),
			interactive: false,
			call_stack: Vec::new(),
			debug_mode: DebugMode::None,
		};

		// parse & evaluate standard library
		log!(&context, "Parsing standard library");
		let stdlib_ast = crate::parse_library(STDLIB, &mut context);
		let stdlib_ast = stdlib_ast
			.inspect_err(|_| {
				dbg!(context.diagnostics.all().first());
			})
			.unwrap();
		log!(&context, "Evaluating standard library");
		let std = stdlib_ast.evaluate(&mut context);

		log!(&context, "Generating prelude");

		// Bring some stdib items into scope
		import!(context, std::Text);
		import!(context, std::Number);
		import!(context, std::List);
		import!(context, std::Any);

		import!(context, std::system::terminal::print);
		import!(context, std::system::terminal::log);
		import!(context, std::system::terminal::debug);
		import!(context, std::system::terminal::input);
		import!(context, std::system::terminal);

		// import!(context, std::Optional::none);
		// import!(context, std::Optional::some);

		// convert stdlib module into an object bring it into global scope under the name `builtin`
		let stdlib_pointer = std.into_object().store(&mut context);
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("builtin".to_owned(), &context),
				VariableDeclaration::with_value(Identifier::create_virtual("builtin".to_owned(), &context), stdlib_pointer),
			)
			.unwrap();

		// return context
		context
	}

	/// Returns the diagnostics found in the user's code. Note that this only returns diagnostics
	/// that are already stored; It doesn't perform a new scan for diagnostics. Usually diagnostics
	/// will be fetched after performing an evaluation step on a `Project`.
	///
	/// # Returns
	///
	/// The diagnostics in the user's code
	pub const fn diagnostics(&self) -> &Diagnostics {
		&self.diagnostics
	}

	pub fn add_to_path<S: AsRef<str>>(&mut self, path: S) {
		self.path.push(path.as_ref().to_owned());
	}

	pub fn pop_from_path(&mut self) {
		let _ = self.path.pop();
	}

	pub fn get_path(&self) -> Option<String> {
		if self.path.is_empty() { None } else { Some(self.path.join("::")) }
	}

	pub const fn diagnostics_mut(&mut self) -> &mut Diagnostics {
		&mut self.diagnostics
	}

	pub fn clear_diagnostics(&mut self) {
		self.diagnostics.clear();
	}

	/// Adds a new diagnostic to the context. Diagnostics are retrievable via
	/// `context.diagnostics()`.
	///
	/// # Parameter
	///
	/// - `diagnostic` - The diagnostic to add
	pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
		self.diagnostics.push(diagnostic);
	}

	pub fn add_diagnostic_at(&mut self, location: &impl Spanned, info: DiagnosticInfo) {
		self.diagnostics.push(Diagnostic::at(location, info));
	}

	pub const fn scope_tree(&self) -> &ScopeTree {
		&self.scope
	}

	pub fn print<S: Display>(&mut self, text: S) {
		self.system_io.write(&StyledString::plain(format!("{text}")));
	}

	pub fn println<S: Display>(&mut self, text: S) {
		self.system_io.writeln(&StyledString::plain(format!("{text}")));
	}

	pub fn eprintln<S: Display>(&mut self, text: S) {
		self.system_io.error_writeln(&StyledString::plain(format!("{text}")));
	}

	pub fn input(&mut self) -> String {
		self.system_io.read_line()
	}

	pub fn get_false(&mut self) -> Pointer {
		self.get_builtin("false").unwrap()
	}

	pub fn get_true(&mut self) -> Pointer {
		self.get_builtin("true").unwrap()
	}

	pub fn none(&mut self) -> Pointer {
		self.get_builtin("none").unwrap()
	}

	pub fn none_expr(&self) -> Expression {
		Expression::Identifier(Identifier::create_virtual("none".to_owned(), self))
	}

	pub fn any(&mut self) -> Pointer {
		self.get_builtin("Any").unwrap()
	}

	pub(crate) fn builtin(&self, name: &'static str) -> Pointer {
		#[allow(clippy::panic, reason = "dev error")]
		self.get_builtin(name).unwrap_or_else(|| {
			dbg!(&self.diagnostics);
			dbg!(&self.scope);
			panic!("Could not find {name}");
		})
	}

	pub fn extensions(&self) -> Vec<&Extend> {
		self.scope
			.available_variables_with_underscores()
			.into_iter()
			.filter_map(|value| match value.value().value(self) {
				Storable::Expression(Expression::Extend(extend)) => Some(extend),
				_ => None,
			})
			.collect()
	}

	pub fn extensions_for(&self, group: Pointer) -> Vec<&Pointer> {
		todo!()
	}

	pub fn reassign_variable(&mut self, name: &Identifier, value: Pointer) {
		let Some(declaration) = self.get_variable_declaration_mut(name) else {
			self.add_diagnostic_at(name, DiagnosticInfo::UnknownVariable(name.source_text().to_owned()));
			return;
		};

		declaration.value = value;
	}

	pub fn get_variable_declaration_mut(&mut self, name: &Identifier) -> Option<&mut VariableDeclaration> {
		let mut current_ref = Some(name.scope());

		while let Some(current) = current_ref {
			let variable = self.scope.get_scope_mut(current).variables.remove(name);

			if let Some(declaration) = variable {
				let evaluated = declaration.evaluate(self);
				let _ = self.scope.get_scope_mut(current).variables.insert(name.to_owned(), evaluated);
				return Some(self.scope.get_scope_mut(current).variables.get_mut(name).unwrap());
			}

			current_ref = self.scope.get_scope(current).parent.map(ScopeRef);
		}

		None
	}

	pub fn get_variable_declaration(&mut self, name: &Identifier) -> Option<&VariableDeclaration> {
		let mut current_ref = Some(name.scope());

		while let Some(current) = current_ref {
			let variable = self.scope.get_scope_mut(current).variables.get(name);

			if let Some(declaration) = variable {
				let evaluated = declaration.clone().evaluate(self);
				let _ = self.scope.get_scope_mut(current).variables.insert(name.to_owned(), evaluated);
				return Some(self.scope.get_scope_mut(current).variables.get_mut(name).unwrap());
			}

			current_ref = self.scope.get_scope(current).parent.map(ScopeRef);
		}

		None
	}

	pub fn get_unevaluated_variable_declaration(&self, name: &Identifier) -> Option<&VariableDeclaration> {
		let mut current_ref = Some(name.scope());

		while let Some(current) = current_ref {
			if let Some(variable) = self.scope.get_scope(current).variables.get(name) {
				return Some(variable);
			}

			current_ref = self.scope.get_scope(current).parent.map(ScopeRef);
		}

		None
	}

	pub fn get_variable(&mut self, name: &Identifier) -> Option<Pointer> {
		self.get_variable_declaration(name).map(|declaration| declaration.value().to_owned())
	}

	pub fn get_unevaluated_variable(&self, name: &Identifier) -> Option<Pointer> {
		self.get_unevaluated_variable_declaration(name).map(|declaration| declaration.value().to_owned())
	}

	pub fn variable_exists(&self, name: &Identifier) -> bool {
		let mut current_ref = Some(name.scope());

		while let Some(current) = current_ref {
			let declaration = self.scope.get_scope(current).variables.get(name);

			if declaration.is_some() {
				return true;
			}

			current_ref = self.scope.get_scope(current).parent.map(ScopeRef);
		}

		false
	}

	pub fn declare_new_variable(&mut self, declaration: VariableDeclaration) {
		if self.variable_exists(declaration.name()) {
			self.add_diagnostic_at(declaration.name(), DiagnosticInfo::DuplicateVariableDeclaration {
				name: declaration.name().source_text().to_owned(),
			});
		}

		let scope = self.scope.get_scope_mut(declaration.name().scope());

		if declaration.name().source_text() == "_" {
			scope.underscores.push(declaration);
			return;
		}

		let _ = scope.variables.insert(declaration.name().to_owned(), declaration);
	}

	pub fn get_builtin(&self, name: &str) -> Option<Pointer> {
		self.get_unevaluated_variable(&Identifier::create_virtual_in_scope(name.to_owned(), ScopeRef::stdlib()))
	}
}

#[macro_export]
macro_rules! log {
	($context: expr, $($tokens: tt)*) => {
		if $context.debug_mode == $crate::api::context::DebugMode::Full {
			eprintln!($($tokens)*);
		}
	};
}
