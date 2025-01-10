use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, item::container::item::EquipStatus},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct RemoveItem(pub ItemRef);

crate::impl_trait_eq!(RemoveItem);
kdlize::impl_kdl_node!(RemoveItem, "item_remove");

impl Change for RemoveItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let inventory = &mut character.persistent_mut().inventory;
		let equip_status = match self.0.path.as_single() {
			Some(equipment_id) => inventory.get_equip_status(&equipment_id),
			None => EquipStatus::Unequipped,
		};
		let _item = inventory.remove_at_path(&self.0.path);
		if equip_status != EquipStatus::Unequipped {
			character.persistent_mut().mark_structurally_changed();
		}
	}
}

impl FromKdl<NodeContext> for RemoveItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		Ok(Self(ItemRef::from_kdl(node)?))
	}
}

impl AsKdl for RemoveItem {
	fn as_kdl(&self) -> NodeBuilder {
		self.0.as_kdl()
	}
}
