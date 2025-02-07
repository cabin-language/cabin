use std::{
	collections::HashMap,
	fs::{File, OpenOptions},
	io::{Read, Stdout, Write},
};

mod lsp;

use lsp::State;
use regex_macro::regex;

fn main() -> anyhow::Result<()> {
	std::fs::write("/home/violet/Documents/Coding/Developer Tools/Cabin/cabin/crates/cabin-language-server/log.md", "")?;
	let mut logger = Logger {
		file: OpenOptions::new()
			.append(true)
			.open("/home/violet/Documents/Coding/Developer Tools/Cabin/cabin/crates/cabin-language-server/log.md")?,
	};
	logger.log("**Cabin Language Server started**")?;
	if let Err(error) = run(&mut logger) {
		logger.log(format!("\nERROR: {error}"))?;
	}
	logger.log("\nShutting down language server.")?;
	Ok(())
}

fn run(logger: &mut Logger) -> anyhow::Result<()> {
	logger.log("\n\n# Awaiting next request from client...\n")?;
	let mut buffer = String::new();
	let mut responder = std::io::stdout();
	let mut state = State { files: HashMap::new() };
	for byte in std::io::stdin().lock().bytes() {
		if buffer.is_empty() {
			logger.log("\n**Request received:**\n```json\n")?;
		}
		let byte = byte?;
		buffer.push(byte as char);
		logger.log((byte as char).to_string())?;
		handle_request(logger, &mut state, &mut responder, &mut buffer)?;
	}

	Ok(())
}

fn handle_request(logger: &mut Logger, state: &mut State, responder: &mut Stdout, buffer: &mut String) -> anyhow::Result<()> {
	// Matching
	let pattern = regex!("^Content-Length: (\\d+)\r\n\r\n(.+)");
	let captures = pattern.captures(buffer);
	let Some(captures) = captures else { return Ok(()); };

	// Content
	let content_length: usize = captures.get(1).unwrap().as_str().parse().unwrap();
	let content = captures.get(2).unwrap().as_str();
	if content.len() != content_length {
		return Ok(());
	}

	// Request
	logger.log(format!("\n```\n\n**Handling request:**\n```json\n{content}\n```\n"))?;
	let request: lsp::Request = serde_json::from_str(content)?;
	*buffer = String::new();

	// Response
	let response = request.response(state, logger)?;
	if let Some(response) = response {
		let response_string: String = response.try_into()?;
		logger.log(format!("\n**Response:**\n```json\n{response_string}\n```"))?;
		responder.write_all(response_string.as_bytes())?;
		responder.flush()?;
	} else {
		logger.log("\n**No response needed.**")?;
	}

	// Done
	logger.log("\n\n# Awaiting next request from client...\n")?;
	return Ok(());
}

struct Logger {
	file: File,
}

impl Logger {
	fn log<S: AsRef<str>>(&mut self, message: S) -> std::io::Result<()> {
		self.file.write(message.as_ref().replace("\r", "").as_bytes()).map(|_| {})
	}
}
