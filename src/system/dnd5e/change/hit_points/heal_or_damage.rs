use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, HitPoint},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct HealOrDamage(pub i32);

crate::impl_trait_eq!(HealOrDamage);
kdlize::impl_kdl_node!(HealOrDamage, "heal_or_damage");

impl Change for HealOrDamage {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let max_hp = character.get_hp(HitPoint::Max);
		*character.persistent_mut().hit_points_mut() += (self.0, max_hp);
	}
}

impl FromKdl<NodeContext> for HealOrDamage {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let amount = node.next_i64_req()? as i32;
		Ok(Self(amount))
	}
}

impl AsKdl for HealOrDamage {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.0 as i64);
		node
	}
}
