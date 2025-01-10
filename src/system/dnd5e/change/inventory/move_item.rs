use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, item::container::item::ItemPath},
		Change,
	},
};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct MoveItem {
	pub item: (ItemPath, Vec<String>),
	pub destination_container: Option<(ItemPath, Vec<String>)>,
}

crate::impl_trait_eq!(MoveItem);
kdlize::impl_kdl_node!(MoveItem, "item_move");

impl Change for MoveItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let Some(item) = character.persistent_mut().inventory.remove_at_path(&self.item.0) else { return };
		let container = self.destination_container.as_ref().map(|(path, _)| path);
		character.persistent_mut().inventory.insert_to(item, container);
	}
}

impl FromKdl<NodeContext> for MoveItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let item_path = node.query_str_req_t("scope() > item", 0)?;
		let item_name = {
			let iter = node.query_str_req("scope() > item", 1)?.split("/");
			let iter = iter.map(str::to_owned);
			iter.collect()
		};
		let destination_container = match node.query_opt("scope() > dest")? {
			None => None,
			Some(mut node) => {
				let path = node.next_str_req_t()?;
				let names = {
					let iter = node.next_str_req()?.split("/");
					let iter = iter.map(str::to_owned);
					iter.collect()
				};
				Some((path, names))
			}
		};
		Ok(Self { item: (item_path, item_name), destination_container })
	}
}

impl AsKdl for MoveItem {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child((
			"item",
			NodeBuilder::default().with_entry(self.item.0.to_string()).with_entry(self.item.1.join("/")),
		));
		if let Some((path, name)) = &self.destination_container {
			node.child(("dest", NodeBuilder::default().with_entry(path.to_string()).with_entry(name.join("/"))));
		}
		node
	}
}
