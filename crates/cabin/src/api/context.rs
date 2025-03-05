use std::path::PathBuf;

use super::traits::TryAs as _;
use crate::{
	api::{diagnostics::Diagnostic, scope::ScopeTree},
	ast::expressions::{
		name::Name,
		new_literal::{EvaluatedLiteral, Object},
		Expression,
	},
	comptime::memory::VirtualMemory,
	Diagnostics,
	STDLIB,
};

/// Global(ish) data about the state of the compiler. The context holds the program's scope data,
/// as well as memory where expressions are stored, among some other metadata like the file path
/// being evaluated.
pub struct Context {
	/// Scope information about the program. Scopes are stored in this tree, with each scope
	/// containing a map between variable names and their values (as `ExpressionPointers`)
	///
	/// This should never be reassigned, only mutated with the methods on it for declaring and
	/// reassigning variables.
	pub(crate) scope_tree: ScopeTree,

	/// Storage for expressions. All expressions are stored in `VirtualMemory`, and can be accessed
	/// with an `ExpressionPointer`, allowing one expression to be reused and mutated globally from
	/// different places in the user's code.
	pub(crate) virtual_memory: VirtualMemory,

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
	pub(crate) side_effects: bool,

	/// The path to the file currently being acted upon (tokenized/parsed/evaluated/transpiled etc.)
	pub(crate) file: PathBuf,

	/// Whether the user has printed to stdout or stderr at compile-time. This is stored because
	/// when the first line is printed (and the first line only), an additional empty line should
	/// be printed before it. Additionally, if any lines are printed, an additional newline is
	/// printed after compile-time evaluation.
	pub(crate) has_printed: bool,

	/// Diagnostic information about the user's code, such as warnings, errors, hints, etc.
	diagnostics: Diagnostics,

	pub(crate) name_query_result: Option<Name>,
	pub(crate) name_query: Option<usize>,
}

impl Default for Context {
	fn default() -> Self {
		let mut context = Context {
			scope_tree: ScopeTree::global(),
			virtual_memory: VirtualMemory::empty(),
			diagnostics: Diagnostics::empty(),
			side_effects: true,
			has_printed: false,
			file: "stdlib".into(),
			name_query: None,
			name_query_result: None,
		};

		// Add stdlib
		let stdlib_pointer =
			Expression::EvaluatedLiteral(EvaluatedLiteral::Object(crate::parse_library(STDLIB, &mut context).into_object(&mut context))).store_in_memory(&mut context);
		context.scope_tree.declare_new_variable("builtin", stdlib_pointer).unwrap();
		let Expression::EvaluatedLiteral(EvaluatedLiteral::Object(stdlib)) = stdlib_pointer.expression(&context).to_owned() else { unreachable!() };

		// Bring some stdib items into scope
		context.scope_tree.declare_new_variable("Text", stdlib.get_field("Text").unwrap().into()).unwrap();
		context.scope_tree.declare_new_variable("Number", stdlib.get_field("Number").unwrap().into()).unwrap();
		context
			.scope_tree
			.declare_new_variable(
				"print",
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("print")
					.unwrap()
					.into(),
			)
			.unwrap();
		context
			.scope_tree
			.declare_new_variable(
				"debug",
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("debug")
					.unwrap()
					.into(),
			)
			.unwrap();
		context
			.scope_tree
			.declare_new_variable(
				"input",
				stdlib
					.get_field("system")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("terminal")
					.unwrap()
					.get_literal(&context)
					.try_as::<Object>()
					.unwrap()
					.get_field("input")
					.unwrap()
					.into(),
			)
			.unwrap();

		context
	}
}

impl Context {
	/// Returns the diagnostics found in the user's code. Note that this only returns diagnostics
	/// that are already stored; It doesn't perform a new scan for diagnostics. Usually diagnostics
	/// will be fetched after performing an evaluation step on a `Project`.
	///
	/// # Returns
	///
	/// The diagnostics in the user's code
	pub fn diagnostics(&self) -> &Diagnostics {
		&self.diagnostics
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

	pub fn scope_tree(&self) -> &ScopeTree {
		&self.scope_tree
	}
}
