use crate::Context;

#[derive(Debug, Clone, thiserror::Error)]
pub enum TranspileError {
	#[error("Attempted to transpile an AST that contains an error")]
	TranspileError,
}

pub trait TranspileToC {
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
	fn to_c(&self, context: &mut Context, output: Option<String>) -> Result<String, TranspileError>;

	fn c_prelude(&self, _context: &mut Context) -> Result<String, TranspileError> {
		Ok(String::new())
	}

	fn c_type_prelude(&self, _context: &mut Context) -> Result<String, TranspileError> {
		Ok(String::new())
	}
}

pub trait ToCabin {
	fn to_cabin(&self) -> String;
}

impl<T: ToCabin> ToCabin for Option<T> {
	fn to_cabin(&self) -> String {
		self.as_ref().map(|inner| inner.to_cabin()).unwrap_or_default()
	}
}

impl<T: ToCabin> ToCabin for Vec<T> {
	fn to_cabin(&self) -> String {
		self.iter().map(|inner| inner.to_cabin()).collect::<Vec<_>>().join(", ")
	}
}
