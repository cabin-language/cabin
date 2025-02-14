use std::collections::HashMap;

#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize)]
pub struct Config {
	options: CompilerOptions,
	information: ProjectInformation,
	libraries: HashMap<String, Library>,
}

#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize)]
pub struct ProjectInformation {
	name: String,
	description: String,
	version: String,

	#[serde(rename = "type")]
	project_type: ProjectType,
}

#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize)]
pub struct CompilerOptions {
	#[serde(default)]
	quiet: bool,
}

#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize)]
pub struct Library {
	version: String,
	git: Option<String>,
	branch: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
	Program,
	Library,
}
