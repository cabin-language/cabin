use std::collections::HashMap;

pub(crate) trait Theme {
	fn keyword() -> (u8, u8, u8);
	fn function_call() -> (u8, u8, u8);
	fn number() -> (u8, u8, u8);
	fn field() -> (u8, u8, u8);
	fn comment() -> (u8, u8, u8);
	fn normal() -> (u8, u8, u8);
	fn background() -> (u8, u8, u8);
	fn error() -> (u8, u8, u8);
	fn error_background() -> (u8, u8, u8);

	fn highlight<'a, 'b>(query: &'a str) -> Option<(u8, u8, u8)> {
		let highlight = HashMap::from([
			("function.call", Self::function_call()),
			("keyword", Self::keyword()),
			("number", Self::number()),
			("variable.member", Self::field()),
		]);
		highlight.get(query).cloned()
	}
}

pub(crate) struct CatppuccinMocha;

impl Theme for CatppuccinMocha {
	fn normal() -> (u8, u8, u8) {
		(205, 214, 244)
	}

	fn keyword() -> (u8, u8, u8) {
		(203, 166, 247)
	}

	fn comment() -> (u8, u8, u8) {
		(147, 153, 178)
	}

	fn function_call() -> (u8, u8, u8) {
		(137, 180, 250)
	}

	fn number() -> (u8, u8, u8) {
		(250, 179, 135)
	}

	fn field() -> (u8, u8, u8) {
		(180, 190, 254)
	}

	fn background() -> (u8, u8, u8) {
		(30, 30, 46)
	}

	fn error() -> (u8, u8, u8) {
		(243, 139, 168)
	}

	fn error_background() -> (u8, u8, u8) {
		(50, 40, 58)
	}
}
