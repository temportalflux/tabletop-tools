use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::Character,
			currency::Wallet,
			item::{self, container::item::ItemPath, Item},
		},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

#[derive(Clone, Debug, PartialEq)]
pub struct PurchaseItem {
	pub item: Item,
	pub amount: usize,
	pub cost: Wallet,
	pub container: Option<(ItemPath, Vec<String>)>,
}

crate::impl_trait_eq!(PurchaseItem);
kdlize::impl_kdl_node!(PurchaseItem, "item_purchase");

impl Change for PurchaseItem {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let mut item = self.item.clone();
		let items = match &mut item.kind {
			item::Kind::Simple { count } => {
				*count *= self.amount as u32;
				vec![item]
			}
			_ => {
				let mut items = Vec::with_capacity(self.amount);
				items.resize(self.amount, item);
				items
			}
		};

		if !self.cost.is_empty() {
			let auto_exchange = character.persistent().settings.currency_auto_exchange;
			character.persistent_mut().inventory.wallet_mut().remove(self.cost, auto_exchange);
		}

		let container = self.container.as_ref().map(|(id, _)| id);
		for item in items {
			character.persistent_mut().inventory.insert_to(item, container);
		}

		// need items to have their data paths set up
		// (normally this isn't needed until an item is equipped,
		// but equipment with charges can be viewed without being actively equipped)
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for PurchaseItem {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let amount = node.get_i64_req("amount")? as usize;
		let cost = node.query_opt_t::<Wallet>("scope() > wallet")?.unwrap_or_default();
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
		Ok(Self { item, amount, cost, container })
	}
}

impl AsKdl for PurchaseItem {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(("amount", self.amount as i64));
		node.child(("cost", &self.cost, OmitIfEmpty));
		node.child(("item", &self.item));
		if let Some((path, name)) = &self.container {
			node.child(("dest", NodeBuilder::default().with_entry(path.to_string()).with_entry(name.join("/"))))
		}
		node
	}
}
