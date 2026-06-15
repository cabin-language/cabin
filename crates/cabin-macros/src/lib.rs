use proc_macro::TokenStream;
use syn::{
	Expr,
	Ident,
	LitStr,
	Path,
	Token,
	parse::{Parse, ParseStream},
};

#[proc_macro]
pub fn import(input: TokenStream) -> TokenStream {
	let input = syn::parse_macro_input!(input as ImportInput);
	let mut quotes = Vec::new();

	let name = input.name;
	let context = input.context;
	let last_index = input.path.segments.len() - 2;

	let mut segments = input.path.segments.into_iter();
	let module = segments.next().unwrap();
	let mut last = None;

	for (index, other) in segments.enumerate() {
		let is_last = index == last_index;
		let field_name = LitStr::new(&other.ident.to_string(), other.ident.span());

		if is_last {
			last = Some(field_name.clone());
		}

		quotes.push(quote::quote! {
			.dot(&#context, #field_name)
			.unwrap()
		});
	}

	let last = name.unwrap_or_else(|| last.unwrap());

	quote::quote! {
		context.scope.declare_new_variable(
			crate::ast::expressions::identifier::Identifier::create_virtual(#last, &#context),
			#module #(#quotes)*
		).unwrap()
	}
	.into()
}

#[proc_macro]
pub fn try_import(input: TokenStream) -> TokenStream {
	let input = syn::parse_macro_input!(input as ImportInput);
	let mut quotes = Vec::new();

	let name = input.name;
	let context = input.context;
	let last_index = input.path.segments.len() - 2;

	let mut segments = input.path.segments.into_iter();
	let module = segments.next().unwrap();
	let mut last = None;

	for (index, other) in segments.enumerate() {
		let is_last = index == last_index;
		let field_name = LitStr::new(&other.ident.to_string(), other.ident.span());

		if is_last {
			last = Some(field_name.clone());
		}

		quotes.push(quote::quote! {
			.and_then(|obj| obj.dot(&#context, #field_name))
		});
	}

	let last = name.unwrap_or_else(|| last.unwrap());

	quote::quote! {
		Some(#module)#(#quotes)*.and_then(|obj| context.scope.declare_new_variable(
			crate::ast::expressions::identifier::Identifier::create_virtual(#last, &#context),
			obj
		).ok())
	}
	.into()
}

struct ImportInput {
	context: Expr,
	path: Path,
	name: Option<LitStr>,
}

impl Parse for ImportInput {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let expr = input.parse::<Expr>()?;
		let _ = input.parse::<Token![,]>()?;
		let path = input.parse::<Path>()?;
		let name = if input.peek(Token![as]) {
			let _ = input.parse::<Token![as]>()?;
			Some(if input.peek(Token![_]) {
				let span = input.parse::<Token![_]>()?.span;
				LitStr::new("_", span)
			} else {
				let ident = input.parse::<Ident>()?;
				LitStr::new(&ident.to_string(), ident.span())
			})
		} else {
			None
		};

		Ok(Self { context: expr, path, name })
	}
}
