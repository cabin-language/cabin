use std::{path::PathBuf, process::Command};

use cabin::config::ProjectType;

use super::CabinCommand;

#[derive(clap::Parser)]
pub struct NewCommand {
	name: Option<String>,

	#[arg(long, default_value_t = false)]
	library: bool,
}

impl CabinCommand for NewCommand {
	fn execute(self) {
		cliclack::intro("New Cabin project").unwrap();

		let name: String = cliclack::input("Project name").default_input("example-project").interact().unwrap();

		let location: PathBuf = cliclack::input("Location").default_input(&format!("./{name}")).interact().unwrap();

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

		let mut config = cabin::config::Config::default();
		config.information.project_type = project_type;
		config.information.name = name;
		config.information.description = description;

		// Project
		std::fs::create_dir_all(location.join("src")).unwrap();
		std::fs::write(location.join("src").join("main.cabin"), "run print(\"Hello world!\");").unwrap();
		std::fs::write(location.join("cabin.toml"), toml_edit::ser::to_string_pretty(&config).unwrap()).unwrap();

		// Cache
		std::fs::create_dir_all(location.join("cache").join("libraries")).unwrap();

		// Builds
		std::fs::create_dir_all(location.join("builds")).unwrap();

		// Git
		if which::which("git").is_ok() {
			Command::new("git").arg("init").arg("-q").current_dir(&location).spawn().unwrap();
			std::fs::write(location.join(".gitignore"), "cache/\ncabin.local.toml").unwrap();
		}

		cliclack::outro("Done!").unwrap();
		println!();
	}
}
