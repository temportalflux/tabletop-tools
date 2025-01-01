use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::character::{Character, DeathSave},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct DeathSaves {
	pub save: DeathSave,
	pub delta: i8,
}

crate::impl_trait_eq!(DeathSaves);
kdlize::impl_kdl_node!(DeathSaves, "death_saves");

impl Change for DeathSaves {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let save_count = &mut character.persistent_mut().hit_points_mut().saves[self.save];
		*save_count = save_count.saturating_add_signed(self.delta);
	}
}

impl FromKdl<NodeContext> for DeathSaves {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let save = node.next_str_req_t()?;
		let delta = node.next_i64_req()? as i8;
		Ok(Self { save, delta })
	}
}

impl AsKdl for DeathSaves {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.save.as_str());
		node.entry(self.delta as i64);
		node
	}
}
