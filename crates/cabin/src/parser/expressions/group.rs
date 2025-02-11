use std::collections::{HashMap, VecDeque};

use convert_case::{Case, Casing as _};

use crate::{
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
	},
	bail_err,
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo, Warning},
	if_then_else_default,
	if_then_some,
	lexer::{Span, Token, TokenType},
	parse_list,
	parser::{
		expressions::{
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, InternalFieldValue},
			parameter::Parameter,
			Expression,
			Spanned,
			TryParse,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		ParseError,
		TokenQueueFunctionality,
	},
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

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordGroup)?.span;
		let outer_scope_id = context.scope_data.unique_id();
		context.scope_data.enter_new_scope(ScopeType::Group);
		let inner_scope_id = context.scope_data.unique_id();

		let _compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				let name = parameter.virtual_deref(context).name().to_owned();
				if let Err(error) = context.scope_data.declare_new_variable(name.clone(), Expression::Pointer(parameter)) {
					context.add_diagnostic(Diagnostic {
						span: name.span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		// Fields
		let mut fields: Vec<Field> = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			//  Group field tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			// Group field name
			let name = Name::try_parse(tokens, context)?;
			if !name.unmangled_name().is_case(Case::Snake) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					info: DiagnosticInfo::Warning(Warning::NonSnakeCaseName {
						original_name: name.unmangled_name().to_owned(),
					}),
				});
			}

			if fields.iter().any(|field| field.name == name) {
				context.add_diagnostic(Diagnostic {
					span: name.span(context),
					info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::DuplicateField(name.unmangled_name().to_owned()))),
				});
			}

			// Group field type
			let field_type = if_then_some!(tokens.next_is(TokenType::Colon), {
				let _ = tokens.pop(TokenType::Colon)?;
				Expression::parse(tokens, context)
			});

			// Group field value
			let value = if_then_some!(tokens.next_is(TokenType::Equal), {
				let _ = tokens.pop(TokenType::Equal)?;
				let mut value = Expression::parse(tokens, context);

				// Set tags
				if let Some(tags) = tags {
					value.set_tags(tags, context);
				}

				value.try_set_name(format!("anonymous_group_{}", name.unmangled_name()).into(), context);

				value
			});

			// Add field
			fields.push(Field { name, value, field_type });
		})
		.span;
		context.scope_data.exit_scope().unwrap();

		Ok(GroupDeclaration {
			fields,
			inner_scope_id,
			outer_scope_id,
			name: "anonymous_group".into(),
			span: start.to(end),
		}
		.to_literal()
		.store_in_memory(context))
	}
}

impl CompileTime for GroupDeclaration {
	type Output = GroupDeclaration;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let scope_reverter = context.scope_data.set_current_scope(self.inner_scope_id);
		let mut fields = Vec::new();

		for field in self.fields {
			// Field value
			let value = if let Some(value) = field.value {
				let span = value.span(context);
				let evaluated = value.evaluate_at_compile_time(context);

				if !evaluated.is_pointer() && !evaluated.is_error() {
					context.add_diagnostic(Diagnostic {
						span,
						info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::GroupValueNotKnownAtCompileTime)),
					});
				}

				Some(evaluated)
			} else {
				None
			};

			// Field type
			let field_type = if let Some(field_type) = field.field_type {
				Some(field_type.evaluate_at_compile_time(context))
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
		scope_reverter.revert(context);
		GroupDeclaration {
			fields,
			inner_scope_id: self.inner_scope_id,
			outer_scope_id: self.outer_scope_id,
			name: self.name,
			span: self.span,
		}
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
			fields: literal.get_internal_field::<Vec<Field>>("fields").cloned().unwrap_or(Vec::new()),
			outer_scope_id: literal.outer_scope_id(),
			inner_scope_id: literal.inner_scope_id.unwrap_or(ScopeId::global()),
			name: literal.name.clone(),
			span: literal.span,
		})
	}
}

impl Spanned for GroupDeclaration {
	fn span(&self, _context: &Context) -> Span {
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
