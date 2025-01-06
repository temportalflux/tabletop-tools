use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, Ability},
		Change,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyAbilityScores(pub BTreeMap<Ability, u32>);

crate::impl_trait_eq!(ApplyAbilityScores);
kdlize::impl_kdl_node!(ApplyAbilityScores, "ability_scores");

impl Change for ApplyAbilityScores {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		for (ability, value) in &self.0 {
			character.persistent_mut().ability_scores[*ability] = *value;
		}
		character.ability_scores_mut().finalize();
	}
}

impl FromKdl<NodeContext> for ApplyAbilityScores {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let mut scores = BTreeMap::new();
		for mut node in node.query_all("scope() > score")? {
			let ability = node.next_str_req_t()?;
			let value = node.next_i64_req()? as u32;
			scores.insert(ability, value);
		}
		Ok(Self(scores))
	}
}

impl AsKdl for ApplyAbilityScores {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		for (ability, value) in &self.0 {
			node.child(("score", { NodeBuilder::default().with_entry(ability.to_string()).with_entry(*value as i64) }));
		}
		node
	}
}
