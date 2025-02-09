use std::ops::Deref;

use colored::Colorize as _;

/// A theme for the Cabin CLI tool. Themes are used by the compiler at various points during compilation to pretty-print code snippets from the source
/// code to point out error locations.
pub struct Theme {
	/// The style for keywords, such as `function`, `new`, and `group`.
	keyword: Style,
	/// The style for lowercase-starting variable names, such as `x`, `person`, and `myVARIABLE`.
	variable_name: Style,
	/// The style for uppercase-starting variable names, such as `Y`, `John`, and `MyGroup`.
	type_name: Style,
	/// The style for the background. This should really only be a color, but is a style to allow use with the `Styled` trait.
	background: Style,
	/// The style for the line numbers to the left of the program.
	line_numbers: Style,
	normal: Style,
	string: Style,
	number: Style,
	comment: Style,
	function: Style,
}

impl Default for Theme {
	fn default() -> Self {
		CATPPUCCIN_MOCHA
	}
}

impl Theme {
	/// Returns the keyword style for this theme. Themes should be immutable, so the `keyword` field is kept private, and this method is the only way to access
	/// it and obtain a reference to the keyword style.
	///
	/// # Returns
	/// An immutable reference to the style for keywords according to this theme.

	pub const fn keyword(&self) -> &Style {
		&self.keyword
	}

	/// Returns the variable name style for this theme. Themes should be immutable, so the `variable_name` field is kept private, and this method is the only way to access
	/// it and obtain a reference to the variable name style.
	///
	/// # Returns
	/// An immutable reference to the style for variable names according to this theme.

	pub const fn variable_name(&self) -> &Style {
		&self.variable_name
	}

	/// Returns the type name style for this theme. Themes should be immutable, so the `type_name` field is kept private, and this method is the only way to access
	/// it and obtain a reference to the type name style.
	///
	/// # Returns
	/// An immutable reference to the style for type names according to this theme.

	pub const fn type_name(&self) -> &Style {
		&self.type_name
	}

	/// Returns the background style for this theme. Themes should be immutable, so the `background` field is kept private, and this method is the only way to access
	/// it and obtain a reference to the background style.
	///
	/// # Returns
	/// An immutable reference to the style for the background according to this theme.

	pub const fn background(&self) -> &Style {
		&self.background
	}

	pub const fn normal(&self) -> &Style {
		&self.normal
	}

	pub const fn string(&self) -> &Style {
		&self.string
	}

	pub const fn comment(&self) -> &Style {
		&self.comment
	}

	pub const fn number(&self) -> &Style {
		&self.number
	}

	pub const fn function(&self) -> &Style {
		&self.function
	}

	/// Returns the line number style for this theme. Themes should be immutable, so the `line_number` field is kept private, and this method is the only way to access
	/// it and obtain a reference to the line number style.
	///
	/// # Returns
	/// An immutable reference to the style for line numbers according to this theme.

	pub const fn line_numbers(&self) -> &Style {
		&self.line_numbers
	}
}

/// Creates a hex `ColorLike` at compile-time. This takes a string literal as a parameter, and validates at compile-time that the string given
/// is a properly formatted hex string. Hex strings must be of the form `^[\da-fA-F]{6}$`, and shorthands (such as #FFF) cannot be used, nor
/// can an alpha value be specified.
macro_rules! hex {
	(
		$input: literal
	) => {{
		const HEX: &str = $crate::cli::theme::validate_hex_color($input);
		const COLOR_LIKE: $crate::cli::theme::ColorLike = $crate::cli::theme::ColorLike::Hex($crate::cli::theme::Hex::create_unsafely_from_raw_unvalidated_hex_string(HEX));
		COLOR_LIKE
	}};
}

/// The "one midnight" theme. This is Cabin's default theme; It's a midnight blurple with classic Atom One Dark foreground colors.
pub const ONE_MIDNIGHT: Theme = Theme {
	keyword: Style::foreground().color(hex!("#C678DD")),
	variable_name: Style::foreground().color(hex!("#e07c75")),
	type_name: Style::foreground().color(hex!("#e5c07b")),
	background: Style::background().color(hex!("#060115")),
	normal: Style::foreground().color(hex!("#BBBBBB")),
	string: Style::foreground().color(hex!("#98C379")),
	number: Style::foreground().color(hex!("#D19A66")),
	line_numbers: Style::foreground().color(Color::rgb(60, 50, 90)),
	comment: Style::foreground().color(Color::rgb(60, 50, 90)),
	function: Style::foreground().color(hex!("#61AFEF")),
};

pub const CATPPUCCIN_MOCHA: Theme = Theme {
	keyword: Style::foreground().color(hex!("#cba6f7")),
	variable_name: Style::foreground().color(hex!("#eba0ac")),
	type_name: Style::foreground().color(hex!("#f9e2af")),
	background: Style::background().color(hex!("#1e1e2e")),
	normal: Style::foreground().color(hex!("#cdd6f4")),
	string: Style::foreground().color(hex!("#a6e3a1")),
	number: Style::foreground().color(hex!("#fab387")),
	function: Style::foreground().color(hex!("#89b4fa")),
	line_numbers: Style::foreground().color(hex!("#7f849c")),
	comment: Style::foreground().color(hex!("#7f849c")),
};

/// The "empty" or "none" theme. This is used to print snippets in plain uncolored text.
pub const NO_THEME: Theme = Theme {
	keyword: Style::foreground(),
	variable_name: Style::foreground(),
	type_name: Style::foreground(),
	background: Style::background(),
	line_numbers: Style::foreground(),
	normal: Style::foreground(),
	string: Style::foreground(),
	number: Style::foreground(),
	comment: Style::foreground(),
	function: Style::foreground(),
};

pub enum ParseableTheme {
	/// The "one midnight" theme. This is Cabin's default theme; It's a midnight blurple with classic Atom One Dark foreground colors.
	OneMidnight,
	/// The "empty" or "none" theme. This is used to print snippets in plain uncolored text.
	None,
}

impl ParseableTheme {
	/// Returns a reference to the proper `Theme` object associated with this `ParseableTheme`. This shouldn't really need to be called explicitly often because
	/// `ParseableTheme` implements `Deref<Target = Theme>`, so consider using that instead when possible.

	pub const fn get_theme(&self) -> &Theme {
		match self {
			Self::OneMidnight => &ONE_MIDNIGHT,
			Self::None => &NO_THEME,
		}
	}
}

impl Deref for ParseableTheme {
	type Target = Theme;

	fn deref(&self) -> &Self::Target {
		self.get_theme()
	}
}

/// A 24-bit color. This is a color that can be built from various representations and converted between various representations. This is used by themes to create styles
/// using whatever color preference they have.
#[derive(Clone)]
pub struct Color {
	/// The red component of the color, as an unsigned 8-bit integer ranging from [0 - 255]
	red: u8,
	/// The green component of the color, as an unsigned 8-bit integer ranging from [0 - 255]
	green: u8,
	/// The blue component of the color, as an unsigned 8-bit integer ranging from [0 - 255]
	blue: u8,
}

/// A hex color, which contains a string of the hex representation. This should only ever be constructed using the `hex!()` macro, which validates at compile-time
/// that the given string is a valid hex string.
#[derive(Clone)]
pub struct Hex {
	/// The inner hex string representing the color.
	string: &'static str,
}

impl Hex {
	/// This is an internal function that should never be used, and is only public because it must be called from the `hex!` macro, hence the long name.
	/// To create a `Hex` color, use the `hex!` macro.

	pub const fn create_unsafely_from_raw_unvalidated_hex_string(im_not_telling_you_what_this_parameter_is_because_you_shouldnt_use_it: &'static str) -> Self {
		Self {
			string: im_not_telling_you_what_this_parameter_is_because_you_shouldnt_use_it,
		}
	}

	/// Returns a canonicalized `Color` object from this `Hex` color.

	pub fn to_color(&self) -> Color {
		let bigint = u32::from_str_radix(self.string.get(1..).unwrap(), 16).unwrap();
		let r = ((bigint >> 16) & 255) as u8;
		let g = ((bigint >> 8) & 255) as u8;
		let b = (bigint & 255) as u8;

		Color { red: r, green: g, blue: b }
	}
}

/// Validates that a string is a valid hex color at compile-time.
#[allow(clippy::indexing_slicing, reason = "Indexing is the only option in const fns.")]
const fn validate_hex_color(input: &str) -> &str {
	let mut index: usize = 0;
	let bytes = input.as_bytes();

	assert!(bytes.len() == 7, "Hex string must be 7 characters long");
	assert!(bytes[index] == b'#', "Hex literal must start with a #");

	index += 1;

	while index < 6 {
		match bytes[index] {
			b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {},
			_ => panic!("Invalid character encountered"),
		}
		index += 1;
	}

	input
}

impl Color {
	/// Creates a new color from red, green, and blue components. These are unsigned 8-bit integers, meaning they range from [0 - 255], so Rust automatically checks
	/// for us that the given values are within the range we want and that the color will be constructed properly.
	///
	/// # Parameters
	/// - `red` - The red component of the color, as an unsigned 8-bit integer from 0 to 255.
	/// - `green` - The green component of the color, as an unsigned 8-bit integer from 0 to 255.
	/// - `blue` - The blue component of the color, as an unsigned 8-bit integer from 0 to 255.
	///
	/// # Returns
	/// The created color wrapped in a `ColorLike`.

	pub const fn rgb(red: u8, green: u8, blue: u8) -> ColorLike {
		ColorLike::Color(Self { red, green, blue })
	}
}

/// An enum of objects that can be converted into colors. Themes can be made at compile-time and in `const` expressions, but we can't convert hex to RGB at compile-time
/// (because `u8::from_str_radix` isn't `const`) so instead we store a `ColorLike` inside `Style`, and then when we apply the style at runtime we convert the different
/// formats into a canonical RGB format.
#[derive(Clone)]
pub enum ColorLike {
	/// The standard color variant. This color holds RGB data, but can be built from HSL data or otherwise.
	Color(Color),
	/// The hex color variant. This holds hex data, and can be validated at compile-time with the `hex!` macro.
	Hex(Hex),
}

impl ColorLike {
	/// Converts this color to a canonicalized `Color` object.
	///
	/// # Returns
	/// A `Color` object representing the same color as this `ColorLike`.
	fn to_color(&self) -> Color {
		match self {
			Self::Color(color) => color.clone(),
			Self::Hex(hex) => hex.to_color(),
		}
	}
}

/// A style that can be constructed at compile-time and converted into a [`colored::Style`]. This is different from a [`colored::Style`] because it allows
/// being constructed at compile-time, so that we can create constant themes without `OnceLock`s or `lazy_static`s.
#[derive(Clone)]
pub struct Style {
	/// The color of the style. If this is a "background" style this is the background color, otherwise, this is the foreground color. This also can
	/// be `None` and defaults to `None`, in which case, the user's terminal's default themed foreground/background color will be used.
	color: Option<ColorLike>,
	/// Whether this style is bold
	bold: bool,
	/// Whether this style is underlined
	underline: bool,
	/// Whether this style is strikethrough
	strikethrough: bool,
	/// Whether this style is italic
	italic: bool,
	/// Whether this represents a "background" style. If true, setting the color of this style will set the background color, not the foreground. This
	/// should really only be true for the `background` color on themes.
	is_background: bool,
}

impl Style {
	/// Creates a new "foreground" style. This is a blank style, and applying a color to it applies a foreground color. This should be the default used for
	/// creating styles for most token types, outside of `background`.
	///
	/// # Returns
	/// The newly created foreground style

	pub const fn foreground() -> Self {
		Self {
			color: None,
			bold: false,
			italic: false,
			strikethrough: false,
			underline: false,
			is_background: false,
		}
	}

	/// Creates a new "background" style. This is a blank style, and applying a color to it applies a background color. This should only be used for creating a theme's
	/// background color.
	///
	/// # Returns
	/// The newly created background style

	pub const fn background() -> Self {
		Self {
			color: None,
			bold: false,
			italic: false,
			strikethrough: false,
			underline: false,
			is_background: true,
		}
	}

	/// Sets the color of this style. If this is a `foreground` style, this sets the foreground color, and if this is a `background` style, this sets the background color.
	/// This uses a builder-style pattern and consumes `self` and then returns `self` for convenient chaining, i.e.,
	///
	/// ```rust
	/// "my string".bold().italic().underline()
	/// ```

	pub const fn color(mut self, color: ColorLike) -> Self {
		self.color = Some(color);
		self
	}

	/// Sets this style to be bold. This uses a builder-style pattern and consumes `self` and then returns `self` for convenient chaining, i.e.,
	///
	/// ```rust
	/// "my string".bold().italic().underline()
	/// ```

	pub const fn bold(mut self) -> Self {
		self.bold = true;
		self
	}

	/// Sets this style to be underlined. This uses a builder-style pattern and consumes `self` and then returns `self` for convenient chaining, i.e.,
	///
	/// ```rust
	/// "my string".bold().italic().underline()
	/// ```

	pub const fn underline(mut self) -> Self {
		self.underline = true;
		self
	}

	/// Sets this style to be italic. This uses a builder-style pattern and consumes `self` and then returns `self` for convenient chaining, i.e.,
	///
	/// ```rust
	/// "my string".bold().italic().underline()
	/// ```

	pub const fn italic(mut self) -> Self {
		self.italic = true;
		self
	}

	/// Sets this style to be strikethrough. This uses a builder-style pattern and consumes `self` and then returns `self` for convenient chaining, i.e.,
	///
	/// ```rust
	/// "my string".bold().italic().underline()
	/// ```

	pub const fn strikethrough(mut self) -> Self {
		self.strikethrough = true;
		self
	}

	/// Applies this style to the given string. This usually doesn't need to be used directly, and generally you can just use `Styled::style` on a string directly;
	/// This is what `Styled::style` uses internally to convert a string to a [`colored::ColoredString`]
	///
	/// # Parameters
	/// - `string` - The string to style and convert

	fn on(&self, string: &str) -> colored::ColoredString {
		let mut base = colored::ColoredString::from(string);
		if self.bold {
			base = base.bold();
		}

		if self.italic {
			base = base.italic();
		}

		if self.strikethrough {
			base = base.strikethrough();
		}

		if self.underline {
			base = base.underline();
		}

		if let Some(color_like) = &self.color {
			let color = color_like.to_color();
			base = if self.is_background {
				base.on_truecolor(color.red, color.green, color.blue)
			} else {
				base.truecolor(color.red, color.green, color.blue)
			};
		}

		base
	}
}

/// A trait for strings providing functionality that converts them into a [`colored::ColoredString`] using a theme's `style`. This is used to style tokens in error messages
/// with the themes colors and styles.
pub trait Styled {
	/// Styles this string into a [`colored::ColoredString`] using the given `style` object from a theme.
	///
	/// # Parameters
	/// - `style` - The style to use on this string, likely from a method from a `Theme` object.
	fn style(&self, style: &Style) -> colored::ColoredString;
}

impl Styled for String {
	fn style(&self, style: &Style) -> colored::ColoredString {
		style.on(self)
	}
}

impl Styled for str {
	fn style(&self, style: &Style) -> colored::ColoredString {
		style.on(self)
	}
}
