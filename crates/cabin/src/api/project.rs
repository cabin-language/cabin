use std::path::PathBuf;

pub struct Project {
	root_directory: PathBuf,
}

impl Project {
	pub fn new(root_directory: &PathBuf) -> Project {
		Self {
			root_directory: root_directory.to_owned(),
		}
	}

	pub const fn root_directory(&self) -> &PathBuf {
		&self.root_directory
	}

	pub fn main_file(&self) -> PathBuf {
		self.root_directory.join("src").join("main.cabin")
	}
}
