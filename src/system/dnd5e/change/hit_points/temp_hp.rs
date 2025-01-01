use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct TempHP(pub u32);

crate::impl_trait_eq!(TempHP);
kdlize::impl_kdl_node!(TempHP, "temp_hp");

impl Change for TempHP {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		character.persistent_mut().hit_points_mut().temp = self.0;
	}
}

impl FromKdl<NodeContext> for TempHP {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		Ok(Self(node.next_i64_req()? as u32))
	}
}

impl AsKdl for TempHP {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default().with_entry(self.0 as i64)
	}
}
