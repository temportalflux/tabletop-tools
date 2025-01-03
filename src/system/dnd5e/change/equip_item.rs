use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, item::container::item::EquipStatus},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct EquipItem {
	pub id: Uuid,
	pub status: EquipStatus,
}

crate::impl_trait_eq!(EquipItem);
kdlize::impl_kdl_node!(EquipItem, "equip_item");

impl Change for EquipItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		character.persistent_mut().inventory.set_equipped(&self.id, self.status);
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for EquipItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let status = node.next_str_req_t()?;
		let id = node.next_str_req_t()?;
		Ok(Self { id, status })
	}
}

impl AsKdl for EquipItem {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default().with_entry(self.status.to_string()).with_entry(self.id.to_string())
	}
}
