use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::Character,
			item::{container::item::ItemPath, Item},
		},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct AddItem {
	pub container: Option<(ItemPath, Vec<String>)>,
	pub item: Item,
}

crate::impl_trait_eq!(AddItem);
kdlize::impl_kdl_node!(AddItem, "item_add");

impl Change for AddItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let container = self.container.as_ref().map(|(id, _)| id);
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
		let container = match node.query_opt("scope() > dest")? {
			None => None,
			Some(mut node) => {
				let path = node.next_str_req_t()?;
				let names = {
					let iter = node.next_str_req()?;
					let iter = iter.split("/").map(str::to_owned);
					iter.collect()
				};
				Some((path, names))
			}
		};
		Ok(Self { item, container })
	}
}

impl AsKdl for AddItem {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("item", &self.item));
		if let Some((path, name)) = &self.container {
			node.child(("dest", NodeBuilder::default().with_entry(path.to_string()).with_entry(name.join("/"))))
		}
		node
	}
}
