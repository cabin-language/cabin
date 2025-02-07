use std::{collections::HashMap, fmt::Debug};

use colored::Colorize as _;

use super::{context::context, traits::TryAs as _};
use crate::{
	bail_err,
	comptime::memory::VirtualPointer,
	debug_log,
	parser::{
		expressions::{name::Name, Expression, Spanned},
		statements::use_extend::DefaultExtend,
		ParseError,
	},
	DiagnosticInfo,
	Error,
};

/// Scopes never get deleted, so all `ScopeIds` are always guaranteed to point to a valid `Scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeId(usize);

/// A type of scope in the language. Currently, this is only used for debugging purposes, as scopes are able to be printed as a string representation,
/// and doing so will show their type. However, in the future, this may be used for other purposes, so it's good to leave here regardless
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeType {
	/// The function declaration scope type. This is used for the body of a function declaration. Note that this is not in any way related to a scope that
	/// a function is declared in, but represents the scope *inside* of a function's body.
	Function,
	RepresentAs,
	File,
	Directory,
	Either,
	OneOf,
	/// The global scope type. This should only ever be used on a single scope in the whole program: The global scope.
	Global,
	/// The group scope type. This is used for inside group declarations. The only variables that are added to
	/// scopes of this type are compile-time parameters on groups.
	Group,
	/// The block scope type. This is used for the inside of expression blocks.
	Block,
}

/// A scope in the language. This is a node in the overall scope tree that's stored in `ScopeData`. Scopes represent a section of code in curly braces
/// that has variables declared in it which are not accessible outside of it. They are, however, accessible to any child scopes declared within it,
/// meaning that this scope also inherits variables from its parent. One important thing to note is that Cabin doesn't support any kind of shadowing -
/// meaning globally declared variables are available in *every* scope. No matter what scope you're in, you can be 100% certain there is a `String`
/// variable defined, and that it is exactly what you expect it to be. This is important for resolving things like `Boolean`s.
pub struct Scope {
	/// The index of the scope which is the parent to this one. This is the scope's direct parent, i.e., the scope in which this one is declared in. This
	/// is represented as an index into a `ScopeData`'s `scopes` vector, because trying to create a tree data structure in Rust with regular semantics
	/// can get *really* tricky - You need to either resort to lots of unsafe code with raw pointers (and probably pinning), or use some fancy
	/// reference counting wrappers like `Rc<RefCell<Scope>>` and `Weak<RefCell<Scope>>`. Even in doing so, the implementation is not trivial. Using
	/// indices and an arena `Vec` is likely the best option.
	///
	/// This is optional because the global scope has no parent. On all scopes except the global scope, this is guaranteed to be `Some`. On the global scope,
	/// this is guaranteed to be `None`.
	pub parent: Option<usize>,

	/// The indices of the child scopes of this one. This is the scope's direct children, i.e., the scopes in which this one declared them. This
	/// is represented as an index into a `ScopeData`'s `scopes` vector, because trying to create a tree data structure in Rust with regular semantics
	/// can get *really* tricky - You need to either resort to lots of unsafe code with raw pointers (and probably pinning), or use some fancy
	/// reference counting wrappers like `Rc<RefCell<Scope>>` and `Weak<RefCell<Scope>>`. Even in doing so, the implementation is not trivial. Using
	/// indices and an arena `Vec` is likely the best option.
	children: Vec<usize>,

	/// The variables declared in this scope. Note that this only holds the variables declared in this exact specific scope, and does not count the
	/// variables declared in any parent scope, even though those are accessible in the language from this one. To get a variable from anywhere up
	/// the parent tree, use `scope.get_variable`, which will traverse up the scope tree and check all parents.
	variables: HashMap<Name, Expression>,

	/// The index of this scope. This is represented as an index into a `ScopeData`'s `scopes` vector, because trying to create a tree data structure
	/// in Rust with regular semantics can get *really* tricky - You need to either resort to lots of unsafe code with raw pointers (and probably pinning),
	/// or use some fancy reference counting wrappers like `Rc<RefCell<Scope>>` and `Weak<RefCell<Scope>>`. Even in doing so, the implementation is not
	/// trivial. Using indices and an arena `Vec` is likely the best option.
	index: usize,

	/// The type of this scope. This is currently only used for debugging purposes when calling `to_string` on this scope, which will print its type among
	/// other information. However, this may in the future be used for more.
	scope_type: ScopeType,

	label: Option<Name>,

	default_extensions: Vec<DefaultExtend>,
}

impl Scope {
	pub fn default_extensions<'scopes>(&'scopes self, scopes: &'scopes [Scope]) -> Vec<&'scopes DefaultExtend> {
		// TODO: check for real type overlap here
		let mut extensions = self.parent.map(|parent| scopes.get(parent).unwrap().default_extensions(scopes)).unwrap_or(Vec::new());
		extensions.retain(|parent_extension| {
			for extension in &self.default_extensions {
				if parent_extension.type_to_extend.try_as::<VirtualPointer>().unwrap() == extension.type_to_extend.try_as::<VirtualPointer>().unwrap() {
					return false;
				}
			}
			true
		});
		extensions.extend(&self.default_extensions);
		extensions
	}

	pub fn map_default_extension<E, F: FnOnce(DefaultExtend) -> Result<DefaultExtend, E>>(&mut self, id: usize, map: F) -> Result<(), E> {
		let index = self
			.default_extensions
			.iter()
			.enumerate()
			.find_map(|(index, extension)| (extension.id == id).then_some(index))
			.unwrap();

		let value = self.default_extensions.remove(index);
		let result = map(value)?;
		self.default_extensions.insert(index, result);
		Ok(())
	}

	/// Returns the information about a variable declared in this scope with the given name. Note that this only checks variables declared exactly
	/// in this scope, and does not check parents of this scope, meaning this cannot give accurate information about whether a variable exists in
	/// the current scope; To get a variable from the current scope, use `get_variable()`, which traverses up the scope tree and recursively checks parents.
	///
	/// # Parameters
	/// - `name` - The name of the variable declared in this scope to get information about
	///
	/// # Returns
	/// A reference to the declaration data of the variable declared in this scope with the given name. If none exists, `None` is returned. If it does exist
	/// and `Some` is returned, the returned reference will have the same lifetime as this `Scope` object.

	pub fn get_variable_direct(&self, name: &Name) -> Option<&Expression> {
		self.variables.get(name)
	}

	/// Returns the information about a variable that exists in this scope with the given name. Note that this will also traverse up the scope tree and check
	/// parent scopes for a variable with the given name if this scope doesn't have a variable with the given name. To get information about a variable that
	/// was declared specifically in this scope, use `get_variable_direct()`. If the function traverses all the way to the global scope and the variable is
	/// not found, `None` is returned.
	///
	/// # Parameters
	/// - `<'scopes>` - The lifetime of the scopes slice passed. The returned declaration reference will have this lifetime.
	/// - `name` - The name of the variable that exists in this scope to get information about
	/// - `scopes` - The scope tree as an area vector slice; This is passed from a `ScopeData` object.
	///
	/// # Returns
	/// A reference to the declaration data of the variable that exists in this scope with the given name. If none exists, `None` is returned. If it does exist
	/// and `Some` is returned, the returned reference will have the same lifetime as this `Scope` object, as well as the given scopes slice.

	pub fn get_variable<'scopes>(&'scopes self, name: impl Into<Name> + Clone, scopes: &'scopes [Self]) -> Option<&'scopes Expression> {
		self.variables
			.get(&name.clone().into())
			.or_else(|| self.parent.and_then(|parent| scopes.get(parent).unwrap().get_variable(name, scopes)))
	}

	/// Returns all variables that are available in this scope, including variables declared in ancestor scopes.
	/// This traverses up the scope tree up to and including the global scope, and returns all variables declared
	/// in those scopes.
	///
	/// # Parameters
	/// - `<'scopes>` - The lifetime of the given `Scope` slice. This must be the same as the lifetime of `&self`,
	/// and it will also be the lifetime of the returned references.
	/// - `scopes` - The slice of scopes available from a `ScopeData` object.
	///
	/// # Returns
	/// All variables that exist in this scope, including those declared in ancestor scopes.
	pub fn get_variables<'scopes>(&'scopes self, scopes: &'scopes [Self]) -> Vec<(&'scopes Name, &'scopes Expression)> {
		let mut variables = self.variables.iter().collect::<Vec<_>>();
		if let Some(parent) = self.parent {
			for variable in scopes.get(parent).unwrap().get_variables(scopes) {
				variables.push(variable);
			}
		}

		variables
	}

	/// Reassigns a variable in this scope. This will NOT traverse up the scope tree through the current scope's parents to find the declaration for the given
	/// variable name; it will only reassign a variable declared in this scope. This is only to be used to reassign an existing variable. To add a new variable,
	/// use `declare_new_variable_direct()`. If no variable with the given name is found, an error is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to reassign. A variable with this name must be declared in this scope, otherwise, an `Err` wil be returned.
	/// - `value` - The new value to set the variable to
	///
	/// # Returns
	/// An `Err` if no variable with the given name exists in the current scope.
	fn reassign_variable_direct(&mut self, name: &Name, value: Expression) -> Result<(), Expression> {
		if let Some(variable) = self.variables.get_mut(name) {
			*variable = value;
			Ok(())
		} else {
			Err(value)
		}
	}

	/// Converts this scope to a debug string representation. This requires the `Scope` slice because it needs to print information about it's children,
	/// which are only stored in the variable as an id (usize) (see the `Scope` struct for reasoning behind this).
	///
	/// # Parameters
	/// - `scopes` - The arena vector of scopes provided by `ScopeData`.
	///
	/// # Returns
	/// A string representation of this scope to debug programs.

	pub fn to_string(&self, scopes: &[Self]) -> String {
		let mut string = vec!["{".to_owned()];
		string.push(format!("\ttype: [{:?}]", self.scope_type));
		string.push(format!("\tindex: [{}]", self.index));
		string.push(format!("\tlabel: [{:?}]", self.label));
		string.push(format!(
			"\tvariables: [{}],",
			self.variables.keys().map(|name| name.unmangled_name()).collect::<Vec<_>>().join(",")
		));
		for child_scope in &self.children {
			for line in scopes.get(*child_scope).unwrap().to_string(scopes).lines() {
				string.push(format!("\t{line}"));
			}
		}

		string.push("}".to_owned());
		string.join("\n")
	}

	pub fn set_label(&mut self, name: Name) {
		self.label = Some(name);
	}
}

/// Current scope data for the language. The scopes in the language are a tree data structure, with the root being the global scope. This can be hard to implement
/// in Rust; Some common strategies are using lots of `unsafe` code or using lots of wrapper structures like `Rc<RefCell<Scope>>` and `Weak<RefCell<Scope>>`. Even
/// with these, the implementation is not trivial. The easiest solution is using an "arena allocation" - meaning a single flat vector that holds all scopes,
/// and then "references" to scopes are just indices into that vector. This is a particularly simple pattern to implement here because scopes are never
/// destroyed or removed, so their indices act as permanent unique IDs.
///
/// This acts simply as a wrapper around the scope arena vector, as well as keeping track of the current scope, be it during parsing, compile-time, etc.
pub struct ScopeData {
	/// The arena of scopes stored as a flat vector. For more information, see the documentation on the `ScopeData` struct.
	scopes: Vec<Scope>,
	/// The id of the current scope. This is guaranteed to always point to a valid scope, and by default is the global scope.
	current_scope: usize,
}

impl ScopeData {
	/// Creates a new scope data with an empty global scope. This should only be used once in each program to create the main scope data.
	/// The current scope is set to the global scope.
	///
	/// # Returns
	/// A newly created scope data object with an empty global scope.

	pub fn global() -> Self {
		Self {
			scopes: vec![Scope {
				scope_type: ScopeType::Global,
				index: 0,
				children: Vec::new(),
				variables: HashMap::new(),
				parent: None,
				label: None,
				default_extensions: Vec::new(),
			}],
			current_scope: 0,
		}
	}

	/// Returns an immutable reference to the current scope.
	///
	/// # Returns
	/// An immutable reference to the current scope (did you really have to ask?)
	fn current(&self) -> &Scope {
		self.scopes.get(self.current_scope).unwrap()
	}

	/// Returns a mutable reference to the current scope.
	///
	/// # Returns
	/// A mutable reference to the current scope
	fn current_mut(&mut self) -> &mut Scope {
		self.scopes.get_mut(self.current_scope).unwrap()
	}

	/// Returns a reference to the scope with the given id. If none exists, `None` is returned. This is `O(1)`.
	///
	/// # Parameters
	/// - `id` - The id of the scope to get
	///
	/// # Returns
	/// An immutable reference to the scope with this id, or `None` if no scope exists with the given id.

	pub fn get_scope_from_id(&self, id: ScopeId) -> &Scope {
		self.scopes.get(id.0).unwrap()
	}

	pub fn get_scope_mut_from_id(&mut self, id: ScopeId) -> &mut Scope {
		self.scopes.get_mut(id.0).unwrap()
	}

	/// Returns the declaration information about a variable that exists in the current scope. The variable may be declared in this scope, or any one of its parents;
	/// As long as it exists in the current scope, the information will be retrieved. If no variable exists in the current scope with the given name,
	/// `None` is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to get the information of
	///
	/// # Returns
	/// A reference to the variable declaration, or `None` if the variable does not exist in the current scope.

	pub fn get_variable(&self, name: impl Into<Name> + Clone) -> Option<&Expression> {
		self.current().get_variable(name, &self.scopes)
	}

	/// Returns the declaration information about a variable that exists in the scope with the given id. The variable may be declared in this scope, or any one
	/// of its parents; As long as it exists in the current scope, the information will be retrieved. If no variable exists in the scope with the given id
	/// with the given name, `None` is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to get the information of
	///
	/// # Returns
	/// A reference to the variable declaration, or `None` if the variable does not exist in the current scope.

	pub fn get_variable_from_id(&self, name: impl Into<Name> + Clone, id: ScopeId) -> Option<&Expression> {
		self.get_scope_from_id(id).get_variable(name, &self.scopes)
	}

	/// Enters a new scope. This creates a new scope with the given scope type, and sets the current scope to be that one. The newly created scope is added
	/// to the children of this scope, and its parent will be this scope. When you're done with this scope, use `exit_scope()`.
	pub fn enter_new_scope(&mut self, scope_type: ScopeType) {
		self.scopes.push(Scope {
			variables: HashMap::new(),
			index: self.scopes.len(),
			parent: Some(self.current_scope),
			children: Vec::new(),
			scope_type,
			label: None,
			default_extensions: Vec::new(),
		});

		let new_id = self.scopes.len() - 1;
		self.current_mut().children.push(new_id);
		self.current_scope = self.scopes.len() - 1;
	}

	/// Exits the current scope. This sets the current scope of this scope data to be the parent of the current scope. This will only return an `Err` if
	/// the current scope is the global scope, which has no parent and should never be exited. This should only ever be used after an accompanying
	/// `enter_new_scope()` call.
	///
	/// # Errors
	///
	/// If this is currently the global scope
	pub fn exit_scope(&mut self) -> anyhow::Result<()> {
		self.current_scope = self.current().parent.ok_or_else(|| anyhow::anyhow!("Attempted to exit global scope"))?;
		Ok(())
	}

	/// Returns the unique ID of the current scope. This is the index of the current scope in this `ScopeData`'s arena `Scope` vector. Because scopes are never
	/// deleted or removed from this vector, this is a persistent unique ID throughout the duration of the program. This is guaranteed to return a value that
	/// will always be the index of a valid scope (and the same scope that is current when this is called) in the scope vector.
	///
	/// # Returns
	/// The unique ID of the current scope

	pub const fn unique_id(&self) -> ScopeId {
		ScopeId(self.current_scope)
	}

	/// Sets the current scope to the given id, and returns the previous scope id. This is used for things like function calls, where the current scope is
	/// temporarily set to the id of the scope inside the function declaration, and then reverted back to the previous scope when returning to the caller.
	///
	/// # Parameters
	/// - `id` - The id of the scope to set
	///
	/// # Returns
	/// The id of the previously current scope
	pub fn set_current_scope(&mut self, id: ScopeId) -> ScopeReverter {
		let previous = self.current_scope;
		self.current_scope = id.0;
		ScopeReverter(ScopeId(previous))
	}

	/// Declares a new variable in the scope with the given id with the given value and tags. This should only be used to declare a new variable,
	/// not reassign an existing one. To reassign an existing variable, use `reassign_variable_from_id()`. To declare a new variable in the current scope,
	/// use `declare_new_variable()`.
	///
	/// # Parameters
	/// - `name` - The name of the variable to declare. It must be unused in the scope with the given id, including its parent scopes, or an error will be returned.
	/// - `value` - The value of the variable to set. All variables must be initialized with a value.
	/// - `tags` - The tags on the variable declaration.
	/// - `id` - The id of the scope to declare the variable in.
	///
	/// # Errors
	/// Returns an error if a variable already exists with the given name in the scope with the given id.
	pub fn declare_new_variable_from_id(&mut self, name: impl Into<Name>, value: Expression, id: ScopeId) -> Result<(), crate::Diagnostic> {
		let name = name.into();
		debug_log!(
			"Declaring a new variable called {} in a scope of type {:?}",
			name.unmangled_name().red(),
			self.get_scope_from_id(id).scope_type
		);
		if self.get_variable_from_id(name.clone(), id).is_some() {
			return Err(crate::Diagnostic {
				span: name.span(),
				error: DiagnosticInfo::Error(Error::Parse(ParseError::DuplicateVariableDeclaration {
					name: name.unmangled_name().to_owned(),
				})),
			});
		}
		let _ = self.scopes.get_mut(id.0).unwrap().variables.insert(name, value);
		Ok(())
	}

	/// Declares a new variable in the current scope with the given value and tags. This should only be used to declare a new variable,
	/// not reassign an existing one. To reassign an existing variable, use `reassign_variable()`. To declare a new variable in a scope with a specific id,
	/// use `declare_new_variable_from_id()`.
	///
	/// # Parameters
	/// - `name` - The name of the variable to declare. It must be unused in the current scope, including its parent scopes, or an error will be returned.
	/// - `value` - The value of the variable to set. All variables must be initialized with a value.
	/// - `tags` - The tags on the variable declaration.
	///
	/// # Errors
	/// Returns an error if a variable already exists with the given name in the current scope.
	pub fn declare_new_variable(&mut self, name: impl Into<Name>, value: Expression) -> Result<(), crate::Diagnostic> {
		self.declare_new_variable_from_id(name, value, ScopeId(self.current_scope))
	}

	/// Returns an immutable reference to the global scope in this scope data's scope arena. This does have to traverse up the scope tree,
	/// as only the current scope is stored, so this operation is `O(n)`, where `n` is the height of the scope tree.
	///
	/// # Returns
	/// An immutable reference to the global scope stored in this scope tree.
	fn get_global_scope(&self) -> &Scope {
		let mut current = self.current();
		while let Some(parent_index) = current.parent {
			current = self.scopes.get(parent_index).unwrap();
		}
		current
	}

	/// Reassigns a variable in the scope with the given id. This will traverse up the scope tree through the scope's parents to find the declaration for the given
	/// variable name, and reassign the value. This is only to be used to reassign an existing variable. To add a new variable, use `add_variable()`. To
	/// reassign a variable declared in the current scope, use `reassign_variable()`. If the function traverses all the way into the global scope
	/// and no variable with the given name is found, an error is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to reassign. A variable with this name must exist in the current scope, (meaning it's declared here or in a parent
	/// scope), otherwise, an `Err` wil be returned.
	/// - `value` - The new value to set the variable to
	///
	/// # Errors
	/// Errors if no variable with the given name exists in the current scope.
	pub fn reassign_variable_from_id(&mut self, name: &Name, mut value: Expression, id: ScopeId) -> anyhow::Result<()> {
		debug_log!(
			"Reassigning the variable called {} in a scope of type {:?} to be {value:?}",
			name.unmangled_name().red(),
			self.get_scope_from_id(id).scope_type
		);
		// Traverse up the parent tree looking for the declaration and reassign it
		let mut current = Some(id.0);
		while let Some(current_index) = current {
			// If we find it, we're done (return Ok), if not, we continue
			match self.scopes.get_mut(current_index).unwrap().reassign_variable_direct(name, value) {
				Ok(()) => return Ok(()),
				Err(returned_value) => value = returned_value,
			}
			current = self.scopes.get(current_index).unwrap().parent;
		}

		// No variable found
		bail_err! {
			base = format!("attempting to reassign the variable \"{name}\", but no variable with the name \"{name}\" exists in this scope", name = name.unmangled_name().cyan()),
		};
	}

	/// Reassigns a variable in the current scope. This will traverse up the scope tree through the current scope's parents to find the declaration for the given
	/// variable name, and reassign the value. This is only to be used to reassign an existing variable. To add a new variable, use `add_variable()`. To
	/// reassign a variable declared in this specific scope, use `reassign_variable_from_id()`. If the function traverses all the way into the global scope
	/// and no variable with the given name is found, an error is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to reassign. A variable with this name must exist in the current scope, (meaning it's declared here or in a parent
	/// scope), otherwise, an `Err` wil be returned.
	/// - `value` - The new value to set the variable to
	///
	/// # Errors
	/// Returns an error if no variable with the given name exists in the current scope.
	pub fn reassign_variable(&mut self, name: &Name, value: Expression) -> anyhow::Result<()> {
		self.reassign_variable_from_id(name, value, ScopeId(self.current_scope))
	}

	/// Returns the variables in the current scope which have the closest names to the given name. This is
	/// used by the compiler to suggest variables with close names when the user attempts to reference a variable
	/// that can't be found.
	///
	/// # Returns
	/// a sorted list, which are sorted by how close they are to the given variable name.

	pub fn get_variables(&self) -> Vec<(&Name, &Expression)> {
		self.current().get_variables(&self.scopes)
	}

	/// Returns the variables in the current scope which have the closest names to the given name. This is
	/// used by the compiler to suggest variables with close names when the user attempts to reference a variable
	/// that can't be found.
	///
	/// # Parameters
	/// - `name` - The name of the variable to get close to
	/// - `max` - The maximum amount of variables to return in the output.
	///
	/// # Returns
	/// a sorted list, of at most `max` elements, which are sorted by how close they are to the given variable
	/// name.

	pub fn get_closest_variables(&self, name: &Name, max: usize) -> Vec<(&Name, &Expression)> {
		let mut all_variables = self.get_variables();
		all_variables.sort_by(|(first, _), (second, _)| {
			first
				.unmangled_name()
				.distance_to(name.unmangled_name())
				.cmp(&second.unmangled_name().distance_to(name.unmangled_name()))
		});

		all_variables.get(0..max).map(|slice| slice.to_vec()).unwrap_or(all_variables)
	}

	/// Returns the unique ID of the scope of the current file.
	///
	/// # Returns
	///
	/// The id of the scope of the current file.

	pub fn file_id(&self) -> ScopeId {
		let mut current = self.current();
		while current.scope_type != ScopeType::File {
			current = self.get_scope_from_id(ScopeId(*current.parent.as_ref().unwrap()));
		}
		ScopeId(current.index)
	}

	pub fn scope_type_of(&self, label: &Name) -> anyhow::Result<&ScopeType> {
		let mut current = self.current();
		while current.label != Some(label.to_owned()) {
			if let Some(parent) = current.parent {
				current = self.scopes.get(parent).unwrap();
			} else {
				anyhow::bail!("No scope found with the label \"{}\"", label.unmangled_name().bold().cyan())
			}
		}

		Ok(&current.scope_type)
	}

	pub const fn get_stdlib_id() -> ScopeId {
		ScopeId(1)
	}

	pub fn new_scope_id(&mut self, scope_type: ScopeType) -> ScopeId {
		self.enter_new_scope(scope_type);
		let id = ScopeId(self.current_scope);
		self.exit_scope().unwrap();
		id
	}

	pub fn default_extensions(&self) -> Vec<&DefaultExtend> {
		self.current().default_extensions(&self.scopes)
	}

	pub fn add_default_extension(&mut self, extension: DefaultExtend) {
		self.current_mut().default_extensions.push(extension);
	}

	pub fn map_default_extension_from_id<E, F: FnOnce(DefaultExtend) -> Result<DefaultExtend, E>>(&mut self, scope_id: ScopeId, extend_id: usize, map: F) -> Result<(), E> {
		self.get_scope_mut_from_id(scope_id).map_default_extension(extend_id, map)
	}
}

impl Debug for ScopeData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: CURRENT = {}", self.get_global_scope().to_string(&self.scopes), self.current_scope)
	}
}

/// A trait to apply Levenshtein string distance functionality to strings. This allows strings to calculate how close
/// they are to another string, which is used by the compiler to give suggestions for variables with close names
/// when a variable can't be found.
pub trait Levenshtein {
	/// Returns the Levenshtein distance between this string and another string. This distance is smaller the closer
	/// the two strings are. This is used by the compiler to give suggestions for variables with close names
	/// when a variable can't be found.
	fn distance_to(&self, other: &str) -> usize;
}

impl Levenshtein for str {
	/// Calculates how "close" two strings are. The returned value is the sum of the number of letter removals, additions, and
	/// substitutions it would take to get from one string to another.
	///
	/// This is used to get the "closest variables" to a given name - when the developer attempts to reference a variable that
	/// doesn't exist, the compiler suggests the closest ones.
	///
	/// # Parameters
	/// - `other` - The other string to get the distance to, relative to this string.
	///
	/// # Returns
	/// The numerical "distance" from this string to the other string. This result is commutative, so `s1.distance_to(s2)` is
	/// exactly equivalent to `s2.distance_to(s1)` in result. The greater the result, the further apart the strings are. Two
	/// equivalent strings will always return 0.
	///
	/// Algorithm adapted from [`https://en.wikipedia.org/wiki/Levenshtein_distance#Iterative_with_two_matrix_rows`]
	fn distance_to(&self, other: &str) -> usize {
		let mut insertion_cost: usize;
		let mut deletion_cost: usize;
		let mut substitution_cost: usize;

		let mut dummy;
		let first_length = self.len();
		let second_length = other.len();

		let mut v0 = Vec::new();
		let mut v1 = Vec::new();

		for index in 0..=second_length {
			v0.push(index);
		}

		for index in 0..first_length {
			if v1.is_empty() {
				v1.push(0);
			}
			*v1.get_mut(0).unwrap() = index + 1;

			for j in 0..second_length {
				deletion_cost = v0.get(j + 1).unwrap() + 1;
				insertion_cost = v1.get(j).unwrap() + 1;

				substitution_cost = if self.chars().nth(index) == other.chars().nth(j) {
					*v0.get(j).unwrap()
				} else {
					v0.get(j).unwrap() + 1
				};

				while v1.len() <= j + 1 {
					v1.push(0);
				}

				*v1.get_mut(j + 1).unwrap() = [deletion_cost, insertion_cost, substitution_cost].into_iter().min().unwrap();
			}

			dummy = v0;
			v0 = v1;
			v1 = dummy;
		}

		*v0.get(second_length).unwrap()
	}
}

pub struct ScopeReverter(ScopeId);

impl Drop for ScopeReverter {
	fn drop(&mut self) {
		context().scope_data.current_scope = self.0 .0;
	}
}
