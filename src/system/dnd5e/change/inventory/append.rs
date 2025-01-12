use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::{Character, Persistent},
			currency::Wallet,
			item::{
				container::item::{ItemContainerTrait, ItemPath},
				Item,
			},
		},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

#[derive(Clone, Debug, PartialEq)]
pub struct AppendItems {
	pub container: Option<ItemRef>,
	pub items: Vec<Item>,
	pub currency: Wallet,
}

crate::impl_trait_eq!(AppendItems);
kdlize::impl_kdl_node!(AppendItems, "item_append");

fn find_container<'c>(
	persistent: &'c mut Persistent, path: Option<&ItemPath>,
) -> Option<Box<&'c mut dyn ItemContainerTrait>> {
	match path {
		None => Some(Box::new(&mut persistent.inventory)),
		Some(path) => {
			let Some(item) = persistent.inventory.get_mut_at_path(path) else { return None };
			let Some(container) = &mut item.items else { return None };
			Some(Box::new(container))
		}
	}
}

impl Change for AppendItems {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let item_path = self.container.as_ref().map(|item_ref| &item_ref.path);
		let Some(container) = find_container(character.persistent_mut(), item_path) else { return };
		*container.wallet_mut() += self.currency;
		for item in &self.items {
			container.insert(item.clone());
		}
	}
}

impl FromKdl<NodeContext> for AppendItems {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let container = node.query_opt_t("scope() > dest")?;
		let items = node.query_all_t("scope() > item")?;
		let currency = node.query_opt_t("scope() > currency")?.unwrap_or_default();
		Ok(Self { container, items, currency })
	}
}

impl AsKdl for AppendItems {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("dest", &self.container, OmitIfEmpty));
		node.children(("item", &self.items));
		node.child(("currency", &self.currency, OmitIfEmpty));
		node
	}
}
