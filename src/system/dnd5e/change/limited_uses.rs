use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyLimitedUses(pub PathBuf, pub u32);

crate::impl_trait_eq!(ApplyLimitedUses);
kdlize::impl_kdl_node!(ApplyLimitedUses, "limited_uses");

impl Change for ApplyLimitedUses {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		character.persistent_mut().set_selected_value(&self.0, self.1.to_string());
	}
}

impl FromKdl<NodeContext> for ApplyLimitedUses {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let data_path = node.next_str_req_t()?;
		let uses_consumed = node.next_i64_req()? as u32;
		Ok(Self(data_path, uses_consumed))
	}
}

impl AsKdl for ApplyLimitedUses {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.0.to_str().unwrap().to_owned());
		node.entry(self.1 as i64);
		node
	}
}
