use crate::{kdl_ext::NodeContext, system::dnd5e::data::item::container::item::ItemPath};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct ItemRef {
	pub path: ItemPath,
	pub name: Vec<String>,
}

impl FromKdl<NodeContext> for ItemRef {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let path = node.next_str_req_t()?;
		let name = {
			let iter = node.next_str_req()?.split("/");
			let iter = iter.map(str::to_owned);
			iter.collect()
		};
		Ok(Self { path, name })
	}
}

impl AsKdl for ItemRef {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default().with_entry(self.path.to_string()).with_entry(self.name.join("/"))
	}
}
