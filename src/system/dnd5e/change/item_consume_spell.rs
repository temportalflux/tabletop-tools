use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change, SourceId},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ConsumeItemSpell {
	pub item_path: Vec<uuid::Uuid>,
	pub spell_id: SourceId,
	pub consume_item: bool,
}

crate::impl_trait_eq!(ConsumeItemSpell);
kdlize::impl_kdl_node!(ConsumeItemSpell, "consume_item_spell");

impl Change for ConsumeItemSpell {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let inventory = &mut character.persistent_mut().inventory;
		let Some(item) = inventory.get_mut_at_path(&self.item_path) else { return };
		let Some(spell_container) = &mut item.spells else { return };
		if spell_container.remove(&self.spell_id) <= 0 {
			return;
		}
		if self.consume_item && spell_container.spells.is_empty() {
			inventory.remove_at_path(&self.item_path);
		}
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for ConsumeItemSpell {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let consume_item = node.get_bool_opt("consume_item")?.unwrap_or_default();
		let item_path = {
			let mut node = node.query_req("scope() > item")?;
			let mut item_path = Vec::with_capacity(3);
			while let Some(id) = node.next_str_opt_t()? {
				item_path.push(id);
			}
			item_path
		};
		let spell_id = node.query_str_req_t("scope() > spell", 0)?;
		Ok(Self { item_path, spell_id, consume_item })
	}
}

impl AsKdl for ConsumeItemSpell {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		if self.consume_item {
			node.entry(("consume_item", true));
		}
		node.child(("item", {
			let iter_item_ids = self.item_path.iter();
			iter_item_ids.fold(NodeBuilder::default(), |node, id| node.with_entry(id.to_string()))
		}));
		node.child(("spell", self.spell_id.to_string()));
		node
	}
}
