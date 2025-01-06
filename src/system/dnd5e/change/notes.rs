use std::path::PathBuf;

use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyNotes {
	Insert(PathBuf, String),
	Remove(PathBuf),
}

crate::impl_trait_eq!(ApplyNotes);
kdlize::impl_kdl_node!(ApplyNotes, "notes");

impl Change for ApplyNotes {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::Insert(path, value) => {
				character.persistent_mut().notes.set(path, value.clone());
			}
			Self::Remove(path) => {
				character.persistent_mut().notes.remove(path);
			}
		}
	}
}

impl FromKdl<NodeContext> for ApplyNotes {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let path = node.next_str_req_t()?;
		let value = node.next_str_opt()?.map(str::to_owned);
		Ok(match value {
			Some(value) => Self::Insert(path, value),
			None => Self::Remove(path),
		})
	}
}

impl AsKdl for ApplyNotes {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Insert(path, value) => {
				node.entry(path.to_str().unwrap());
				node.entry(value.as_str());
			}
			Self::Remove(path) => {
				node.entry(path.to_str().unwrap());
			}
		}
		node
	}
}
