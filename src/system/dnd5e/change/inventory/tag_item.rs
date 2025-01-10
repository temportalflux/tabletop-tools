use super::ItemRef;
use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyItemUserTag {
	pub item: ItemRef,
	pub tag: String,
	pub should_be_applied: bool,
}

crate::impl_trait_eq!(ApplyItemUserTag);
kdlize::impl_kdl_node!(ApplyItemUserTag, "item_apply_user_tag");

impl Change for ApplyItemUserTag {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let inventory = &mut character.persistent_mut().inventory;
		let Some(item) = inventory.get_mut_at_path(&self.item.path) else { return };
		if self.should_be_applied == item.user_tags.contains(&self.tag) {
			return;
		}
		if self.should_be_applied {
			item.user_tags.push(self.tag.clone());
		} else {
			item.user_tags.retain(|item| item != &self.tag);
		}
	}
}

impl FromKdl<NodeContext> for ApplyItemUserTag {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let item = node.query_req_t("scope() > item")?;
		let tag = node.next_str_req()?.to_owned();
		let should_be_applied = node.get_bool_req("applied")?;
		Ok(Self { item, tag, should_be_applied })
	}
}

impl AsKdl for ApplyItemUserTag {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("item", &self.item));
		node.entry(self.tag.as_str());
		node.entry(("applied", self.should_be_applied));
		node
	}
}
