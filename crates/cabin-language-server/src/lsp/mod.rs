use std::collections::HashMap;

use cabin::{lexer::TokenType, typechecker::Typed as _};
use indoc::indoc;
use url::Url;

use crate::{
	lsp::text_document::{
		diagnostics::{get_diagnostics, PublishDiagnosticParams},
		did_change::TextDocumentDidChangeEvent,
		hover::{HoverResult, MarkupContents},
		Position,
		TextDocumentIdentifier,
		TextDocumentItem,
		VersionTextDocumentIdentifier,
	},
	Logger,
};
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

	#[serde(rename = "custom/setMode")]
	CustomSetMode {
		mode: String,
	},

	#[serde(rename = "textDocument/didSave")]
	DidSave {},

	#[serde(rename = "shutdown")]
	Shutdown,
}

#[derive(serde::Deserialize)]
pub struct ClientInfo {
	name: String,
	version: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum Mode {
	#[default]
	Dev,
	Prod,
}

pub struct State {
	pub files: HashMap<String, String>,
	pub mode: Mode,
}

impl Request {
	pub fn response(&self, state: &mut State, logger: &mut Logger) -> anyhow::Result<Vec<Response>> {
		Ok(match &self.data {
			RequestData::CustomSetMode { mode } => {
				logger.log(format!("\n*Mode changed to `{}`*\n", mode))?;

				state.mode = match mode.as_str() {
					"prod" | "production" => Mode::Prod,
					_ => Mode::Dev,
				};

				let mut diagnostic_responses = Vec::new();

				let active_uris: Vec<String> = state.files.keys().cloned().collect();

				for uri in active_uris {
					let diagnostics = get_diagnostics(state, logger, &uri)?;

					diagnostic_responses.push(Response {
						id: None,
						jsonrpc: "2.0",
						data: ResponseData::Diagnostics {
							method: "textDocument/publishDiagnostics".to_owned(),
							params: PublishDiagnosticParams { uri, diagnostics },
						},
					});
				}

				diagnostic_responses
			},
			RequestData::Initialize { client_info } => {
				logger.log(format!("\n*Connected to client `{} {}`*\n", client_info.name, client_info.version))?;
				vec![Response {
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
				}]
			},
			RequestData::TextDocumentDidOpen { text_document } => {
				let norm_uri = normalize_uri(&text_document.uri)?;
				logger.log(format!("\n*Opened `{}`*\n", norm_uri))?;
				state.files.insert(norm_uri.clone(), text_document.text.clone());
				let diagnostics = get_diagnostics(state, logger, &norm_uri)?;
				if !diagnostics.is_empty() {
					vec![Response {
						id: None,
						jsonrpc: "2.0",
						data: ResponseData::Diagnostics {
							method: "textDocument/publishDiagnostics".to_owned(),
							params: PublishDiagnosticParams { uri: norm_uri, diagnostics },
						},
					}]
				} else {
					Vec::new()
				}
			},

			RequestData::TextDocumentDidChange { text_document, content_changes } => {
				let norm_uri = normalize_uri(&text_document.uri)?;
				logger.log(format!("\n*Changed `{}`*\n", norm_uri))?;
				if let Some(last_change) = content_changes.last() {
					state.files.insert(norm_uri.clone(), last_change.text.clone());
				}
				let diagnostics = get_diagnostics(state, logger, &norm_uri)?;
				vec![Response {
					id: None,
					jsonrpc: "2.0",
					data: ResponseData::Diagnostics {
						method: "textDocument/publishDiagnostics".to_owned(),
						params: PublishDiagnosticParams { uri: norm_uri, diagnostics },
					},
				}]
			},

			RequestData::TextDocumentDidHover { position, text_document } => {
				let norm_uri = normalize_uri(&text_document.uri)?;
				let code = state.files.get(&norm_uri).unwrap_or_else(|| {
					logger.log("\n**ERROR:** Could not find file for hover").unwrap();
					unreachable!()
				});
				let tokens = cabin::tokenize(code).0;
				let span = position.to_span(code);
				let token = tokens.iter().find(|token| token.span.contains(span.start()));
				let hover_text = token
					.map(|token| match token.token_type {
						TokenType::Identifier => {
							let path = url::Url::parse(&norm_uri)
								.unwrap_or_else(|error| {
									logger.log(format!("\n**ERROR:** Could not parse URL for hover: {error}")).unwrap();
									unreachable!()
								})
								.to_file_path()
								.unwrap_or_else(|_error| {
									logger.log("\n**ERROR:** Could not convert URL to file path for hover").unwrap();
									unreachable!()
								});
							if let Ok(mut project) = cabin::Project::from_child(path) {
								let name = project.name_at(span.start);
								name.map(|name| {
									let documentation = name
										.value(project.context_mut())
										.and_then(|expr| {
											expr.expression(project.context())
												.get_documentation()
												.map(|documentation| format!("\n---\n{documentation}"))
										})
										.map(|str| str.to_owned())
										.unwrap_or(String::new());
									format!(
										"```cabin\n{}: {}\n```{}",
										name.source_identifier(),
										name.get_type(project.context_mut()).name(project.context_mut()),
										documentation
									)
								})
								.unwrap_or("unknown".to_owned())
								.to_owned()
							} else {
								"unknown".to_owned()
							}
						},
						TokenType::KeywordLet => indoc!(
							r#"
							```cabin
							let
							```
							
							---

							`let` is used to declare a new variable:

							```cabin
							let message = "Hello world!";
							```

							`let` declarations *must* provide a value, i.e., the following isn't valid:

							```cabin
							let message;
							```

							## Mutability

							`let` declarations may optionally be marked `editable`, indicating that 
							it can be reassigned:

							```cabin
							let editable person: Person = john;
							person = steve;
							```

							If the type itlsef is declared `editable`, then it can be mutated:

							```cabin
							let message: editable Person = john;
							person.name = "johnathan";
							```

							Of course, both is an option:
							
							```cabin
							let editable message: editable Person = john;
							person.name = "johnathan";
							```
							"#,
						)
						.to_owned(),
						TokenType::KeywordGroup => indoc!(
							r#"
							```cabin
							group
							```

							---

							`group` is used to declare a group type, similar to a `struct` in other 
							languages:

							```cabin
							let Person = group {
								name: Text,
								age: Number
							};
							```

							## Nominality

							Groups are nominally typed, meaning even if two groups share the same 
							structure, you cannot use them interchangeably, i.e., the following isn't 
							valid:

							```cabin
							let Point = group { x: Number, y: Number };
							let Position = group { x: Number, y: Number };

							let x: Point = new Position { x = 10, y = 10 };
							```
							"#,
						)
						.to_owned(),
						TokenType::KeywordExtend => indoc!(
							r#"
							```cabin
							extend
							```

							`extend` is used to add properties to a type:

							```cabin
							let text_extension = extend Text {
								is_even = action(this: This) {
									return is this.length mod 2 == 0;
								}
							};

							let even = "hello world!".is_even();
							```

							Properties declared in extensions can only be accessed if the extension
							is in scope:

							```cabin
							let _ = other_file.text_extension;
							```

							The one exception to this is if the extension is marked with `#[default]`,
							in which case, it's automatically in scope whenever the type is used:

							```cabin
							#[default]
							let text_extension = extend Text {
								is_even = action(this: This) {
									return is this.length mod 2 == 0;
								}
							};
							```

							`#[default]` can only be used when defining an extension in the same file as
							the type being extended.

							Extensions also allow extending a type to be assignable to another type. For
							example:

							```cabin
							let Shape = group {
								area: action(this: This): Number
							};

							let Rectangle = group {
								length: Number,
								width: Number
							};

							let RectangleShape = extend Rectangle as Shape {
								area = action(this: This): Number {
									return is this.length * this.width;
								}
							}:

							let area_squared = action(shape: Shape): Number {
								return is shape.area() ^ 2;
							};

							let rect = new Rectangle {
								length: 5,
								width: 10
							};

							let a2 = area_squared(rect);
							```

							As you can see, we can assign an instance of `Rectangle` to a parameter of
							type `Shape`, as long as we have our extension `RectangleShape` in scope.
							"#,
						)
						.to_owned(),
						TokenType::KeywordNew => indoc!(
							r#"
							```cabin
							new

							---

							`new` is used to create a new instance of a `group`:

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
							"#
						)
						.to_owned(),
						TokenType::KeywordAction => indoc!(
							r#"
							action
							======

							`action` takes a list of statements and runs them when it's called:

							```cabin
							let print_hello = action {
								print("Hello!");
							};

							print_hello(); # Prints "Hello!"
							```

							# Returning

							`action` can give a value back to the caller by breaking from the
							`return` label:

							```cabin
							let get_name = action: Text {
								return is "john";
							};

							let name = get_name();
							```

							# Parameters

							`actions` can take parameters:

							```cabin
							let capitalize = action(text: Text): Text {
								return is text.uppercase();
							};
							```

							# Compile-Time Parameters

							`actions` may also specify parameters in angle brackets to indicate
							that their values must be known at compile-time:

							```cabin
							let compile_time_capitalize = action<text: Text>: Text {
								return is text.uppercase();
							};

							let uppercase = compile_time_capitalize<"john">;
							```

							Compile-Time parameters don't need to specify a type, they will default
							to `Anything`:

							```cabin
							let do_something = action<T> { # equivalent to <T: Anything>
								# ...
							};
							```

							Furthermore, compile-time parameters may be used as types of runtime
							parameters:

							```cabin
							let do_something = action<T>(value: T) {
								# ...
							};

							do_something<Text>("Hello");
							```
							"#
						)
						.to_owned(),
						_ => String::new(),
					})
					.unwrap_or(String::new());
				vec![Response {
					id: self.id,
					jsonrpc: "2.0",
					data: ResponseData::Hover {
						result: HoverResult {
							contents: MarkupContents {
								kind: "markdown",
								value: hover_text.to_owned(),
							},
						},
					},
				}]
			},

			RequestData::Initialized {} => Vec::new(),
			RequestData::DidSave {} => Vec::new(),
			RequestData::Shutdown => Vec::new(),
		})
	}
}

#[derive(serde::Serialize)]
pub struct Response {
	#[serde(skip_serializing_if = "Option::is_none")]
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

fn normalize_uri(uri_str: &str) -> anyhow::Result<String> {
	let url = Url::parse(uri_str).map_err(|e| anyhow::anyhow!("Failed to parse URI '{uri_str}': {e}"))?;
	let path = url.to_file_path().map_err(|_| anyhow::anyhow!("URI is not a valid file path: {uri_str}"))?;
	let normalized_url = Url::from_file_path(path).map_err(|_| anyhow::anyhow!("Failed to reconstruct URL from path"))?;

	Ok(normalized_url.to_string())
}
