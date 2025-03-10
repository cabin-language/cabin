use std::{collections::VecDeque, io::Write, path::PathBuf};

use cabin::{diagnostics::Diagnostic, theme::Theme};
use colored::Colorize as _;
use tree_sitter::StreamingIterator;

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
pub(crate) fn show_snippet<CurrentTheme: cabin::theme::Theme>(diagnostic: &Diagnostic, max_columns: usize) {
	let code = if diagnostic.file == PathBuf::from("stdlib") {
		cabin::STDLIB.to_owned()
	} else {
		std::fs::read_to_string(&diagnostic.file).expect(&format!("No file {}", diagnostic.file.display()))
	};

	let mut highlights = VecDeque::from(highlights::<CurrentTheme>(&code));

	// Severity-specifics
	let ((error_r, error_g, error_b), (error_bg_r, error_bg_g, error_bg_b), icon) = match &diagnostic.info {
		cabin::diagnostics::DiagnosticInfo::Error(_) => (CurrentTheme::error(), CurrentTheme::error_background(), ""),
		cabin::diagnostics::DiagnosticInfo::Warning(_) => (CurrentTheme::warning(), CurrentTheme::warning_background(), ""),
		_ => unreachable!(),
	};

	let (bg_r, bg_g, bg_b) = CurrentTheme::background();
	let (fg_r, fg_g, fg_b) = CurrentTheme::normal();
	let (comment_r, comment_g, comment_b) = CurrentTheme::comment();

	eprintln!(
		"{}",
		format!(
			"    {} {}    ",
			"".truecolor(138, 84, 45),
			diagnostic.file.components().last().unwrap().as_os_str().to_str().unwrap()
		)
		.on_truecolor(bg_r, bg_g, bg_b)
	);

	let mut byte_position = 0;
	let mut line: usize = 0;
	let mut column = 0;

	let characters = code.chars().collect::<Vec<_>>();

	let (start_line, _start_column) = diagnostic.span.start_line_column(&code).unwrap();
	let (end_line, _end_column) = diagnostic.span.end_line_column(&code).unwrap();
	let mut leftmost_column = usize::MAX;
	let mut rightmost_column = 0;
	let middle_line = (start_line + end_line) / 2;

	// Line out of range
	while line.abs_diff(start_line) > 2 {
		if byte_position == characters.len() {
			break;
		}
		if characters[byte_position] == '\n' {
			line += 1;
			column = 0;
		} else {
			column += 1;
		}
		byte_position += 1;
	}

	eprint!(
		"{}\n{}",
		" ".repeat(max_columns).on_truecolor(bg_r, bg_g, bg_b),
		format!(" {}  ", line + 1).truecolor(comment_r, comment_g, comment_b).on_truecolor(bg_r, bg_g, bg_b)
	);
	std::io::stderr().flush().unwrap();

	let mut current_line_tabs = 0;

	while byte_position < code.len() {
		if (line as isize - end_line as isize) > 2 {
			break;
		}

		// Extra highlights
		while highlights.front().is_some_and(|highlight| highlight.start < byte_position) {
			highlights.pop_front().unwrap();
		}

		if characters[byte_position] == '\t' {
			current_line_tabs += 1;
		}

		// Diagnostic pointer
		if line == end_line + 1 && column == 0 {
			eprint!(
				"{}",
				format!(
					"{}\n {}  ",
					format!(
						"{}{} here{}",
						" ".repeat(leftmost_column + format!(" {line}  ").len()),
						"^".repeat(rightmost_column - leftmost_column + 1),
						" ".repeat(max_columns - (rightmost_column + 1) - " here".len() - format!(" {line}  ").len())
					)
					.truecolor(comment_r, comment_g, comment_b),
					(line + 1).to_string().truecolor(comment_r, comment_g, comment_b)
				)
				.on_truecolor(bg_r, bg_g, bg_b)
			);
		}

		// Newline
		if characters[byte_position] == '\n' && byte_position != code.len() - 2 {
			let mut ending = column + 3 * current_line_tabs + format!(" {}  ", line + 1).len();
			if line == middle_line {
				let info = format!("{diagnostic}");
				let info = info.get(..info.find(':').unwrap()).unwrap();
				eprint!(
					"{}{}{}{}",
					" ".repeat(5).on_truecolor(bg_r, bg_g, bg_b),
					"".truecolor(error_bg_r, error_bg_g, error_bg_b).on_truecolor(bg_r, bg_g, bg_b),
					format!("{icon} {info}",)
						.on_truecolor(error_bg_r, error_bg_g, error_bg_b)
						.truecolor(error_r, error_g, error_b),
					"".truecolor(error_bg_r, error_bg_g, error_bg_b).on_truecolor(bg_r, bg_g, bg_b),
				);
				ending += info.len() + 9; // +5 leading spaces, +2 padding characters, +1 , +1 space after 
			}

			eprint!("{}", " ".repeat(0.max(max_columns as isize - ending as isize) as usize).on_truecolor(bg_r, bg_g, bg_b));

			eprint!("{}", "\n".on_truecolor(bg_r, bg_g, bg_b));

			// Line numbers
			if byte_position != code.len() - 1 && line != end_line + 2 && line != end_line {
				if start_line > 0 && (start_line..=end_line).contains(&(line + 1)) {
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
			current_line_tabs = 0;
			continue;
		}

		// Error
		if diagnostic.span.contains(byte_position) {
			let undercurl = "\x1b[4:3m";
			let normal = "\x1b[0m";
			eprint!(
				"{undercurl}{}{normal}",
				characters[byte_position]
					.to_string()
					.replace("\t", "    ")
					.on_truecolor(bg_r, bg_g, bg_b)
					.truecolor(error_r, error_g, error_b)
					.bold()
			);

			leftmost_column = leftmost_column.min(column + current_line_tabs * 3);
			rightmost_column = rightmost_column.max(column + current_line_tabs * 3);

			byte_position += 1;
			column += 1;
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
			eprint!(
				"{}",
				characters[byte_position]
					.to_string()
					.replace('\t', "    ")
					.on_truecolor(bg_r, bg_g, bg_b)
					.truecolor(fg_r, fg_g, fg_b)
			);
			byte_position += 1;
			column += 1;
		}
	}

	eprintln!("{}", " ".repeat(max_columns).on_truecolor(bg_r, bg_g, bg_b));
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
