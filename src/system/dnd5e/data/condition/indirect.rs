use super::Condition;
use crate::{kdl_ext::NodeContext, system::SourceId};
use anyhow::Context;
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::str::FromStr;

pub type IndirectCondition = Indirect<Condition>;

#[derive(Clone, PartialEq, Debug)]
pub enum Indirect<V> {
	Id(SourceId),
	Object(V),
}

impl<V> FromKdl<NodeContext> for Indirect<V>
where
	V: FromKdl<NodeContext>,
	anyhow::Error: From<V::Error>,
{
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"Custom" | "Specific" => {
				// this is a custom condition node, parse it as a condition struct
				let condition = V::from_kdl(node)?;
				Ok(Self::Object(condition))
			}
			source_id_str => {
				let mut source_id = SourceId::from_str(source_id_str).with_context(|| {
					format!(
						"Expected {source_id_str:?} to either be the value \"Custom\"/\"Specific\" or a valid SourceId."
					)
				})?;
				source_id.set_relative_basis(node.context().id(), false);
				Ok(Self::Id(source_id))
			}
		}
	}
}

impl<V: AsKdl> AsKdl for Indirect<V> {
	fn as_kdl(&self) -> NodeBuilder {
		let node = NodeBuilder::default();
		match self {
			Self::Id(id) => node.with_entry(id.to_string()),
			Self::Object(value) => {
				let mut node = node.with_entry("Specific");
				node += value.as_kdl();
				node
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::kdl_ext::test_utils::*;

		static NODE_NAME: &str = "condition";

		#[test]
		fn id() -> anyhow::Result<()> {
			let doc = "condition \"condition/invisible.kdl\"";
			let data = IndirectCondition::Id(SourceId { path: "condition/invisible.kdl".into(), ..Default::default() });
			assert_eq_fromkdl!(IndirectCondition, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn custom() -> anyhow::Result<()> {
			let doc = "condition \"Specific\" name=\"Slippery\"";
			let data = IndirectCondition::Object(Condition { name: "Slippery".into(), ..Default::default() });
			assert_eq_fromkdl!(IndirectCondition, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
