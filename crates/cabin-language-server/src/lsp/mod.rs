use std::collections::HashMap;

use cabin::{lexer::TokenType, parser::expressions::Spanned as _};
use text_document::{
	diagnostics::{Diagnostic, PublishDiagnosticParams},
	did_change::TextDocumentDidChangeEvent,
	hover::HoverResult,
	Position,
	Range,
	TextDocumentIdentifier,
	TextDocumentItem,
	VersionTextDocumentIdentifier,
};

use crate::Logger;
mod text_document;

#[derive(serde::Deserialize)]
pub struct Request {
	pub id: Option<u32>,

	#[serde(flatten)]
	pub data: RequestData,
}

#[derive(serde::Deserialize)]
#[serde(tag = "method", content = "params")]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RequestData {
	Initialize {
		client_info: ClientInfo,
	},
	Initialized {},

	#[serde(rename = "textDocument/didOpen")]
	TextDocumentDidOpen {
		text_document: TextDocumentItem,
	},

	#[serde(rename = "textDocument/didChange")]
	TextDocumentDidChange {
		text_document: VersionTextDocumentIdentifier,
		content_changes: Vec<TextDocumentDidChangeEvent>,
	},

	#[serde(rename = "textDocument/hover")]
	TextDocumentDidHover {
		position: Position,
		text_document: TextDocumentIdentifier,
	},

	#[serde(rename = "textDocument/didSave")]
	DidSave {},
}

#[derive(serde::Deserialize)]
pub struct ClientInfo {
	name: String,
	version: String,
}

pub struct State {
	pub files: HashMap<String, String>,
}

impl Request {
	pub fn response(&self, state: &mut State, logger: &mut Logger) -> anyhow::Result<Option<Response>> {
		Ok(match &self.data {
			RequestData::Initialize { client_info } => {
				logger.log(format!("\n*Connected to client `{} {}`*\n", client_info.name, client_info.version))?;
				Some(Response {
					id: self.id,
					jsonrpc: "2.0",
					data: ResponseData::Initialize {
						result: InitializeResult {
							capabilities: ServerCapabilities {
								text_document_sync: 1,
								hover_provider: true,
							},
							server_info: ServerInfo {
								name: "cabin-language-server",
								version: "0.0.1",
							},
						},
					},
				})
			},
			RequestData::TextDocumentDidOpen { text_document } => {
				logger.log(format!("\n*Opened `{}`*\n", text_document.uri))?;
				state.files.insert(text_document.uri.clone(), text_document.text.clone());
				None
			},

			RequestData::TextDocumentDidChange { text_document, content_changes } => {
				logger.log(format!("\n*Changed `{}`*\n", text_document.uri))?;
				for change in content_changes {
					state.files.insert(text_document.uri.clone(), change.text.clone());
				}
				let code = state.files.get(&text_document.uri).unwrap();
				let errors = cabin::parse(code).err().map(|error| vec![error]).unwrap_or(Vec::new());
				let diagnostics = errors
					.into_iter()
					.map(|error| Diagnostic {
						severity: 1,
						message: format!("{error}"),
						source: "Cabin Language Server".to_owned(),
						range: Range {
							start: error.span().start_line_column(code).into(),
							end: error.span().end_line_column(code).into(),
						},
					})
					.collect::<Vec<_>>();

				if diagnostics.is_empty() {
					None
				} else {
					Some(Response {
						id: self.id,
						jsonrpc: "2.0",
						data: ResponseData::Diagnostics {
							method: "textDocument/publishDiagnostics".to_owned(),
							params: PublishDiagnosticParams {
								uri: text_document.uri.clone(),
								diagnostics,
							},
						},
					})
				}
			},

			RequestData::TextDocumentDidHover { position, text_document } => {
				let code = state.files.get(&text_document.uri).unwrap();
				let hover_text = cabin::tokenize(code)
					.map(|tokens| {
						let span = position.to_span(code);
						let token = tokens.iter().find(|token| token.span.contains(span.start()));
						token
							.map(|token| match token.token_type {
								TokenType::KeywordLet => unindent::unindent(
									r#"
									`let`

									---

									`let` is used to declare a new variable:

									```cabin
									let message = "Hello world!";
									```

									`let` declarations *must* provide a value, i.e., the following isn't valid:

									```cabin
									let message;
									```

									`let` declarations may optionally specify a type on the value, indicating that it
									can be reassigned:

									```cabin
									let message: Text = "Hello world!";
									message = "Goodbye world!";
									```
									"#,
								),
								TokenType::KeywordGroup => unindent::unindent(
									r#"
									`group`

									---

									`group` is used to declare a group type, similar to a `struct` in other languages:

									```cabin
									let Person = group {
										name: Text,
										age: Number
									};

									let john = new Person {
										name = "John",
										age = 30
									};
									```

									Groups are nominally typed, meaning even if two groups share the same structure,
									you cannot use them interchangeably, i.e., the following isn't valid:

									```cabin
									let Point = group { x: Number, y: Number };
									let Position = group { x: Number, y: Number };

									let x: Point = new Position { x = 10, y = 10 };
									```
									"#,
								),
								_ => format!("{}", token.token_type),
							})
							.unwrap_or(String::new())
					})
					.unwrap_or(String::new());
				Some(Response {
					id: self.id,
					jsonrpc: "2.0",
					data: ResponseData::Hover {
						result: HoverResult { contents: hover_text },
					},
				})
			},

			RequestData::Initialized {} => None,
			RequestData::DidSave {} => None,

			// Unimplemented
			_ => None,
		})
	}
}

#[derive(serde::Serialize)]
pub struct Response {
	id: Option<u32>,
	jsonrpc: &'static str,
	#[serde(flatten)]
	data: ResponseData,
}

impl TryInto<String> for Response {
	type Error = anyhow::Error;

	fn try_into(self) -> Result<String, Self::Error> {
		let json = serde_json::to_string(&self)?;
		Ok(format!("Content-Length: {}\r\n\r\n{json}", json.len()))
	}
}

#[derive(serde::Serialize)]
enum ResponseData {
	#[serde(untagged)]
	Initialize { result: InitializeResult },

	#[serde(untagged)]
	Hover { result: HoverResult },

	#[serde(untagged, rename = "textDocument/publishDiagnostics")]
	Diagnostics { method: String, params: PublishDiagnosticParams },
}

#[derive(serde::Serialize)]
struct InitializeResult {
	capabilities: ServerCapabilities,

	#[serde(rename = "serverInfo")]
	server_info: ServerInfo,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerCapabilities {
	text_document_sync: u8,
	hover_provider: bool,
}

#[derive(serde::Serialize)]
struct ServerInfo {
	name: &'static str,
	version: &'static str,
}
