use crate::system::{generics, SourceId};
use database::Record;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

mod module;
pub use module::*;
mod module_system;
pub use module_system::*;
mod system;
pub use system::*;
mod system_category;
pub use system_category::*;
mod system_category_variants;
pub use system_category_variants::*;
mod system_variants;
pub use system_variants::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
	pub id: String,
	pub module: String,
	pub system: String,
	pub category: String,
	pub version: Option<String>,
	pub metadata: serde_json::Value,
	pub kdl: String,
	pub file_id: Option<String>,
	// The stringify SourceId of the generator that created this variant.
	// The object which was used as a basis is this entry's id w/o the variant index.
	// If the generator id matches this entry id w/o variant index, then this object
	// had no external basis (the generator creates object entries from scratch).
	pub generator_id: Option<String>,
	// marker for variants in order for idb to index (idb cannot check if `generator_id` is non-empty)
	pub generated: u8,
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

	pub fn generator_id(&self) -> Option<SourceId> {
		match &self.generator_id {
			None => None,
			Some(id_str) => SourceId::from_str(id_str).ok(),
		}
	}

	pub fn get_meta_str(&self, key: impl AsRef<str>) -> Option<&str> {
		let Some(value) = self.metadata.get(key.as_ref()) else {
			return None;
		};
		value.as_str()
	}

	pub fn name(&self) -> Option<&str> {
		self.get_meta_str("name")
	}

	pub fn parse_kdl<T: kdlize::FromKdl<crate::kdl_ext::NodeContext>>(
		&self, node_reg: Arc<generics::Registry>,
	) -> Option<T> where T::Error: std::fmt::Debug {
		// Parse the entry's kdl string:
		// kdl string to document
		let document = match self.kdl.parse::<kdl::KdlDocument>() {
			Ok(doc) => doc,
			Err(err) => {
				log::error!(target: "query:parse_kdl", "{err:?}");
				return None;
			}
		};
		// document to first (and hopefully only) node
		let Some(node) = document.nodes().get(0) else {
			log::error!(target: "query:parse_kdl", "missing root node");
			return None;
		};
		let ctx = crate::kdl_ext::NodeContext::new(Arc::new(self.source_id(true)), node_reg);
		let value = match T::from_kdl(&mut crate::kdl_ext::NodeReader::new_root(node, ctx)) {
			Ok(value) => value,
			Err(err) => {
				log::error!(target: "query:parse_kdl", "{err:?}");
				return None;
			}
		};
		Some(value)
	}
}
