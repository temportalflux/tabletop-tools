use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, roll::Die},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyHitDice {
	pub die: Die,
	// how many hit dice the character should have
	pub amount_remaining: u32,
}

crate::impl_trait_eq!(ApplyHitDice);
kdlize::impl_kdl_node!(ApplyHitDice, "hit_dice");

impl Change for ApplyHitDice {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let capacity = character.hit_dice().dice()[self.die] as u32;
		let Some(data_path) = character.hit_points().hit_dice_selectors[self.die].get_data_path() else { return };
		let uses_consumed = capacity.saturating_sub(self.amount_remaining);
		character.persistent_mut().set_selected_value(data_path, uses_consumed.to_string());
	}
}

impl FromKdl<NodeContext> for ApplyHitDice {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let die = node.next_str_req_t()?;
		let amount_remaining = node.next_i64_req()? as u32;
		Ok(Self { die, amount_remaining })
	}
}

impl AsKdl for ApplyHitDice {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.die.to_string());
		node.entry(self.amount_remaining as i64);
		node
	}
}
