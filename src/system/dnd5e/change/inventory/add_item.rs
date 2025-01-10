use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, item::Item},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

#[derive(Clone, Debug, PartialEq)]
pub struct AddItem {
	pub container: Option<ItemRef>,
	pub item: Item,
}

crate::impl_trait_eq!(AddItem);
kdlize::impl_kdl_node!(AddItem, "item_add");

impl Change for AddItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let container = self.container.as_ref().map(|item| &item.path);
		character.persistent_mut().inventory.insert_to(self.item.clone(), container);

		// need items to have their data paths set up
		// (normally this isn't needed until an item is equipped,
		// but equipment with charges can be viewed without being actively equipped)
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for AddItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let item = node.query_req_t("scope() > item")?;
		let container = node.query_opt_t("scope() > dest")?;
		Ok(Self { item, container })
	}
}

impl AsKdl for AddItem {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("item", &self.item));
		node.child(("dest", &self.container, OmitIfEmpty));
		node
	}
}
