use std::{
	collections::{HashMap, VecDeque},
	fmt::Write as _,
};

use super::field_access::FieldAccessType;
use crate::{
	api::{
		context::context,
		scope::{ScopeId, ScopeType},
	},
	bail_err,
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	if_then_else_default,
	if_then_some,
	lexer::{Span, Token, TokenType},
	parse_list,
	parser::{
		expressions::{
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, InternalFieldValue},
			parameter::Parameter,
			Expression,
			Spanned,
			TryParse,
			Typed,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueueFunctionality,
	},
	transpiler::TranspileToC,
	Diagnostic,
	DiagnosticInfo,
};

#[derive(Debug, Clone)]
pub struct GroupDeclaration {
	fields: Vec<Field>,
	inner_scope_id: ScopeId,
	outer_scope_id: ScopeId,
	name: Name,
	span: Span,
}

impl TryParse for GroupDeclaration {
	type Output = VirtualPointer;

	fn try_parse(tokens: &mut VecDeque<Token>) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordGroup)?.span;
		let outer_scope_id = context().scope_data.unique_id();
		context().scope_data.enter_new_scope(ScopeType::Group);
		let inner_scope_id = context().scope_data.unique_id();

		let _compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens)?;
				context()
					.scope_data
					.declare_new_variable(parameter.virtual_deref().name(), Expression::Pointer(parameter))?;
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		// Fields
		let mut fields = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			//  Group field tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens)?);

			// Group field name
			let name = Name::try_parse(tokens)?;

			// Group field type
			let field_type = if_then_some!(tokens.next_is(TokenType::Colon), {
				let _ = tokens.pop(TokenType::Colon)?;
				Expression::parse(tokens)
			});

			// Group field value
			let value = if_then_some!(tokens.next_is(TokenType::Equal), {
				let _ = tokens.pop(TokenType::Equal)?;
				let mut value = Expression::parse(tokens);

				// Set tags
				if let Some(tags) = tags {
					value.set_tags(tags);
				}

				value.try_set_name(format!("anonymous_group_{}", name.unmangled_name()).into());

				value
			});

			// Add field
			fields.push(Field { name, value, field_type });
		})
		.span;
		context().scope_data.exit_scope().unwrap();

		Ok(GroupDeclaration {
			fields,
			inner_scope_id,
			outer_scope_id,
			name: "anonymous_group".into(),
			span: start.to(end),
		}
		.to_literal()
		.store_in_memory())
	}
}

impl CompileTime for GroupDeclaration {
	type Output = GroupDeclaration;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);
		let mut fields = Vec::new();

		for field in self.fields {
			// Field value
			let value = if let Some(value) = field.value {
				let evaluated = value.evaluate_at_compile_time();

				if !evaluated.is_pointer() {
					context().add_diagnostic(Diagnostic {
						span: evaluated.span(),
						error: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::GroupValueNotKnownAtCompileTime)),
					});
				}

				Some(evaluated)
			} else {
				None
			};

			// Field type
			let field_type = if let Some(field_type) = field.field_type {
				Some(field_type.evaluate_at_compile_time())
			} else {
				None
			};

			// Add the field
			fields.push(Field {
				name: field.name,
				value,
				field_type,
			});
		}

		// Store in memory and return a pointer
		GroupDeclaration {
			fields,
			inner_scope_id: self.inner_scope_id,
			outer_scope_id: self.outer_scope_id,
			name: self.name,
			span: self.span,
		}
	}
}

impl TranspileToC for GroupDeclaration {
	fn to_c(&self) -> anyhow::Result<String> {
		let mut builder = "{".to_owned();

		for field in &self.fields {
			write!(
				builder,
				"\n\t{}* {};",
				if let Some(field_type) = &field.field_type {
					field_type.try_as_literal()?.to_c_type()?
				} else {
					field
						.value
						.as_ref()
						.unwrap_or(&Expression::ErrorExpression(Span::unknown()))
						.get_type()?
						.virtual_deref()
						.to_c_type()?
				},
				field.name.to_c()?
			)
			.unwrap();
		}

		match self.name.unmangled_name() {
			"Text" => builder += "\n\tchar* internal_value;",
			"Number" => builder += "\n\tfloat internal_value;",
			"Function" => builder += "\n\tvoid* call;",
			"List" => builder += "\n\tvoid* elements;\n\tint size;\n\tint capacity;",
			_ => {},
		}

		builder += "\n}";
		Ok(builder)
	}
}

impl LiteralConvertible for GroupDeclaration {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([("fields".to_owned(), InternalFieldValue::FieldList(self.fields))]),
			name: self.name,
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: Some(self.inner_scope_id),
			span: self.span,
			type_name: "Group".into(),
			tags: TagList::default(),
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		if literal.field_access_type != FieldAccessType::Normal {
			bail_err! {
				base = "attempted to convert a non-group literal into a group",
			};
		}

		Ok(GroupDeclaration {
			fields: literal.get_internal_field::<Vec<Field>>("fields")?.to_owned(),
			outer_scope_id: literal.outer_scope_id(),
			inner_scope_id: literal.inner_scope_id.unwrap(),
			name: literal.name.clone(),
			span: literal.span,
		})
	}
}

impl Spanned for GroupDeclaration {
	fn span(&self) -> Span {
		self.span
	}
}

impl GroupDeclaration {
	pub fn fields(&self) -> &[Field] {
		&self.fields
	}

	pub fn set_name(&mut self, name: Name) {
		self.name = name.clone();
		//self.fields.iter_mut().for_each(|field| {
		//	field.name = format!("{}_{}", name.unmangled_name(), field.name.unmangled_name()).into();
		//	if let Some(value) = &mut field.value {
		//		value.try_set_name(field.name.clone());
		//	}
		//});
	}
}
