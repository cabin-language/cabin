use cabin::{diagnostics::DiagnosticInfo, Spanned as _};
use gag::Gag;

use super::Range;
use crate::{lsp::State, Logger};

#[derive(serde::Serialize)]
pub struct PublishDiagnosticParams {
	pub uri: String,
	pub diagnostics: Vec<Diagnostic>,
}

#[derive(serde::Serialize)]
pub struct Diagnostic {
	pub range: Range,
	pub severity: u8,
	pub source: String,
	pub message: String,
}

fn diagnostic_code(diagnostic: &cabin::diagnostics::Diagnostic) -> u8 {
	match diagnostic.info() {
		DiagnosticInfo::Error(_) => 1,
		DiagnosticInfo::Warning(_) => 2,
		DiagnosticInfo::Info(_) => 3,
	}
}

pub fn get_diagnostics(state: &State, logger: &mut Logger, uri: &str) -> anyhow::Result<Vec<Diagnostic>> {
	logger.log("\n*Checking for diagnostics...*")?;

	let code = state.files.get(uri).unwrap_or_else(|| {
		logger.log(format!("\n**ERROR:** could not find file for diagnostics")).unwrap();
		unreachable!()
	});

	let path = url::Url::parse(uri)
		.unwrap_or_else(|error| {
			logger.log(format!("\n**ERROR:** could not parse URI for diagnostics: {error}")).unwrap();
			unreachable!()
		})
		.to_file_path()
		.unwrap_or_else(|_error| {
			logger.log(format!("\n**ERROR:** could not convert URI to file path")).unwrap();
			unreachable!()
		});
	logger.log(format!("\n*Checking project for* `{}`", path.display()))?;
	let mut project = match cabin::Project::from_child(path) {
		Ok(project) => project,
		Err(error) => anyhow::bail!(error),
	};

	let diagnostics = {
		let _stdout_gag = Gag::stdout()?;
		let _stderr_gag = Gag::stderr()?;
		project.check()
	};

	logger.log("\n*Done checking. Reporting diagnostics.*")?;

	Ok(diagnostics
		.to_owned()
		.into_iter()
		.map(|diagnostic| {
			let span = diagnostic.span;
			Diagnostic {
				severity: diagnostic_code(&diagnostic),
				message: format!("{diagnostic}"),
				source: "Cabin Language Server".to_owned(),
				range: Range {
					start: span.start_line_column(code).unwrap_or((0, 0)).into(),
					end: diagnostic.span.end_line_column(code).unwrap_or((0, 0)).into(),
				},
			}
		})
		.collect::<Vec<_>>())
}
