use std::collections::HashMap;

use serde_inline_default::serde_inline_default;
use smart_default::SmartDefault;

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
	pub information: ProjectInformation,
	pub options: CompilerOptions,
	pub libraries: HashMap<String, Library>,
}

#[derive(serde::Serialize, serde::Deserialize, SmartDefault)]
pub struct ProjectInformation {
	pub name: String,
	pub description: String,

	#[default = "0.1.0"]
	version: String,

	#[serde(rename = "type")]
	pub project_type: ProjectType,
}

#[serde_inline_default(true)]
#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize, SmartDefault)]
pub struct CompilerOptions {
	#[default = false]
	#[serde_inline_default(false)]
	quiet: bool,
}

#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize)]
pub struct Library {
	version: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, SmartDefault)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
	#[default]
	Program,
	Library,
}

#[serde_inline_default]
#[derive(derive_getters::Getters, serde::Serialize, serde::Deserialize, SmartDefault)]
pub struct LocalConfig {
	#[default = true]
	#[serde_inline_default(true)]
	icons: bool,
}
