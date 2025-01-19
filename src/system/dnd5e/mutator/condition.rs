use crate::{kdl_ext::{NodeContext, NodeReader}, system::{dnd5e::data::{character::Character, Condition, Indirect}, mutator::{Group, ReferencePath}, Mutator}};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct GrantCondition(pub Indirect<Condition>);

crate::impl_trait_eq!(GrantCondition);
kdlize::impl_kdl_node!(GrantCondition, "grant_condition");

impl Mutator for GrantCondition {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		if let Indirect::Custom(condition) = &self.0 {
			condition.set_data_path(parent);
		}
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		// TODO: This will need to be resolved when its added to the character
		let Indirect::Custom(condition) = &self.0 else { return };
		stats.derived_conditions_mut().push((condition.clone(), parent.display.clone()));
	}
}

impl FromKdl<NodeContext> for GrantCondition {
	type Error = anyhow::Error;
	fn from_kdl(node: &mut NodeReader) -> anyhow::Result<Self> {
		Ok(Self(Indirect::from_kdl(node)?))
	}
}

impl AsKdl for GrantCondition {
	fn as_kdl(&self) -> NodeBuilder {
		self.0.as_kdl()
	}
}
