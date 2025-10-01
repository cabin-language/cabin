use proc_macro::TokenStream;
use quote::format_ident;
use syn::DeriveInput;

#[proc_macro_attribute]
pub fn wasm_fields(_attribute: TokenStream, input: TokenStream) -> TokenStream {
	let original_input: proc_macro2::TokenStream = input.clone().into();

	let ast = syn::parse_macro_input!(input as DeriveInput);
	let syn::Data::Struct(input_struct) = ast.data else {
		panic!("wasm_fields can only be applied to a struct");
	};
	let name = ast.ident;

	let methods = input_struct.fields.into_iter().map(|field| {
		let name = field.ident.unwrap();
		let field_type = field.ty;
		let setter_name = format_ident!("set_{name}");

		quote::quote! {
			#[::wasm_bindgen::prelude::wasm_bindgen(getter)]
			pub fn #name(&self) -> #field_type {
				self.#name.clone()
			}

			#[::wasm_bindgen::prelude::wasm_bindgen(setter)]
			pub fn #setter_name(&mut self, value: #field_type) {
				self.#name = value;
			}
		}
	});

	quote::quote! {
		#original_input

		#[::wasm_bindgen::prelude::wasm_bindgen]
		impl #name {
			#(#methods)*
		}
	}
	.into()
}
