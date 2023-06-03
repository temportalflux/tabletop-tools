use crate::{database::Record, system::core::SourceId};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod module_system;
pub use module_system::*;
mod system;
pub use system::*;
mod system_category;
pub use system_category::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
	pub id: String,
	pub module: String,
	pub system: String,
	pub category: String,
	pub version: Option<String>,
	pub metadata: serde_json::Value,
	pub kdl: String,
}

impl Record for Entry {
	fn store_id() -> &'static str {
		"entries"
	}
}

impl Entry {
	pub fn source_id(&self, with_version: bool) -> SourceId {
		let mut id = SourceId::from_str(&self.id).unwrap();
		if with_version {
			id.version = self.version.clone();
		}
		id
	}

	pub fn get_meta_str(&self, key: impl AsRef<str>) -> Option<&str> {
		let Some(value) = self.metadata.get(key.as_ref()) else { return None; };
		value.as_str()
	}

	pub fn name(&self) -> Option<&str> {
		self.get_meta_str("name")
	}
}
