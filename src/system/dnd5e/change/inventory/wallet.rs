use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, currency::Wallet},
		Change,
	},
	utility::NotInList,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateWallet {
	operation: Operation,
	container: Option<ItemRef>,
}

#[derive(Clone, Debug, PartialEq)]
enum Operation {
	Add(Wallet),
	Remove(Wallet),
	Exchange,
}

crate::impl_trait_eq!(UpdateWallet);
kdlize::impl_kdl_node!(UpdateWallet, "wallet");

impl UpdateWallet {
	pub fn add(wallet: Wallet, container: Option<ItemRef>) -> Self {
		Self { container, operation: Operation::Add(wallet) }
	}

	pub fn remove(wallet: Wallet, container: Option<ItemRef>) -> Self {
		Self { container, operation: Operation::Remove(wallet) }
	}

	pub fn exchange(container: Option<ItemRef>) -> Self {
		Self { container, operation: Operation::Exchange }
	}
}

impl Change for UpdateWallet {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let auto_exchange = character.persistent().settings.currency_auto_exchange;
		let inventory = &mut character.persistent_mut().inventory;
		let wallet = match &self.container {
			None => inventory.wallet_mut(),
			Some(item_ref) => {
				let Some(item) = inventory.get_mut_at_path(&item_ref.path) else { return };
				let Some(container) = &mut item.items else { return };
				container.wallet_mut()
			}
		};
		match &self.operation {
			Operation::Add(adjustment) => {
				*wallet += *adjustment;
			}
			Operation::Remove(adjustment) => {
				if wallet.contains(adjustment, auto_exchange) {
					wallet.remove(*adjustment, auto_exchange);
				}
			}
			Operation::Exchange => {
				wallet.normalize();
			}
		}
	}
}

impl FromKdl<NodeContext> for UpdateWallet {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let container = node.query_opt_t("scope() > container")?;
		match node.next_str_req()? {
			"add" => {
				let wallet = node.query_req_t("scope() > wallet")?;
				Ok(Self::add(wallet, container))
			}
			"remove" => {
				let wallet = node.query_req_t("scope() > wallet")?;
				Ok(Self::remove(wallet, container))
			}
			"exchange" => Ok(Self::exchange(container)),
			id => Err(NotInList(id.to_owned(), vec!["add", "remove", "exchange"]).into()),
		}
	}
}

impl AsKdl for UpdateWallet {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match &self.operation {
			Operation::Add(wallet) => {
				node.entry("add");
				node.child(("wallet", wallet));
			}
			Operation::Remove(wallet) => {
				node.entry("remove");
				node.child(("wallet", wallet));
			}
			Operation::Exchange => {
				node.entry("exchange");
			}
		}
		node.child(("container", &self.container, OmitIfEmpty));
		node
	}
}
