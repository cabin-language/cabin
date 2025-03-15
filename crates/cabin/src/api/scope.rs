use std::{
	collections::HashMap,
	fmt::Debug,
	ops::{Deref, DerefMut},
};

use super::io::{IoReader, IoWriter};
use crate::{api::context::Context, ast::expressions::name::Name, comptime::memory::ExpressionPointer, parser::ParseError};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

impl ScopeId {
	pub const fn global() -> Self {
		Self(0)
	}

	pub const fn stdlib() -> Self {
		Self(1)
	}
}

/// A type of scope in the language. Currently, this is only used for debugging purposes, as scopes are able to be printed as a string representation,
/// and doing so will show their type. However, in the future, this may be used for other purposes, so it's good to leave here regardless
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeType {
	/// The function declaration scope type. This is used for the body of a function declaration. Note that this is not in any way related to a scope that
	/// a function is declared in, but represents the scope *inside* of a function's body.
	Function,
	Extend,
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
	variables: HashMap<Name, ExpressionPointer>,

	/// in Rust with regular semantics can get *really* tricky - You need to either resort to lots of unsafe code with raw pointers (and probably pinning),
	/// or use some fancy reference counting wrappers like `Rc<RefCell<Scope>>` and `Weak<RefCell<Scope>>`. Even in doing so, the implementation is not
	/// trivial. Using indices and an arena `Vec` is likely the best option.
	index: usize,

	/// The type of this scope. This is currently only used for debugging purposes when calling `to_string` on this scope, which will print its type among
	/// other information. However, this may in the future be used for more.
	scope_type: ScopeType,

	label: Option<Name>,
}

impl Scope {
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
	fn get_variable<'scopes>(&'scopes self, name: impl Into<Name> + Clone, scopes: &'scopes [Self]) -> Option<ExpressionPointer> {
		self.variables
			.get(&name.clone().into())
			.copied()
			.or_else(|| self.parent.and_then(|parent| scopes.get(parent).unwrap().get_variable(name, scopes)))
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
	fn reassign_variable_direct(&mut self, name: &Name, value: ExpressionPointer) -> Result<(), ExpressionPointer> {
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
pub struct ScopeTree {
	/// The arena of scopes stored as a flat vector. For more information, see the documentation on the `ScopeData` struct.
	scopes: Vec<Scope>,
	/// The id of the current scope. This is guaranteed to always point to a valid scope, and by default is the global scope.
	current_scope: usize,
}

impl ScopeTree {
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
			}],
			current_scope: 0,
		}
	}

	pub(crate) fn get_variable(&self, name: impl Into<Name> + Clone) -> Option<ExpressionPointer> {
		self.current().get_variable(name, &self.scopes)
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

	/// of its parents; As long as it exists in the current scope, the information will be retrieved. If no variable exists in the scope with the given id
	/// with the given name, `None` is returned.
	///
	/// # Parameters
	/// - `name` - The name of the variable to get the information of
	///
	/// # Returns
	/// A reference to the variable declaration, or `None` if the variable does not exist in the current scope.
	pub fn get_variable_from_id(&self, name: impl Into<Name> + Clone, id: ScopeId) -> Option<ExpressionPointer> {
		self.get_scope_from_id(id).get_variable(name, &self.scopes)
	}

	pub fn get_builtin(&self, name: impl Into<Name> + Clone) -> Option<ExpressionPointer> {
		self.get_variable_from_id(name, ScopeId::stdlib())
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
	pub fn declare_new_variable_from_id(&mut self, name: impl Into<Name>, value: ExpressionPointer, id: ScopeId) -> Result<(), crate::Error> {
		let name = name.into();
		let old = self.scopes.get_mut(id.0).unwrap().variables.insert(name.clone(), value);
		old.map_or(Ok(()), |_| {
			Err(crate::Error::Parse(ParseError::DuplicateVariableDeclaration {
				name: name.unmangled_name().to_owned(),
			}))
		})
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
	pub fn declare_new_variable(&mut self, name: impl Into<Name>, value: ExpressionPointer) -> Result<(), crate::Error> {
		self.declare_new_variable_from_id(name, value, ScopeId(self.current_scope))
	}

	/// as only the current scope is stored, so this operation is `O(n)`, where `n` is the height of the scope tree.
	///
	/// # Returns
	/// An immutable reference to the global scope stored in this scope tree.
	pub fn get_global_scope(&self) -> &Scope {
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
	pub fn reassign_variable_from_id(&mut self, name: &Name, mut value: ExpressionPointer, id: ScopeId) {
		// Traverse up the parent tree looking for the declaration and reassign it
		let mut current = Some(id.0);
		while let Some(current_index) = current {
			// If we find it, we're done (return Ok), if not, we continue
			match self.scopes.get_mut(current_index).unwrap().reassign_variable_direct(name, value) {
				Ok(()) => return,
				Err(returned_value) => value = returned_value,
			}
			current = self.scopes.get(current_index).unwrap().parent;
		}

		// No variable found
		// bail_err! {
		// 	base = format!("attempting to reassign the variable \"{name}\", but no variable with the name \"{name}\" exists in this scope", name = name.unmangled_name().cyan()),
		// };
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
	pub fn reassign_variable(&mut self, name: &Name, value: ExpressionPointer) {
		self.reassign_variable_from_id(name, value, ScopeId(self.current_scope));
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

	pub fn has_variable(&self, name: &Name) -> bool {
		self.get_variable(name).is_some()
	}
}

impl Deref for ScopeTree {
	type Target = Scope;

	fn deref(&self) -> &Self::Target {
		self.current()
	}
}

impl DerefMut for ScopeTree {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.current_mut()
	}
}

impl Debug for ScopeTree {
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

impl ScopeReverter {
	pub fn revert<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) {
		context.scope_tree.current_scope = self.0 .0;
	}
}
