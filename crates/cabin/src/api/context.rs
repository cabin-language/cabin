use std::{fmt::Display, path::PathBuf};

use super::{
	io::{Io, StyledString, SystemIo},
	traits::TryAs as _,
};
use crate::{
	Diagnostics,
	STDLIB,
	api::{diagnostics::Diagnostic, scope::ScopeTree},
	ast::expressions::{
		Expression,
		identifier::Identifier,
		literal::{EvaluatedLiteral, Object},
	},
	comptime::memory::{ExpressionPointer, VirtualMemory},
	scope::ScopeId,
};

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
	pub virtual_memory: VirtualMemory,

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
}

impl Default for Context {
	fn default() -> Self {
		Context::with_io(SystemIo)
	}
}

impl Context {
	pub fn interactive() -> Context {
		Context {
			interactive: true,
			..Default::default()
		}
	}
}

impl Context {
	pub fn with_io<System: Io + 'static>(io: System) -> Self {
		let mut context = Context {
			scope: ScopeTree::global(),
			virtual_memory: VirtualMemory::empty(),
			diagnostics: Diagnostics::empty(),
			side_effects: true,
			has_printed: false,
			file: "stdlib".into(),
			name_query: None,
			name_query_result: None,
			system_io: Box::new(io),
			interactive: false,
		};

		// Add stdlib
		let stdlib_ast = crate::parse_library(STDLIB, &mut context);
		let stdlib_pointer = Expression::EvaluatedLiteral(EvaluatedLiteral::Object(stdlib_ast.into_object(&mut context))).store_in_memory(&mut context);
		context.scope.declare_new_variable(Identifier::create_virtual("builtin", &context), stdlib_pointer).unwrap();
		let Expression::EvaluatedLiteral(EvaluatedLiteral::Object(stdlib)) = stdlib_pointer.expression(&context).to_owned() else {
			unreachable!()
		};

		// Bring some stdib items into scope
		context
			.scope
			.declare_new_variable(Identifier::create_virtual("Text", &context), stdlib.get_field("Text").unwrap().into())
			.unwrap();
		context
			.scope
			.declare_new_variable(Identifier::create_virtual("Number", &context), stdlib.get_field("Number").unwrap().into())
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("print", &context),
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("print")
					.unwrap()
					.into(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("debug", &context),
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("debug")
					.unwrap()
					.into(),
			)
			.unwrap();
		context
			.scope
			.declare_new_variable(
				Identifier::create_virtual("input", &context),
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.as_evaluated()
					.unwrap()
					.try_as::<Object>()
					.unwrap()
					.get_field("input")
					.unwrap()
					.into(),
			)
			.unwrap();

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

	pub fn get_false(&self) -> ExpressionPointer {
		self.scope.get_variable_from_id("false", ScopeId::global()).unwrap()
	}

	pub fn get_true(&self) -> ExpressionPointer {
		self.scope.get_variable_from_id("true", ScopeId::global()).unwrap()
	}

	pub fn none(&self) -> ExpressionPointer {
		self.scope.get_variable_from_id("none", ScopeId::global()).unwrap()
	}
}
