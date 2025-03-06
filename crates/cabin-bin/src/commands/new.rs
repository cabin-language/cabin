use std::{path::PathBuf, process::Command};

use cabin::{
	config::ProjectType,
	theme::{CatppuccinMocha, Theme as _},
};
use colored::Colorize;

use super::CabinCommand;

#[derive(clap::Parser)]
pub struct NewCommand {
	name: Option<String>,

	#[arg(long, default_value_t = false)]
	library: bool,
}

impl CabinCommand for NewCommand {
	fn execute(self) {
		let mut config = cabin::config::Config::default();
		let location;

		// Non-interactive
		let interactive = if let Some(name) = self.name {
			location = PathBuf::from(&name);
			config.information.name = name;
			config.information.project_type = if self.library { ProjectType::Library } else { ProjectType::Program };
			config.information.description = "An example Cabin project created with cabin new.".to_owned();
			false
		}
		// Interactive
		else {
			cliclack::intro("New Cabin project").unwrap();

			let name: String = cliclack::input("Project name").default_input("example-project").interact().unwrap();

			location = cliclack::input("Location").default_input(&format!("./{name}")).interact().unwrap();

			if location.exists() {
				cliclack::select("Location already exists. Overwrite it?")
					.initial_value(false)
					.item(true, "Yes", "")
					.item(false, "No", "")
					.interact()
					.unwrap();
			}

			let project_type = cliclack::select("Type:")
				.initial_value(ProjectType::Program)
				.item(ProjectType::Program, "Program", "")
				.item(ProjectType::Library, "Library", "")
				.interact()
				.unwrap();

			let description: String = cliclack::input("Description")
				.default_input("An example Cabin project created with cabin new.")
				.interact()
				.unwrap();

			config.information.project_type = project_type;
			config.information.name = name;
			config.information.description = description;

			true
		};

		// Project
		std::fs::create_dir_all(location.join("src")).unwrap();
		std::fs::write(location.join("src").join("main.cabin"), "run(print(\"Hello world!\"));").unwrap();
		std::fs::write(location.join("cabin.toml"), toml_edit::ser::to_string_pretty(&config).unwrap()).unwrap();

		// Cache
		std::fs::create_dir_all(location.join("cache").join("libraries")).unwrap();

		// Builds
		std::fs::create_dir_all(location.join("builds")).unwrap();

		// Git
		if which::which("git").is_ok() {
			Command::new("git").arg("init").arg("-q").current_dir(location.canonicalize().unwrap()).spawn().unwrap();
			std::fs::write(location.join(".gitignore"), "cache/\ncabin.local.toml").unwrap();
		}

		if interactive {
			cliclack::outro("Done!").unwrap();
		}

		let (parameter_r, parameter_g, parameter_b) = CatppuccinMocha::parameter();
		let (function_r, function_g, function_b) = CatppuccinMocha::function_call();
		println!("\nCreated blank Cabin project at {}. Run it with:\n", location.display().to_string().cyan());
		println!(
			"    {} {}",
			"cd".truecolor(function_r, function_g, function_b),
			location.display().to_string().truecolor(parameter_r, parameter_g, parameter_b)
		);
		println!(
			"    {} {}\n",
			"cabin".truecolor(function_r, function_g, function_b),
			"run".truecolor(function_r, function_g, function_b),
		);
	}
}
