use std::{collections::HashMap, fmt::Debug};

use try_as::traits::TryAsRef;

use crate::{
	api::{context::Context, scope::ScopeId, traits::TryAs as _},
	comptime::{memory::VirtualPointer, CompileTime},
	err,
	lexer::Span,
	parser::{
		expressions::{
			either::Either,
			extend::Extend,
			field_access::FieldAccessType,
			function_declaration::FunctionDeclaration,
			group::GroupDeclaration,
			name::Name,
			object::{InternalFieldValue, ObjectConstructor},
			oneof::OneOf,
			parameter::Parameter,
			Expression,
			Spanned,
			Typed,
		},
		statements::tag::TagList,
	},
};

/// A "literal object". Literal objects can be thought of as simple associative arrays, similar to a JSON object or similar.
/// Specifically, a literal object is a collection of fields where each field's value is another literal object.
///
/// You may notice that there's no `LiteralObject` variant of `Expression`. This is because literal objects live in "virtual memory",
/// and instead we refer to them with "virtual pointers" via the `Pointer` struct. You can read more about this in the documentation
/// for `VirtualMemory` and `context.virtual_memory`.
///
/// `LiteralObjects` are equivalent to types in Cabin. Cabin allows arbitrary expressions to be used as types, as long as "the entire
/// expression can be evaluated at compile-time" which just means that it can be evaluated down to a `LiteralObject`. If you want to
/// check or ensure that an expression is a type, check if it's a pointer to a `LiteralObject`.
///
/// Many constructs in Cabin are stored as `LiteralObject` that you might not expect. For example, all group declarations, either declarations,
/// function declarations, and one-of declarations are stored as literal objects. That's because at their core, all information about them is
/// known at compile-time. Any such object should be stored as a `LiteralObject`. Read the documentation on the `LiteralConvertible` trait for
/// more information about how these types of syntaxes are stored as and retrieved from `LiteralObjects`.
#[derive(Debug, Clone)]
pub struct LiteralObject {
	/// The type name of this `LiteralObject`. This is the name that the object would be constructed with in an object constructor, such as `Text`,
	/// `Number`, `Object`, etc.
	pub type_name: Name,

	/// The fields on this `LiteralObject`, as a map between field names and pointers to `LiteralObjects` as field values. This should be immutable
	/// after the object's creation; The whole point of being a literal is that it's known entirely at compile-time and won't change.
	pub fields: HashMap<Name, VirtualPointer>,

	/// The "internal" fields of this `LiteralObject`. These are special values that special types or objects need to store. These aren't accessible
	/// from within Cabin. For example, the `Text` group stores a `String` internally here, representing it's actual string value; `Number` behaves
	/// similarly.
	pub internal_fields: HashMap<String, InternalFieldValue>,

	pub field_access_type: FieldAccessType,

	pub outer_scope_id: ScopeId,
	pub inner_scope_id: Option<ScopeId>,

	pub name: Name,

	/// The address of this `LiteralObject` in memory. In theory, all `LiteralObjects` are stored in `VirtualMemory`, and thus have a unique
	/// address that points to them. This is an `Option`, however, because in theory a literal object could be constructed without being stored
	/// in memory for some reason, such as if a temporary value is needed. This is \*generally\* safe to `unwrap()`; It's only in rare exception
	/// cases that a `LiteralObject` will exist that doesn't live in `VirtualMemory`.
	///
	/// This is set to `Some` whenever the object is given to virtual memory, and `VirtualMemory` takes responsibility for updating it if it needs
	/// to be moved in memory or taken out of memory. See the `move_and_overwrite()` function on `VirtualMemory` for an example of this, which is
	/// called by `Declaration::evaluate_at_compile_time()`.
	pub address: Option<VirtualPointer>,

	pub span: Span,
	pub tags: TagList,
}

impl LiteralObject {
	pub fn empty(span: Span, context: &Context) -> Self {
		Self {
			type_name: "Object".into(),
			fields: HashMap::new(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: context.scope_data.unique_id(),
			inner_scope_id: None,
			address: None,
			span,
			name: "anonymous_object".into(),
			tags: TagList::default(),
		}
	}

	pub const fn type_name(&self) -> &Name {
		&self.type_name
	}

	pub const fn field_access_type(&self) -> &FieldAccessType {
		&self.field_access_type
	}

	pub fn is_error(&self) -> bool {
		self.address.is_some_and(|address| address == VirtualPointer::ERROR)
	}

	pub const fn name(&self) -> &Name {
		&self.name
	}

	pub fn get_field(&self, name: impl Into<Name>) -> Option<VirtualPointer> {
		self.fields.get(&name.into()).copied()
	}

	pub fn get_field_literal(&self, name: impl Into<Name>) -> Option<VirtualPointer> {
		self.fields.get(&name.into()).cloned()
	}

	pub fn expect_field_literal(&self, name: impl Into<Name>) -> VirtualPointer {
		self.get_field_literal(name).unwrap()
	}

	pub fn get_internal_field<T>(&self, name: &str) -> anyhow::Result<&T>
	where
		InternalFieldValue: TryAsRef<T>,
	{
		self.internal_fields
			.get(name)
			.ok_or_else(|| anyhow::anyhow!("Attempted to get an internal field that doesn't exist"))?
			.try_as::<T>()
	}

	/// Stores this value in virtual memory and returns a pointer to the location stored. Naturally, this consumes
	/// `self`, because virtual memory should own it's literal objects. To retrieve a reference of this object, use
	/// one of the methods on `VirtualMemory` with the returned pointer.
	///
	/// # Parameters
	/// - `context` - Global data about the current state of the compiler. In this case, it's used to access the compiler's
	/// virtual memory, which is stored on the context.
	///
	/// # Returns
	/// A pointer to the location of this literal object, which is now owned by the compiler's virtual memory.
	pub fn store_in_memory(self, context: &mut Context) -> VirtualPointer {
		context.virtual_memory.store(self)
	}

	pub const fn outer_scope_id(&self) -> ScopeId {
		self.outer_scope_id
	}

	pub fn dependencies(&self) -> Vec<VirtualPointer> {
		self.fields.values().map(|pointer| pointer.to_owned()).collect()
	}

	pub fn fields(&self) -> impl Iterator<Item = (&Name, &VirtualPointer)> {
		self.fields.iter()
	}

	pub fn has_any_fields(&self) -> bool {
		self.fields.is_empty()
	}

	/// Returns whether a value who's type is this literal, can be assigned to a name who's type is pointed to by the given pointer.
	pub fn is_this_type_assignable_to_type(&self, mut target_type: VirtualPointer, context: &Context) -> anyhow::Result<bool> {
		if target_type.virtual_deref(context).type_name() == &"Parameter".into() {
			let parameter = Parameter::from_literal(target_type.virtual_deref(context))?;
			target_type = *parameter.parameter_type().try_as::<VirtualPointer>()?;
		}

		let anything = *context.scope_data.get_variable("Anything").unwrap().try_as::<VirtualPointer>().unwrap();
		if target_type == anything {
			return Ok(true);
		}

		if self.address.unwrap() == target_type {
			return Ok(true);
		}

		Ok(false)
	}

	pub fn try_from_object(object: ObjectConstructor, context: &mut Context) -> Result<Self, anyhow::Error> {
		let mut fields = HashMap::new();
		for field in object.fields {
			let value = field.value.unwrap();
			if let Expression::Pointer(address) = value {
				let _ = fields.insert(field.name, address);
				continue;
			}

			let Expression::ObjectConstructor(field_object) = value else {
				anyhow::bail!("Value isn't an object: {value:?}");
			};

			let value_address = LiteralObject::try_from_object(field_object, context)?.store_in_memory(context);
			let _ = fields.insert(field.name, value_address);
		}

		Ok(LiteralObject {
			type_name: object.type_name,
			fields,
			internal_fields: object.internal_fields,
			field_access_type: object.field_access_type,
			outer_scope_id: object.outer_scope_id,
			inner_scope_id: Some(object.inner_scope_id),
			name: object.name,
			address: None,
			span: object.span,
			tags: object.tags.evaluate_at_compile_time(context),
		})
	}
}

impl TryAsRef<String> for LiteralObject {
	fn try_as_ref(&self) -> Option<&String> {
		self.get_internal_field("internal_value").ok()
	}
}

impl TryAsRef<f64> for LiteralObject {
	fn try_as_ref(&self) -> Option<&f64> {
		self.get_internal_field("internal_value").ok()
	}
}
impl TryAsRef<Vec<Expression>> for LiteralObject {
	fn try_as_ref(&self) -> Option<&Vec<Expression>> {
		self.get_internal_field("elements").ok()
	}
}

impl Typed for LiteralObject {
	fn get_type(&self, context: &mut Context) -> anyhow::Result<VirtualPointer> {
		let result = context
			.scope_data
			.get_variable(self.type_name.clone())
			.ok_or_else(|| {
				err! {
					base = format!("No variable found with the name {}", self.type_name().unmangled_name().red()),
				}
			})?
			.try_as::<VirtualPointer>()?
			.to_owned();

		Ok(result)
	}
}

impl CompileTime for LiteralObject {
	type Output = LiteralObject;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let address = self.address;
		let mut literal = match self.type_name().unmangled_name() {
			"Function" => FunctionDeclaration::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			"Group" => GroupDeclaration::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			"Either" => Either::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			"OneOf" => OneOf::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			"RepresentAs" => Extend::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			"Parameter" => Parameter::from_literal(&self).unwrap().evaluate_at_compile_time(context).to_literal(),
			_ => self,
		};
		literal.address = address;
		literal
	}
}

pub trait LiteralConvertible: Sized {
	/// Attempts to serialize `self` into a literal object.
	///
	/// For example, consider a `FunctionDeclaration`. Function declarations are their own struct with their own
	/// type information (return type, parameters, etc.), but within the language, they're just objects like everything
	/// else. They interact just like objects, meaning you can access fields on them, pass them as values, etc. For this
	/// reason, it's often helpful to be able to convert a type-safe function declaration object into a generic Cabin
	/// object; For example, the compiler's virtual memory only stores `LiteralObjects`, so to store a function declaration
	/// in memory, it needs to be converted first.
	///
	/// The reverse of this method is `from_literal`, which exists for all types that implement this method, `to_literal`.
	/// This is used, for example, to retrieve a function declaration as a type-safe instance of `FunctionDeclaration` from
	/// the compiler's virtual memory. Together, these two functions allow storing and retrieving arbitrary types in virtual
	/// memory.
	///
	/// This function is generally called at the very end of compile-time evaluation in a type's implementation of
	/// `evaluate_at_compile_time` from the `CompileTime` trait. This is when literals should be stored in virtual memory,
	/// and such types should return a pointer to that location in virtual memory from their compile-time evaluation method.
	///
	/// # Parameters
	///
	/// - `context` - Global data about the current state of the compiler. For this function in particular, implementors may
	/// find use out of the context by being able to access the program's scopes, which is how `Name`s are resolved, among
	/// other things.
	///
	/// # Returns
	///
	/// The literal object that this was converted to, or an error if there was an error while attempting to convert this
	/// to a literal object. This could be, for example, that a value that should be a literal isn't; Such as the case of a
	/// user using an expression as a type when that expression can't be fully evaluated at compile-time.
	fn to_literal(self) -> LiteralObject;

	/// Attempts to deserialize a literal object into `Self`.
	///
	/// For example, consider a `FunctionDeclaration`. Function declarations are their own struct with their own
	/// type information (return type, parameters, etc.), but they're serialized as literals with `to_literal` to be
	/// stored in virtual memory. Thus, when we want to retrieve information about the function declaration (such as
	/// when calling the function), we need to be able to deserialize the literal object back into a type-safe function
	/// declaration.
	///
	/// Note that this function takes a reference to a literal object, but returns an owned instance of `Self`. It may involve
	/// cloning. This is because literal objects are owned by virtual memory, and currently nothing can be moved out of virtual
	/// memory. Additionally, this function can't receive a mutable reference to a literal object, because any borrow of a literal
	/// object is indirectly a borrow of the compiler's `context`, and then `context` couldn't be passed to this function at all
	/// because only one mutable reference of it can exist at a time, which would be taken up by the borrow to the literal.
	///
	/// When this function is called depends on the specific type that's implementing it. For example, function declarations
	/// get deserialized during function calls, but group declarations get deserialized during object construction.
	///
	/// # Parameters
	///
	/// - `context` - Global data about the current state of the compiler. For this function in particular, implementors may
	/// find use out of the context by being able to access the program's scopes, which is how `Name`s are resolved, among
	/// other things.
	///
	/// # Returns
	///
	/// The instance of `Self` that the literal object was
	///
	/// # Errors
	///
	/// If the given literal isn't the right type.
	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self>;
}

impl Spanned for LiteralObject {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

#[derive(strum_macros::Display, Clone, Copy)]
pub enum CompilerWarning {
	SingleVariantEither,
	EmptyEither,
	RuntimeFunctionCall,
}
