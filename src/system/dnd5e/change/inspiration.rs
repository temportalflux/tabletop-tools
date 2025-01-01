use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleInspiration;

crate::impl_trait_eq!(ToggleInspiration);
kdlize::impl_kdl_node!(ToggleInspiration, "inspiration");

impl Change for ToggleInspiration {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		character.persistent_mut().inspiration = !character.persistent_mut().inspiration;
	}
}

impl FromKdl<NodeContext> for ToggleInspiration {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(_node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		Ok(Self)
	}
}

impl AsKdl for ToggleInspiration {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default()
	}
}
