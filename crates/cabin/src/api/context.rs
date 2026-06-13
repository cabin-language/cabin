use std::{fmt::Display, path::PathBuf};

use super::io::{Io, StyledString, SystemIo};
use crate::{
	Diagnostics,
	STDLIB,
	api::{diagnostics::Diagnostic, scope::ScopeTree},
	ast::expressions::identifier::Identifier,
	comptime::{
		Evaluate as _,
		memory::{Pointer, VirtualMemory},
	},
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
	pub memory: VirtualMemory,

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
}

impl Default for Context {
	fn default() -> Self {
		Context::with_io(SystemIo)
	}
}

#[derive(Debug, Clone)]
pub struct ContextSnapshot {
	scope: ScopeTree,
	virtual_memory: VirtualMemory,
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
			virtual_memory: self.memory.clone(),
		}
	}

	pub fn roll_back(&mut self, snapshot: ContextSnapshot) {
		self.scope = snapshot.scope;
		self.memory = snapshot.virtual_memory;
	}

	pub fn with_io<System: Io + 'static>(io: System) -> Self {
		let mut context = Context {
			scope: ScopeTree::global(),
			memory: VirtualMemory::empty(),
			diagnostics: Diagnostics::empty(),
			side_effects: true,
			has_printed: false,
			file: "stdlib".into(),
			name_query: None,
			name_query_result: None,
			system_io: Box::new(io),
			interactive: false,

			debug_mode: DebugMode::None,
		};

		// Add stdlib
		let stdlib_ast = crate::parse_library(STDLIB, &mut context);
		let stdlib_ast = stdlib_ast
			.inspect_err(|_| {
				dbg!(context.diagnostics.all().first());
			})
			.unwrap();
		let stdlib = stdlib_ast.evaluate(&mut context);

		// Bring some stdib items into scope
		context
			.scope
			.declare_new_variable(Identifier::create_virtual("Text", &context), stdlib.get_pointer("Text"))
			.unwrap();
		context
			.scope
			.declare_new_variable(Identifier::create_virtual("Number", &context), stdlib.get_pointer("Number"))
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("print", &context),
				stdlib
					.get_object("system", &context)
					.get_field_value("terminal")
					.unwrap()
					.value(&context)
					.as_object()
					.unwrap()
					.get_field_value("print")
					.unwrap(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("log", &context),
				stdlib
					.get_object("system", &context)
					.get_field_value("terminal")
					.unwrap()
					.value(&context)
					.as_object()
					.unwrap()
					.get_field_value("log")
					.unwrap(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("debug", &context),
				stdlib
					.get_object("system", &context)
					.get_field_value("terminal")
					.unwrap()
					.value(&context)
					.as_object()
					.unwrap()
					.get_field_value("debug")
					.unwrap(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("input", &context),
				stdlib
					.get_object("system", &context)
					.get_field_value("terminal")
					.unwrap()
					.value(&context)
					.as_object()
					.unwrap()
					.get_field_value("input")
					.unwrap(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("none", &context),
				*stdlib
					.get_either("Optional", &context)
					.variants()
					.iter()
					.find(|variant| variant.name().source_identifier() == "none")
					.unwrap()
					.object(),
			)
			.unwrap();

		let stdlib_pointer = stdlib.into_object(&context).store(&mut context);
		context.scope.declare_new_variable(Identifier::create_virtual("builtin", &context), stdlib_pointer).unwrap();

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

	pub fn get_false(&self) -> Pointer {
		self.scope.get_variable_from_id("false", ScopeRef::global()).unwrap()
	}

	pub fn get_true(&self) -> Pointer {
		self.scope.get_variable_from_id("true", ScopeRef::global()).unwrap()
	}

	pub fn none(&self) -> Pointer {
		self.scope.get_variable_from_id("none", ScopeRef::global()).unwrap()
	}

	pub fn any(&self) -> Pointer {
		self.scope.get_variable_from_id("Any", ScopeRef::global()).unwrap()
	}

	pub fn ident(&self, name: &'static str) -> Pointer {
		self.scope.get_variable_from_id(name, ScopeRef::stdlib()).expect(&format!("Couldnt find {name}"))
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
