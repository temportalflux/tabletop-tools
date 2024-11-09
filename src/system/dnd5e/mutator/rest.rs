use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::{Character, RestEntry}, description, roll::RollSet, Rest},
		mutator::ReferencePath,
		Mutator,
	}, utility::{selector::IdPath, NotInList},
};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder};

// Provides a way to change the character when rests are taken.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyWhenRest {
	rest: Rest,
	effect: RestEffect,
}

#[derive(Clone, Debug, PartialEq)]
enum RestEffect {
	GrantUses {
		amount: u32,
		resource: IdPath,
	},
}

crate::impl_trait_eq!(ApplyWhenRest);
kdlize::impl_kdl_node!(ApplyWhenRest, "rest");

impl Mutator for ApplyWhenRest {
	type Target = Character;

	fn description(&self, _state: Option<&Character>) -> description::Section {
		description::Section { ..Default::default() }
	}

	fn set_data_path(&self, parent: &ReferencePath) {
		match &self.effect {
			RestEffect::GrantUses { resource, .. } => {
				resource.set_path(parent);
			}
		}
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		match &self.effect {
			RestEffect::GrantUses { amount, resource } => {
				let Some(data_path) = resource.data() else { return };
				stats.rest_resets_mut().add(self.rest, RestEntry {
					restore_amount: Some(RollSet::from(*amount as i32)),
					data_paths: vec![data_path],
					source: parent.display.clone(),
				});
			}
		}
	}
}

impl FromKdl<NodeContext> for ApplyWhenRest {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rest = node.next_str_req_t()?;
		let effect = RestEffect::from_kdl(node)?;
		Ok(Self { rest, effect })
	}
}

impl FromKdl<NodeContext> for RestEffect {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"GrantUses" => {
				let amount = node.query_i64_req("scope() > amount", 0)? as u32;
				let resource = IdPath::from(node.query_str_req("scope() > resource", 0)?);
				Ok(Self::GrantUses { amount, resource })
			}
			name => Err(NotInList(name.to_owned(), vec!["GrantUses"]).into()),
		}
	}
}

impl AsKdl for ApplyWhenRest {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.rest.to_string());
		node += self.effect.as_kdl();
		node
	}
}

impl AsKdl for RestEffect {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			RestEffect::GrantUses { amount, resource } => {
				node.entry("GrantUses");
				node.child(("amount", *amount));
				let resource = resource.get_id().map(std::borrow::Cow::into_owned).unwrap_or_default();
				node.child(("resource", resource));
			}
		}
		node
	}
}
