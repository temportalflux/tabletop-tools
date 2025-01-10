use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

#[derive(Clone, Debug, PartialEq)]
pub struct MoveItem {
	pub item: ItemRef,
	pub destination_container: Option<ItemRef>,
}

crate::impl_trait_eq!(MoveItem);
kdlize::impl_kdl_node!(MoveItem, "item_move");

impl Change for MoveItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let inventory = &mut character.persistent_mut().inventory;
		let Some(item) = inventory.remove_at_path(&self.item.path) else { return };
		let container = self.destination_container.as_ref().map(|item| &item.path);
		inventory.insert_to(item, container);
	}
}

impl FromKdl<NodeContext> for MoveItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let item = node.query_req_t("scope() > item")?;
		let destination_container = node.query_opt_t("scope() > dest")?;
		Ok(Self { item, destination_container })
	}
}

impl AsKdl for MoveItem {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("item", &self.item));
		node.child(("dest", &self.destination_container, OmitIfEmpty));
		node
	}
}
