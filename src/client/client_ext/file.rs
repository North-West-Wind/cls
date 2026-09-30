use crate::common::base::file::SaveableFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientFile {
	pub base: SaveableFile,
	pub duration: String,
}

impl Default for ClientFile {
	fn default() -> Self {
		Self {
			base: SaveableFile::default(),
			duration: String::new()
		}
	}
}

impl From<SaveableFile> for ClientFile {
	fn from(base: SaveableFile) -> Self {
		Self {
			base,
			duration: String::new()
		}
	}
}