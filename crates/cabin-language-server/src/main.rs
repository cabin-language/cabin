use std::{
	collections::HashMap,
	fs::{File, OpenOptions},
	io::{BufRead, BufReader, Read, Stdout, Write},
};

mod lsp;
use lsp::State;

fn main() -> anyhow::Result<()> {
	let logfile = directories::BaseDirs::new().unwrap().data_local_dir().join("cabin-language-server-log.md");
	std::fs::write(&logfile, "")?;

	let mut logger = Logger {
		file: OpenOptions::new().append(true).open(logfile)?,
	};

	logger.log("**Cabin Language Server started**\n")?;
	if let Err(error) = run(&mut logger) {
		logger.log(format!("\nERROR: {error}\n"))?;
	}
	logger.log("\nShutting down language server.\n")?;
	Ok(())
}

fn run(logger: &mut Logger) -> anyhow::Result<()> {
	logger.log("\n\n# Awaiting next request from client...\n")?;

	let mut responder = std::io::stdout();
	let mut state = State { files: HashMap::new() };

	let stdin = std::io::stdin();
	let mut reader = BufReader::new(stdin.lock());

	loop {
		let mut line = String::new();

		let mut content_length: Option<usize> = None;
		loop {
			line.clear();
			let bytes_read = reader.read_line(&mut line)?;
			if bytes_read == 0 {
				return Ok(()); // EOF reached, client disconnected
			}

			// End of header
			if line == "\r\n" || line == "\n" {
				break;
			}

			// content length
			if line.starts_with("Content-Length:") {
				let parts: Vec<&str> = line.split(':').collect();
				if parts.len() >= 2 {
					content_length = Some(parts[1].trim().parse()?);
				}
			}

			// ignore other headers like Content-Type for now
		}

		let Some(len) = content_length else {
			logger.log("\n**ERROR:** Missing Content-Length header\n")?;
			continue;
		};

		// 2. Read the exact payload body based on the Content-Length
		let mut body = vec![0; len];
		reader.read_exact(&mut body)?;

		let content = String::from_utf8(body)?;

		logger.log("\n**Request received:**\n```json\n")?;
		logger.log(format!("{content}\n```\n"))?;

		// 3. Handle the successfully parsed request
		handle_request(logger, &mut state, &mut responder, &content)?;
		logger.log("\n\n# Awaiting next request from client...\n")?;
	}
}

fn handle_request(logger: &mut Logger, state: &mut State, responder: &mut Stdout, content: &str) -> anyhow::Result<()> {
	logger.log(format!("\n**Handling request:**\n```json\n{content}\n```\n"))?;

	let request: lsp::Request = serde_json::from_str(content).map_err(|error| anyhow::anyhow!("Error deserializing content: {error}; content: {content}"))?;

	// Response
	let response = request.response(state, logger)?;

	if let Some(response) = response {
		let response_string: String = response.try_into()?;
		logger.log(format!("\n**Response:**\n```json\n{response_string}\n```"))?;
		responder.write_all(response_string.as_bytes())?;
		responder.flush()?;
	} else {
		logger.log("\n**No response needed.**\n")?;
	}

	Ok(())
}

struct Logger {
	file: File,
}

impl Logger {
	fn log<S: AsRef<str>>(&mut self, message: S) -> std::io::Result<()> {
		self.file.write_all(message.as_ref().replace('\r', "").as_bytes())
	}
}
