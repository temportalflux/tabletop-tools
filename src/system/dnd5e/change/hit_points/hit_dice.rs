use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, roll::Die},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct HitDice {
	pub die: Die,
	// how many hit dice are used by this operation (values are stored as "amount consumed")
	pub delta: i32,
}

crate::impl_trait_eq!(HitDice);
kdlize::impl_kdl_node!(HitDice, "hit_dice");

impl Change for HitDice {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let Some(data_path) = character.hit_points().hit_dice_selectors[self.die].get_data_path() else { return };
		let prev_value = character.persistent().get_first_selection_at::<u32>(&data_path);
		let prev_value = prev_value.map(Result::ok).flatten().unwrap_or_default();
		let new_value = prev_value.saturating_add_signed(self.delta);
		character.persistent_mut().set_selected(data_path, (new_value > 0).then(|| new_value.to_string()));
	}
}

impl FromKdl<NodeContext> for HitDice {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let die = node.next_str_req_t()?;
		let delta = node.next_i64_req()? as i32;
		Ok(Self { die, delta })
	}
}

impl AsKdl for HitDice {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.die.to_string());
		node.entry(self.delta as i64);
		node
	}
}
