use std::{
	fmt::Write as _,
	path::{Path, PathBuf},
};

use super::context::StandardContext;
use crate::{
	api::{config::Config, diagnostics::Diagnostics},
	ast::{expressions::name::Name, misc::program::Program},
	comptime::CompileTime as _,
	transpiler::TranspileToC as _,
	Context,
};

#[derive(thiserror::Error, Debug)]
pub enum ProjectError {
	#[error("Project directory doesn't exist.")]
	RootDirectoryDoesntExist,

	/// The error that occurs when there is no `cabin.toml` file locateable.
	#[error("No cabin.toml file exists in the project root.")]
	ConfigFileDoesntExist,

	/// The error that occurs when the user's config file contains invalid TOML data, or is
	/// missing required information.
	#[error("cabin.toml contains invalid data: {0}")]
	MalformattedConfigFile(toml_edit::de::Error),

	/// The error that occurs when a project has no `src/main.cabin`.
	#[error("No main file found.")]
	NoMainFile,
}

pub struct Project {
	root_directory: PathBuf,
	config: Config,
	context: StandardContext,
	program: Option<Program>,
	main_file_contents: String,
}

impl Project {
	/// Reads a Cabin project, creating a `Project` object as a result.
	///
	/// # Parameters
	///
	/// - `root_directory` - The project's root directory. To create a `Project` from anywhere
	/// nested within a project, use `Project::from_child()`.
	///
	/// # Returns
	///
	/// The project object, or an error if the project is corrupted in some way (root
	/// directory doesn't exist, `cabin.toml` doesn't exist, etc.)
	///
	/// # Errors
	///
	/// Several errors can occur while instantiating the project object:
	///
	/// - If the given `root_directory` doesn't exist, an error is returned.
	/// - If the project has no `cabin.toml` in it's root, or it does but it contains invalid data,
	/// an error is returned.
	/// - If the project has no locateable main file (`src/main.cabin` by default, or otherwise as
	/// specified in `cabin.toml`), an error is returned.
	pub fn from_root<P: AsRef<Path>>(root_directory: P) -> Result<Project, ProjectError> {
		let root_directory = root_directory.as_ref();
		if !root_directory.is_dir() {
			return Err(ProjectError::RootDirectoryDoesntExist);
		}

		let config_file = root_directory.join("cabin.toml");
		let config_contents = std::fs::read_to_string(config_file).map_err(|_error| ProjectError::ConfigFileDoesntExist)?;
		let config = toml_edit::de::from_str(&config_contents).map_err(ProjectError::MalformattedConfigFile)?;

		let main_file = root_directory.join("src").join("main.cabin");
		let main_file_contents = std::fs::read_to_string(&main_file).map_err(|_error| ProjectError::NoMainFile)?;

		let mut context = Context::default();
		context.file = main_file;

		Ok(Project {
			root_directory: root_directory.into(),
			context,
			program: None,
			config,
			main_file_contents,
		})
	}

	pub fn from_child<P: AsRef<Path>>(directory: P) -> Result<Project, ProjectError> {
		let mut directory = directory.as_ref().canonicalize().map_err(|_| ProjectError::RootDirectoryDoesntExist)?;
		while !directory.join("cabin.toml").is_file() {
			directory = directory.parent().ok_or(ProjectError::ConfigFileDoesntExist)?.into();
		}
		Project::from_root(directory)
	}

	pub const fn root_directory(&self) -> &PathBuf {
		&self.root_directory
	}

	pub const fn config(&self) -> &Config {
		&self.config
	}

	pub const fn context(&self) -> &StandardContext {
		&self.context
	}

	pub fn context_mut(&mut self) -> &mut StandardContext {
		&mut self.context
	}

	pub fn run_compile_time_code(&mut self) -> &Diagnostics {
		let program = crate::parse_program(&self.main_file_contents, &mut self.context);

		self.program = Some(program.evaluate_at_compile_time(&mut self.context));

		self.context.diagnostics()
	}

	pub fn check(&mut self) -> &Diagnostics {
		self.context.side_effects = false;

		let program = crate::parse_program(&self.main_file_contents, &mut self.context);
		if !self.context.diagnostics().is_empty() {
			return self.context.diagnostics();
		}

		self.program = Some(program.evaluate_at_compile_time(&mut self.context));
		self.context.side_effects = true;

		self.context.diagnostics()
	}

	/// Returns the name at the given byte position in the source code, or `None` if there is no
	/// name at that byte position. The returned name contains environment (scope) information, so
	/// this requires tokenizing and parsing the program, if it wasn't already.
	pub fn name_at(&mut self, name_position: usize) -> Option<Name> {
		self.context.side_effects = false;

		self.context.name_query = Some(name_position);
		let program = crate::parse_program(&self.main_file_contents, &mut self.context);
		if !self.context.diagnostics().is_empty() {
			return None;
		}

		self.program = Some(program.evaluate_at_compile_time(&mut self.context));
		self.context.side_effects = true;

		self.context().name_query_result.clone()
	}

	pub const fn printed(&self) -> bool {
		self.context.has_printed
	}

	pub fn transpile(&mut self) -> Result<String, Diagnostics> {
		if self.program.is_none() {
			let diagnostics = self.run_compile_time_code();
			if !diagnostics.errors().is_empty() {
				return Err(diagnostics.to_owned());
			}
		}

		let mut c_code = "#include <stdio.h>\n#include<stdlib.h>\n\nint main(int argc, char* argv[]) {\n\n".to_owned();

		let body = self.program.as_ref().unwrap().to_c(&mut self.context, None).unwrap();
		body.lines().for_each(|line| writeln!(c_code, "\t{line}").unwrap());

		c_code += "\n\n\treturn 0;\n}";

		Ok(c_code)
	}
}
