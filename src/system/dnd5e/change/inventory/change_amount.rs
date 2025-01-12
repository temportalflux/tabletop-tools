use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, item},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ChangeItemAmount {
	pub item: ItemRef,
	pub amount: u32,
}

crate::impl_trait_eq!(ChangeItemAmount);
kdlize::impl_kdl_node!(ChangeItemAmount, "item_amount");

impl Change for ChangeItemAmount {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let Some(item) = character.inventory_mut().get_mut_at_path(&self.item.path) else { return };
		let item::Kind::Simple { count } = &mut item.kind else { return };
		*count = self.amount;
	}
}

impl FromKdl<NodeContext> for ChangeItemAmount {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let amount = node.next_i64_req()? as u32;
		let item = node.query_req_t("scope() > item")?;
		Ok(Self { item, amount })
	}
}

impl AsKdl for ChangeItemAmount {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.amount as i64);
		node.child(("item", &self.item));
		node
	}
}
