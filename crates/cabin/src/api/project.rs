use std::path::{Path, PathBuf};

use crate::{
	api::{config::Config, diagnostics::Diagnostics},
	ast::misc::program::Program,
	comptime::CompileTime,
	transpiler::TranspileToC,
	Context,
};

#[derive(thiserror::Error, Debug)]
pub enum ProjectError {
	#[error("Project directory doesn't exist.")]
	RootDirectoryDoesntExist,

	#[error("No cabin.toml file exists in the project root.")]
	ConfigFileDoesntExist,

	#[error("cabin.toml contains invalid data.")]
	MalformattedConfigFile,

	#[error("No main file found.")]
	NoMainFile,
}

pub struct Project {
	root_directory: PathBuf,
	config: Config,
	context: Context,
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
	pub fn from_root<P: AsRef<Path>>(root_directory: P) -> Result<Project, ProjectError> {
		let root_directory = root_directory.as_ref();
		if !root_directory.is_dir() {
			return Err(ProjectError::RootDirectoryDoesntExist);
		}

		let config_file = root_directory.join("cabin.toml");
		let Ok(config_contents) = std::fs::read_to_string(config_file) else { return Err(ProjectError::ConfigFileDoesntExist) };
		let Ok(config) = toml_edit::de::from_str(&config_contents) else { return Err(ProjectError::MalformattedConfigFile) };

		let main_file = root_directory.join("src").join("main.cabin");
		let Ok(main_file_contents) = std::fs::read_to_string(&main_file) else { return Err(ProjectError::NoMainFile) };

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

	pub fn config(&self) -> &Config {
		&self.config
	}

	pub fn run_compile_time_code(&mut self) -> &Diagnostics {
		let program = crate::parse_program(&self.main_file_contents, &mut self.context);

		self.program = Some(program.evaluate_at_compile_time(&mut self.context));

		self.context.diagnostics()
	}

	pub fn check(&mut self) -> &Diagnostics {
		self.context.side_effects = false;
		let program = crate::parse_program(&self.main_file_contents, &mut self.context);
		self.program = Some(program.evaluate_at_compile_time(&mut self.context));
		self.context.side_effects = true;

		self.context.diagnostics()
	}

	pub fn transpile(&mut self) -> Result<String, Diagnostics> {
		if self.program.is_none() {
			let diagnostics = self.run_compile_time_code();
			if !diagnostics.errors().is_empty() {
				return Err(diagnostics.to_owned());
			}
		}

		let mut c_code = "#include <stdio.h>\n#include<stdlib.h>\n\nint main(int argc, char* argv[]) {\n\n".to_owned();

		for (library_name, library_value) in self.context.libraries.clone() {
			c_code += &format!("\n\t// Library \"{}\" type definitions {}\n\n", library_name.unmangled_name(), "-".repeat(80));
			library_value
				.c_type_prelude(&mut self.context)
				.unwrap()
				.lines()
				.for_each(|line| c_code += &format!("\t{line}\n"));

			c_code += &format!("\n\t// Library \"{}\" value definitions {}\n\n", library_name.unmangled_name(), "-".repeat(80));
			library_value
				.c_prelude(&mut self.context)
				.unwrap()
				.lines()
				.for_each(|line| c_code += &format!("\t{line}\n"));
		}

		let body = self.program.as_ref().unwrap().to_c(&mut self.context, None).unwrap();
		body.lines().for_each(|line| c_code += &format!("\t{line}\n"));

		c_code += "\n\n\treturn 0;\n}";

		Ok(c_code)
	}
}
