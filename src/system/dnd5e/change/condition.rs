use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, Condition},
		Change, SourceId,
	},
	utility::NotInList,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::{path::PathBuf, str::FromStr};

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyCondition {
	Add(Condition),
	RemoveCustom(usize),
	RemoveId(SourceId),
	Append(Vec<Condition>, /*feature path*/ PathBuf),
}

crate::impl_trait_eq!(ApplyCondition);
kdlize::impl_kdl_node!(ApplyCondition, "apply_condition");

impl Change for ApplyCondition {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::Add(condition) => {
				character.persistent_mut().conditions.insert(condition.clone());
			}
			Self::RemoveId(id) => {
				character.persistent_mut().conditions.remove_by_id(id);
			}
			Self::RemoveCustom(idx) => {
				character.persistent_mut().conditions.remove_custom(*idx);
			}
			Self::Append(conditions, _feature_path) => {
				for condition in conditions {
					character.persistent_mut().conditions.insert(condition.clone());
				}
			}
		}
		// So that the contents of any added or removed condition is recompiled
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for ApplyCondition {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"add" => Ok(Self::Add(Condition::from_kdl(node)?)),
			"remove" => {
				let entry = node.next_req()?;
				if let Some(value) = entry.value().as_i64() {
					Ok(Self::RemoveCustom(value as usize))
				} else if let Some(value) = entry.value().as_string() {
					Ok(Self::RemoveId(SourceId::from_str(value)?))
				} else {
					Err(anyhow::Error::msg(format!(
						"Invalid apply_condition value, expected number or string: {}",
						entry.value()
					)))
				}
			}
			"append" => {
				let feature_path = node.next_str_req_t()?;
				let conditions = node.query_all_t("scope() > condition")?;
				Ok(Self::Append(conditions, feature_path))
			}
			s => Err(NotInList(s.to_owned(), vec!["add", "remove", "append"]).into()),
		}
	}
}

impl AsKdl for ApplyCondition {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Add(condition) => {
				node.entry("add");
				node += condition.as_kdl();
			}
			Self::RemoveId(id) => {
				node.entry("remove");
				node.entry(id.to_string());
			}
			Self::RemoveCustom(idx) => {
				node.entry("remove");
				node.entry(*idx as i64);
			}
			Self::Append(conditions, feature_path) => {
				node.entry("append");
				node.entry(feature_path.to_str().unwrap());
				node.children(("condition", conditions.iter()));
			}
		}
		node
	}
}
