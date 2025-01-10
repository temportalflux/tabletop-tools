use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::Character,
			item::container::item::{EquipStatus, ItemPath},
		},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct RemoveItem {
	pub path: ItemPath,
	pub name: Vec<String>,
}

crate::impl_trait_eq!(RemoveItem);
kdlize::impl_kdl_node!(RemoveItem, "item_remove");

impl Change for RemoveItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let equip_status = match self.path.as_single() {
			Some(equipment_id) => character.persistent().inventory.get_equip_status(&equipment_id),
			None => EquipStatus::Unequipped,
		};
		let _item = character.persistent_mut().inventory.remove_at_path(&self.path);
		if equip_status != EquipStatus::Unequipped {
			character.persistent_mut().mark_structurally_changed();
		}
	}
}

impl FromKdl<NodeContext> for RemoveItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let path = node.next_str_req_t()?;
		let name = {
			let iter = node.next_str_req()?.split("/");
			let iter = iter.map(str::to_owned);
			iter.collect()
		};
		Ok(Self { path, name })
	}
}

impl AsKdl for RemoveItem {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default().with_entry(self.path.to_string()).with_entry(self.name.join("/"))
	}
}
