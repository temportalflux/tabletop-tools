use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::{Character, RestEffect, RestEntry},
			description,
			roll::RollSet,
			Condition, Indirect, Rest,
		},
		mutator::{Group, ReferencePath},
		Mutator,
	},
	utility::{selector::IdPath, NotInList},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::collections::BTreeMap;

// Provides a way to change the character when rests are taken.
#[derive(Clone, Debug, PartialEq)]
pub struct ApplyWhenRest {
	rest: Rest,
	effects: Vec<RestMutatorEffect>,
}

#[derive(Clone, Debug, PartialEq)]
enum RestMutatorEffect {
	GrantSpellSlots(Option<BTreeMap<u8, Option<u32>>>),
	RestoreResourceUses { amount: u32, resource: IdPath },
	GrantCondition(Indirect<Condition>),
}

crate::impl_trait_eq!(ApplyWhenRest);
kdlize::impl_kdl_node!(ApplyWhenRest, "rest");

impl Mutator for ApplyWhenRest {
	type Target = Character;

	fn description(&self, _state: Option<&Character>) -> description::Section {
		description::Section { ..Default::default() }
	}

	fn set_data_path(&self, parent: &ReferencePath) {
		for effect in &self.effects {
			match effect {
				RestMutatorEffect::RestoreResourceUses { resource, .. } => {
					resource.set_path(parent);
				}
				RestMutatorEffect::GrantCondition(Indirect::Custom(condition)) => {
					condition.set_data_path(parent);
				}
				_ => {}
			}
		}
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		let mut entry = RestEntry { source: parent.display.clone(), effects: Vec::with_capacity(self.effects.len()) };
		for effect in &self.effects {
			entry.effects.push(match effect {
				RestMutatorEffect::GrantSpellSlots(rank_amounts) => RestEffect::GrantSpellSlots(rank_amounts.clone()),
				RestMutatorEffect::RestoreResourceUses { amount, resource } => {
					let Some(data_path) = resource.data() else { return };
					RestEffect::RestoreResourceUses { amount: Some(RollSet::from(*amount as i32)), data_path }
				}
				RestMutatorEffect::GrantCondition(indirect) => RestEffect::GrantCondition(indirect.clone()),
			});
		}
		stats.rest_resets_mut().add(self.rest, entry);
	}
}

impl FromKdl<NodeContext> for ApplyWhenRest {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rest = node.next_str_req_t()?;
		let effects = node.query_all_t("scope() > effect")?;
		Ok(Self { rest, effects })
	}
}

impl FromKdl<NodeContext> for RestMutatorEffect {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"GrantSpellSlots" => {
				let mut rank_amounts = BTreeMap::default();
				for mut node in node.query_all("scope() > rank")? {
					let rank = node.next_i64_req()? as u8;
					let amount: Option<u32> = node.next_i64_opt()?.map(|v| v as u32);
					rank_amounts.insert(rank, amount);
				}
				Ok(Self::GrantSpellSlots((!rank_amounts.is_empty()).then_some(rank_amounts)))
			}
			"RestoreResourceUses" => {
				let resource = IdPath::from(node.next_str_req()?.to_owned() + "/uses");
				let amount = node.next_i64_req()? as u32;
				Ok(Self::RestoreResourceUses { amount, resource })
			}
			"GrantCondition" => Ok(Self::GrantCondition(Indirect::from_kdl(node)?)),
			id => {
				Err(NotInList(id.to_owned(), vec!["GrantSpellSlots", "RestoreResourceUses", "GrantCondition"]).into())
			}
		}
	}
}

impl AsKdl for ApplyWhenRest {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.rest.to_string());
		node.children(("effect", &self.effects));
		node
	}
}

impl AsKdl for RestMutatorEffect {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			RestMutatorEffect::GrantSpellSlots(rank_amounts) => {
				node.entry("GrantSpellSlots");
				match rank_amounts {
					None => {}
					Some(rank_amounts) => {
						for (rank, amount) in rank_amounts {
							node.child(("rank", {
								let mut node = kdlize::NodeBuilder::default();
								node.entry(*rank as i64);
								node.entry(amount.as_ref().map(|v| *v as i64));
								node
							}));
						}
					}
				}
			}
			RestMutatorEffect::RestoreResourceUses { amount, resource } => {
				node.entry("RestoreResourceUses");
				node.entry(resource.get_id().map(std::borrow::Cow::into_owned).unwrap_or_default());
				node.entry(*amount as i64);
			}
			RestMutatorEffect::GrantCondition(indirect) => {
				node.entry("GrantCondition");
				node += indirect.as_kdl();
			}
		}
		node
	}
}
