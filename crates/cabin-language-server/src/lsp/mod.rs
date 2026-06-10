use std::{collections::HashMap, ops::Not};

use cabin::{lexer::TokenType, typechecker::Typed as _};
use indoc::indoc;

use crate::{
	lsp::text_document::{
		diagnostics::{get_diagnostics, PublishDiagnosticParams},
		did_change::TextDocumentDidChangeEvent,
		hover::HoverResult,
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
				let norm_uri = normalize_uri(&text_document.uri)?;
				logger.log(format!("\n*Opened `{}`*\n", norm_uri))?;
				state.files.insert(norm_uri.clone(), text_document.text.clone());
				let diagnostics = get_diagnostics(state, logger, &norm_uri)?;
				diagnostics.is_empty().not().then_some(Response {
					id: None,
					jsonrpc: "2.0",
					data: ResponseData::Diagnostics {
						method: "textDocument/publishDiagnostics".to_owned(),
						params: PublishDiagnosticParams { uri: norm_uri, diagnostics },
					},
				})
			},

			RequestData::TextDocumentDidChange { text_document, content_changes } => {
				let norm_uri = normalize_uri(&text_document.uri)?;
				logger.log(format!("\n*Changed `{}`*\n", norm_uri))?;
				if let Some(last_change) = content_changes.last() {
					state.files.insert(norm_uri.clone(), last_change.text.clone());
				}
				let diagnostics = get_diagnostics(state, logger, &norm_uri)?;
				Some(Response {
					id: None,
					jsonrpc: "2.0",
					data: ResponseData::Diagnostics {
						method: "textDocument/publishDiagnostics".to_owned(),
						params: PublishDiagnosticParams { uri: norm_uri, diagnostics },
					},
				})
			},

			RequestData::TextDocumentDidHover { position, text_document } => {
				let code = state.files.get(&text_document.uri).unwrap_or_else(|| {
					logger.log(format!("\n**ERROR:** Could not find file for hover")).unwrap();
					unreachable!()
				});
				let tokens = cabin::tokenize(code).0;
				let span = position.to_span(code);
				let token = tokens.iter().find(|token| token.span.contains(span.start()));
				let hover_text = token
					.map(|token| match token.token_type {
						TokenType::Identifier => {
							let path = url::Url::parse(&text_document.uri)
								.unwrap_or_else(|error| {
									logger.log(format!("\n**ERROR:** Could not parse URL for hover: {error}")).unwrap();
									unreachable!()
								})
								.to_file_path()
								.unwrap_or_else(|_error| {
									logger.log(format!("\n**ERROR:** Could not convert URL to file path for hover")).unwrap();
									unreachable!()
								});
							if let Ok(mut project) = cabin::Project::from_child(path) {
								let name = project.name_at(span.start);
								name.map(|name| {
									let documentation = name
										.value(project.context_mut())
										.map(|expr| {
											expr.expression(project.context())
												.get_documentation()
												.map(|documentation| format!("\n---\n{documentation}"))
										})
										.flatten()
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
                            let
                            ===

                            `let` is used to declare a new variable:

                            ```cabin
                            let message = "Hello world!";
                            ```

                            `let` declarations *must* provide a value, i.e., the following isn't valid:

                            ```cabin
                            let message;
                            ```

                            # Mutability

                            `let` declarations may optionally specify a type on the value, indicating that 
							it can be reassigned:

                            ```cabin
                            let message: Text = "Hello world!";
                            message = "Goodbye world!";
                            ```

                            Without specifying a type, the variable cannot be reassigned:

                            ```cabin
                            let message = "Hello world!";
                            message = "Goodbye world!"; # not allowed!
                            ```

                            "#,
						)
						.to_owned(),
						TokenType::KeywordGroup => indoc!(
							r#"
                            group
                            =====

                            `group` is used to declare a group type, similar to a `struct` in other 
							languages:

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

                            # Nominality

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
                            # extend
                            ========

                            # Extensions

                            `extend` is used to "extend" one type "to be" another. `extend` is how
                            Cabin implements open polymorphism.

                            ```cabin
                            let Shape = group {
                                area: action(this: This): Number
                            };

                            let Square = group {
                                side_length: Number
                            };

                            let SquareArea = extend Square tobe Shape {
                                area = action(this: This): Number {
                                    return is this.side_length ^ 2;
                                }
                            };

                            let get_area = action(shape: Shape): Number {
                                return is shape.area();
                            };

                            get_area(new Square {});
                            ```

                            The `area` action is only available if it's implementation, `SquareArea`,
                            is in scope.

                            # Extending Non-Action Fields

                            Conveniently, non-action fields can be substituted for an action that
                            takes only an immutable `this` parameter:

                            ```cabin
                            let Shape = group {
                                area: Number
                            };

                            let Square = group {
                                side_length: Number
                            };

                            let SquareArea = extend Square tobe Shape {

                                # This is allowed!
                                area = action(this: This): Number {
                                    return is this.side_length ^ 2;
                                }
                            };

                            let area = new Square { side_length = 10 }.area;
                            ```

                            As with all actions in Cabin, it's automatically called without needing parentheses.

                            # Untyped Extensions

                            `extend` can also be used to extend a type without projecting it onto another:

                            ```cabin
                            let TextLength = extend Text {
                                get_length = action(this: This) {
                                    return is this.length;
                                }
                            };

                            let length = "Hello".get_length;
                            ```

                            Once again, these fields can only be accessed if their implementations are in scope.

                            # Default Extensions

                            Finally, extensions can be made "default", which means they are automatically brought
                            into scope whenever the target type is used:

                            ```cabin
                            let Vector = group {
                                x: Number,
                                y: Number
                            };

                            # This is automatically available when using `Vector`
                            default extend Vector tobe Addable {
                                plus = action(this: Vector, other: Vector): Vector {
                                    return is new Vector { x = this.x + other.x, y = this.y + other.y };
                                }
                            };
                            ```

                            Note that you can only declare a `default` extension for a type in the same scope
                            that the type is declared, i.e, you can't declare a `default` extension for `Text`,
                            because it's declared in a different scope.

                            Default extensions cannot be bound to a name.
                            "#,
						)
						.to_owned(),
						TokenType::KeywordNew => indoc!(
							r#"
							new
							===

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
				Some(Response {
					id: self.id,
					jsonrpc: "2.0",
					data: ResponseData::Hover {
						result: HoverResult { contents: hover_text.to_owned() },
					},
				})
			},

			RequestData::Initialized {} => None,
			RequestData::DidSave {} => None,
			RequestData::Shutdown {} => None,
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

use url::Url;

fn normalize_uri(uri_str: &str) -> anyhow::Result<String> {
	let url = Url::parse(uri_str).map_err(|e| anyhow::anyhow!("Failed to parse URI '{uri_str}': {e}"))?;
	let path = url.to_file_path().map_err(|_| anyhow::anyhow!("URI is not a valid file path: {uri_str}"))?;
	let normalized_url = Url::from_file_path(path).map_err(|_| anyhow::anyhow!("Failed to reconstruct URL from path"))?;

	Ok(normalized_url.to_string())
}
