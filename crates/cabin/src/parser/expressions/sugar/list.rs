use crate::{
	api::context::context,
	parse_list,
	parser::{
		expressions::{
			field_access::FieldAccessType,
			name::Name,
			object::{InternalFieldValue, ObjectConstructor},
			Expression,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
};

pub struct List;

impl TryParse for List {
	type Output = Expression;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let mut list = Vec::new();
		let start = tokens.current_position().unwrap();
		let end = parse_list!(tokens, ListType::Bracketed, { list.push(Expression::parse(tokens)) }).span;

		let constructor = ObjectConstructor {
			type_name: Name::from("List"),
			fields: Vec::new(),
			internal_fields: std::collections::HashMap::from([("elements".to_owned(), InternalFieldValue::ExpressionList(list))]),
			outer_scope_id: context().scope_data.unique_id(),
			inner_scope_id: context().scope_data.unique_id(),
			field_access_type: FieldAccessType::Normal,
			name: "anonymous_runtime_list".into(),
			span: start.to(end),
			tags: TagList::default(),
		};

		Ok(Expression::ObjectConstructor(constructor))
	}
}
