use std::{collections::HashMap, ops::Not};

use cabin::lexer::TokenType;
use indoc::indoc;

use crate::{
	lsp::text_document::{
		diagnostics::{get_diagnostics, Diagnostic, PublishDiagnosticParams},
		did_change::TextDocumentDidChangeEvent,
		hover::HoverResult,
		Position,
		Range,
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
				let diagnostics = get_diagnostics(state, logger, &text_document.uri)?;
				diagnostics.is_empty().not().then_some(Response {
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
			},

			RequestData::TextDocumentDidChange { text_document, content_changes } => {
				logger.log(format!("\n*Changed `{}`*\n", text_document.uri))?;
				for change in content_changes {
					state.files.insert(text_document.uri.clone(), change.text.clone());
				}
				let diagnostics = get_diagnostics(state, logger, &text_document.uri)?;
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
			},

			RequestData::TextDocumentDidHover { position, text_document } => {
				let code = state.files.get(&text_document.uri).unwrap();
				let tokens = cabin::tokenize(code).0;
				let span = position.to_span(code);
				let token = tokens.iter().find(|token| token.span.contains(span.start()));
				let hover_text = token
					.map(|token| match token.token_type {
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

                            `let` declarations may optionally specify a type on the value, indicating that it
                            can be reassigned:

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
						),
						TokenType::KeywordGroup => indoc!(
							r#"
                            group
                            =====

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

                            # Nominality

                            Groups are nominally typed, meaning even if two groups share the same structure,
                            you cannot use them interchangeably, i.e., the following isn't valid:

                            ```cabin
                            let Point = group { x: Number, y: Number };
                            let Position = group { x: Number, y: Number };

                            let x: Point = new Position { x = 10, y = 10 };
                            ```
                            "#,
						),
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
						),
						_ => "",
					})
					.unwrap_or("");
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
