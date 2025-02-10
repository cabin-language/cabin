use crate::{
	api::{
		config_files::{CabinToml, CabinTomlWriteOnDrop},
		diagnostics::Diagnostic,
		project::Project,
		scope::ScopeData,
	},
	cli::theme::Theme,
	comptime::memory::VirtualMemory,
	Diagnostics,
};

pub struct Context {
	// Publicly mutable
	pub scope_data: ScopeData,
	pub virtual_memory: VirtualMemory,
	pub lines_printed: usize,
	pub theme: Theme,
	pub phase: Phase,
	pub project: Option<Project>,

	// Privately mutable
	side_effects_stack: Vec<bool>,
	options: CabinToml,
	diagnostics: Diagnostics,
}

impl Default for Context {
	fn default() -> Self {
		Context {
			phase: Phase::Stdlib,
			options: CabinToml::default(),
			scope_data: ScopeData::global(),
			virtual_memory: VirtualMemory::empty(),
			side_effects_stack: Vec::new(),
			lines_printed: 0,
			theme: Theme::default(),
			diagnostics: Diagnostics::empty(),
			project: None,
		}
	}
}

impl Context {
	pub fn reset(&mut self) {
		*self = Self::default();
	}

	pub fn toggle_side_effects(&mut self, side_effects: bool) {
		self.side_effects_stack.push(side_effects);
	}

	pub fn untoggle_side_effects(&mut self) {
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

	pub const fn config(&self) -> &CabinToml {
		&self.options
	}

	/// Returns a mutable reference to the data stored in the project's `cabin.toml`. If the user is running a single
	/// Cabin file and not in a project, an error is returned. When this value is dropped, the `cabin.toml` file is
	/// written to update to the contents of the returned object.
	///
	/// # Errors
	///
	/// If the compiler is currently operating on a single file instead of in a project that contains options, since
	/// single Cabin files can't contain compiler configuration.
	pub fn cabin_toml_mut(&mut self) -> anyhow::Result<CabinTomlWriteOnDrop> {
		Ok(CabinTomlWriteOnDrop::new(&mut self.options, self.project.as_ref().unwrap().root_directory().to_owned()))
	}
}

#[derive(Debug, Clone)]
pub struct SourceFilePosition {
	/// The line of the position.
	line: u32,

	/// The column of the position.
	column: u32,

	/// The name of the source file.
	name: &'static str,

	/// The fully qualified path name of the Rust function this location takes place in. This is
	/// generally obtained via the `function!()` macro from `crate::api::macros`.
	function: String,
}

impl SourceFilePosition {
	pub const fn new(line: u32, column: u32, name: &'static str, function: String) -> Self {
		Self { line, column, name, function }
	}

	pub const fn line(&self) -> u32 {
		self.line
	}

	pub const fn column(&self) -> u32 {
		self.column
	}

	pub const fn file_name(&self) -> &'static str {
		self.name
	}

	pub fn function_name(&self) -> String {
		self.function.clone()
	}
}

/// A phase in compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
	/// This phase represents when the compiler is parsing and evaluating the Cabin standard
	/// library. This is the very first phase in compilation.
	Stdlib,

	/// The phase for when the compiler reads all of the source files in a Cabin project.
	ReadingSourceFiles,

	/// The phase for when the compiler tokenizes source code into a token stream.
	Tokenization,

	/// The phase for when the compiler parses its token stream into an abstract syntax tree.
	Parsing,

	/// The phase for when the compiler links all files' ASTs together into a single AST.
	Linking,

	/// The phase for when the compiler evaluates its ASTs at compile-time.
	CompileTimeEvaluation,

	/// The phase for when the compiler transpiles its evaluated ASTs into C code.
	Transpilation,

	/// The phase for when the compiler compiles C code into a native binary.
	Compilation,

	/// The phase for when the compiler runs a compiled native binary.
	RunningBinary,
}

impl Phase {
	/// Returns what this phase is doing as a tuple of two strings; The first being the verb for
	/// what the phase does and the second being the object for what the phase is acting on. This
	/// is used by `crate::cli::commands::step()` to pretty-print information as compilation
	/// happens.
	pub const fn action(&self) -> (&'static str, &'static str) {
		match self {
			Phase::Stdlib => ("Adding", "standard library"),
			Phase::ReadingSourceFiles => ("Reading", "source files"),
			Phase::Tokenization => ("Tokenizing", "source code"),
			Phase::Parsing => ("Parsing", "token stream"),
			Phase::Linking => ("Linking", "source files"),
			Phase::CompileTimeEvaluation => ("Running", "compile-time code"),
			Phase::Transpilation => ("Transpiling", "program to C"),
			Phase::Compilation => ("Compiling", "C code"),
			Phase::RunningBinary => ("Running", "executable"),
		}
	}
}
