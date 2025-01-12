use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
	utility::NotInList,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateSettings {
	AutoExchange(bool),
}

crate::impl_trait_eq!(UpdateSettings);
kdlize::impl_kdl_node!(UpdateSettings, "settings");

impl Change for UpdateSettings {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::AutoExchange(enabled) => {
				character.persistent_mut().settings.currency_auto_exchange = *enabled;
			}
		}
	}
}

impl FromKdl<NodeContext> for UpdateSettings {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"autoexchange" => Ok(Self::AutoExchange(node.next_bool_req()?)),
			id => Err(NotInList(id.to_owned(), vec!["autoexchange"]).into()),
		}
	}
}

impl AsKdl for UpdateSettings {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::AutoExchange(enabled) => {
				node.entry("autoexchange");
				node.entry(*enabled);
			}
		}
		node
	}
}
