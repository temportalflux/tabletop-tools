use crate::utility::NotInList;
use kdlize::NodeBuilder;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Settings {
	pub currency_auto_exchange: bool,
}

impl Settings {
	pub fn insert_from_kdl<'doc>(&mut self, node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<()> {
		match node.next_str_req()? {
			"currency_auto_exchange" => {
				self.currency_auto_exchange = node.next_bool_req()?;
			}
			key => {
				return Err(NotInList(key.into(), vec!["currency_auto_exchange"]).into());
			}
		}
		Ok(())
	}

	pub fn export_as_kdl(&self, nodes: &mut NodeBuilder) {
		nodes.child(
			NodeBuilder::default()
				.with_entry("currency_auto_exchange")
				.with_entry(self.currency_auto_exchange)
				.build("setting"),
		);
	}
}
