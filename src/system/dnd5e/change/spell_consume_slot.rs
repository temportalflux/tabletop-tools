use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change, SourceId},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ConsumeSpellSlot {
	rank: u8,
	slots_available: usize,
	// for reference in UI
	spell_id: Option<SourceId>,
}

crate::impl_trait_eq!(ConsumeSpellSlot);
kdlize::impl_kdl_node!(ConsumeSpellSlot, "consume_spell_slot");

impl ConsumeSpellSlot {
	pub fn manual(rank: u8, slots_available: usize) -> Self {
		Self { rank, slots_available, spell_id: None }
	}

	pub fn cast_spell(rank: u8, slots_available: usize, spell_id: SourceId) -> Self {
		Self { rank, slots_available, spell_id: Some(spell_id) }
	}
}

impl Change for ConsumeSpellSlot {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let Some(slots) = character.spellcasting().spell_slots(character) else { return };
		let Some(slot_count) = slots.get(&self.rank) else { return };
		let data_path = character.persistent().selected_spells.consumed_slots_path(self.rank);
		let slots_consumed = slot_count.saturating_sub(self.slots_available);
		character.persistent_mut().set_selected_value(&data_path, slots_consumed.to_string());
	}
}

impl FromKdl<NodeContext> for ConsumeSpellSlot {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rank = node.next_i64_req()? as u8;
		let slots_available = node.next_i64_req()? as usize;
		let spell_id = node.next_str_opt_t()?;
		Ok(Self { rank, slots_available, spell_id })
	}
}

impl AsKdl for ConsumeSpellSlot {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.rank as i64);
		node.entry(self.slots_available as i64);
		if let Some(id) = &self.spell_id {
			node.entry(("spell", id.to_string()));
		}
		node
	}
}
