use crate::{io::Io, Context};

#[derive(Debug, Clone, thiserror::Error)]
pub enum TranspileError {
	#[error("Attempted to transpile an AST that contains an error")]
	TranspileError,
}

pub(crate) trait TranspileToC {
	/// Transpiles this AST node into C code.
	///
	/// # Parameters
	///
	/// - `context` - Global data about the program.
	///
	/// # Returns
	///
	/// The C code for this AST node, or an error if this AST node contains (or is) an error.
	///
	/// # Errors
	///
	/// If this AST node is invalid, meaning it contains an error node, an error is returned.
	fn to_c<System: Io>(&self, context: &mut Context<System>, output: Option<String>) -> Result<String, TranspileError>;

	fn c_prelude<System: Io>(&self, _context: &mut Context<System>) -> Result<String, TranspileError> {
		Ok(String::new())
	}

	fn c_type_prelude<System: Io>(&self, _context: &mut Context<System>) -> Result<String, TranspileError> {
		Ok(String::new())
	}
}
