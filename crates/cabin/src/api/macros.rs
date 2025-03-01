/// Returns the fully qualified path to the current function, similar to how `file!()` from `std` works, but for function names.
///
/// This is used by the compiler to log stack traces for printing developer information upon errors.
///
/// modified from <https://stackoverflow.com/a/40234666>
#[macro_export]
macro_rules! function {
	() => {{
		const fn f() {}
		fn type_name_of<T>(_: T) -> &'static str {
			std::any::type_name::<T>()
		}
		let name = type_name_of(f);
		let stripped = name.strip_suffix("::f").unwrap();
		let simplified = regex_macro::regex!("^<([^>< ]+) as ([^>< ]+)>(.*)$").replace(stripped, "${1}${3}").to_string();
		simplified.strip_suffix("::{{closure}}").unwrap_or(&simplified).to_owned()
	}};
}

/// Returns the second value provided wrapped in `Some()` if the first value is true; Otherwise, returns `None`.
///
/// This is equivalent to `boolean::then`, but doesn't create a closure, meaning return statements and the question mark operator
/// can be used in reference to the surrounding function.
#[macro_export]
macro_rules! if_then_some {
	(
		$value: expr, $body: expr
	) => {
		if $value {
			Some($body)
		} else {
			None
		}
	};
}

/// Returns the second value provided if the first provided value is `true`, otherwise, returns `Default::default()`.
#[macro_export]
macro_rules! if_then_else_default {
	(
		$value: expr, $body: expr
	) => {
		if $value {
			$body
		} else {
			Default::default()
		}
	};
}

/// Parses a comma-separated list of things. This takes a block of code as one of its parameters. The block is run once at the beginning,
/// and then while the next token is a comma, a comma is consumed and the block is run again. This is used for many comma-separated lists
/// in the language like function parameters, function arguments, group fields, group instantiation, etc.
///
/// This will return the last token that was parsed, so that expressions that end in a list can generate their spans.
#[macro_export]
macro_rules! parse_list {
	(
		$tokens: expr, $list_type: expr, $body: block
	) => {{
		use $crate::parser::TokenQueueFunctionality as _;

		let _ = $tokens.pop($list_type.opening())?;
		while !$tokens.next_is($list_type.closing()) {
			$body
			if $tokens.next_is($crate::lexer::TokenType::Comma) {
				let _ = $tokens.pop($crate::lexer::TokenType::Comma)?;
			} else {
				break;
			}
		}

		$tokens.pop($list_type.closing())?
	}};
}
