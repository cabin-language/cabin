use std::collections::VecDeque;

use cabin::diagnostics::Diagnostic;
use colored::Colorize as _;
use tree_sitter::StreamingIterator;

use crate::theme::Theme;

/// Prints the code snippet for the given diagnostic, showing the location of the error. All
/// printing is done to `stderr`.
///
/// # Parameters
///
/// - `code` - The source code of the file that the diagnostic occurred in
/// - `diagnostic` - The diagnostic to show the snippet of
///
/// # Type Parameters
///
/// - `TTheme` - The theme to color the output with
pub(crate) fn show_snippet<TTheme: Theme>(diagnostic: &Diagnostic) {
	let code = std::fs::read_to_string(&diagnostic.file).unwrap();
	let (diagnostic_line, diagnostic_column) = diagnostic.span.start_line_column(&code).unwrap();
	let mut highlights = VecDeque::from(highlights::<TTheme>(&code));

	let (bg_r, bg_g, bg_b) = TTheme::background();
	let (fg_r, fg_g, fg_b) = TTheme::normal();
	let (comment_r, comment_g, comment_b) = TTheme::comment();

	eprintln!(
		"{}",
		format!(
			"    {} {}    ",
			"".truecolor(138, 84, 45),
			diagnostic.file.components().last().unwrap().as_os_str().to_str().unwrap()
		)
		.on_truecolor(bg_r, bg_g, bg_b)
	);
	eprint!(
		"{}\n{}",
		" ".repeat(80).on_truecolor(bg_r, bg_g, bg_b),
		" 1  ".truecolor(comment_r, comment_g, comment_b).on_truecolor(bg_r, bg_g, bg_b)
	);

	let mut byte_position = 0;
	let mut line: usize = 0;
	let mut column = 0;

	let characters = code.chars().collect::<Vec<_>>();

	while byte_position < code.len() {
		// Line out of range
		if line.abs_diff(diagnostic_line) > 2 {
			byte_position += 1;
			if characters[byte_position] == '\n' {
				line += 1;
				column = 0;
			} else {
				column += 1;
			}
			continue;
		}

		// Extra highlights
		while highlights.front().is_some_and(|highlight| highlight.start < byte_position) {
			highlights.pop_front().unwrap();
		}

		// Diagnostic pointer
		if line == diagnostic_line + 1 && column == 0 {
			eprint!(
				"{}",
				format!(
					"{}\n {}  ",
					format!(
						"{}{} here{}",
						" ".repeat(diagnostic_column + 4),
						"^".repeat(diagnostic.span.length),
						" ".repeat(80 - diagnostic.span.length - diagnostic_column - 9)
					)
					.truecolor(comment_r, comment_g, comment_b),
					(line + 1).to_string().truecolor(comment_r, comment_g, comment_b)
				)
				.on_truecolor(bg_r, bg_g, bg_b)
			);
		}

		// Newline
		if byte_position != code.len() - 2 && characters[byte_position] == '\n' {
			let (error_r, error_g, error_b) = TTheme::error();
			let mut ending = 0;
			if line == diagnostic_line {
				let (error_bg_r, error_bg_g, error_bg_b) = TTheme::error_background();
				let info = format!("{diagnostic}");
				let info = info.get(..info.find(':').unwrap()).unwrap();
				eprint!(
					"{}{}",
					" ".repeat(5).on_truecolor(bg_r, bg_g, bg_b),
					format!(" x {info} ",).on_truecolor(error_bg_r, error_bg_g, error_bg_b).truecolor(error_r, error_g, error_b),
				);
				ending = info.len() + 9;
			}

			eprint!("{}", " ".repeat(80 - column - 4 - ending).on_truecolor(bg_r, bg_g, bg_b));
			eprint!("{}", "\n".on_truecolor(bg_r, bg_g, bg_b));
			if line != diagnostic_line && byte_position != code.len() - 1 {
				if line == diagnostic_line - 1 {
					eprint!(
						"{}",
						format!(" {}  ", (line + 2).to_string().bold().truecolor(error_r, error_g, error_b)).on_truecolor(bg_r, bg_g, bg_b)
					);
				} else {
					eprint!(
						"{}",
						format!(" {}  ", (line + 2).to_string().truecolor(comment_r, comment_g, comment_b)).on_truecolor(bg_r, bg_g, bg_b)
					);
				}
			}
			line += 1;
			column = 0;
			byte_position += 1;
			continue;
		}

		// Error
		if line == diagnostic_line && column == diagnostic_column {
			let (error_r, error_g, error_b) = TTheme::error();
			eprint!(
				"\x1b[4:3m{}\x1b[0m",
				code.get(byte_position..byte_position + diagnostic.span.length)
					.unwrap()
					.on_truecolor(bg_r, bg_g, bg_b)
					.truecolor(error_r, error_g, error_b)
					.bold()
			);
			byte_position += diagnostic.span.length;
			column += diagnostic.span.length;
		}
		// Highlight
		else if highlights.front().is_some_and(|highlight| highlight.start == byte_position) {
			let highlight = highlights.pop_front().unwrap();
			let (r, g, b) = highlight.highlight;
			eprint!("{}", code.get(byte_position..highlight.end).unwrap().truecolor(r, g, b).on_truecolor(bg_r, bg_g, bg_b));
			byte_position += highlight.length();
			column += highlight.length();
		}
		// No highlight
		else {
			eprint!("{}", characters[byte_position].to_string().on_truecolor(bg_r, bg_g, bg_b).truecolor(fg_r, fg_g, fg_b));
			byte_position += 1;
			column += 1;
		}
	}

	eprintln!("{}", " ".repeat(80 - column).on_truecolor(bg_r, bg_g, bg_b));
	eprintln!();
}

/// Returns a list of `Highlights` for the given code, using the Tree-Sitter grammar for Cabin and
/// its highlight queries. The returned highlights are guaranteed to be in the order that they
/// appear in the given source code.
///
/// Note that the returned highlights may not exhaustively cover the entire source code; There may
/// be chunks that are left unhighlighted.
///
/// # Parameters
///
/// - `code` - The source code to highlight
///
/// # Type Parameters
///
/// - `TTheme` - The theme to highlight with
///
/// # Returns
///
/// The matched highlights in the source code.
fn highlights<TTheme: Theme>(code: &str) -> Vec<Highlight> {
	let mut parser = tree_sitter::Parser::new();
	let language = tree_sitter_cabin::LANGUAGE.into();
	parser.set_language(&language).unwrap();
	let tree = parser.parse(code, None).unwrap();

	let query = tree_sitter::Query::new(&language, tree_sitter_cabin::HIGHLIGHTS_QUERY).unwrap();
	let mut cursor = tree_sitter::QueryCursor::new();

	let mut highlights = Vec::new();
	cursor.matches(&query, tree.root_node(), code.as_bytes()).for_each(|query_match| {
		for capture in query_match.captures {
			let start = capture.node.start_byte();
			let end = capture.node.end_byte();
			let name = query.capture_names()[capture.index as usize];
			if let Some(highlight) = TTheme::highlight(name) {
				highlights.push(Highlight { start, end, highlight });
			}
		}
	});

	highlights
}

/// A highlight for a string of source code.
struct Highlight {
	/// The 0-based byte start index for the highlight, inclusive.
	start: usize,
	/// The 0-based byte end index for the highlight, non-inclusive.
	end: usize,
	/// The highlight color's red, green, and blue components.
	highlight: (u8, u8, u8),
}

impl Highlight {
	/// Returns the length of this highlight.
	fn length(&self) -> usize {
		self.end - self.start
	}
}
