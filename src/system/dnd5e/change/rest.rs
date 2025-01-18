use std::path::Path;
use crate::{
	kdl_ext::NodeContext, path_map::PathMap, system::{
		change, dnd5e::data::{
			character::{Character, RestEffect}, DeathSave, HitPoint, Rest
		}, Change
	}
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use super::hit_points::{DeathSaves, HealOrDamage, TempHP};

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyRest {
	rest: Rest,
	changes: PathMap<change::Generic<Character>>,
}

crate::impl_trait_eq!(ApplyRest);
kdlize::impl_kdl_node!(ApplyRest, "rest");

impl From<Rest> for ApplyRest {
	fn from(rest: Rest) -> Self {
		Self { rest, changes: PathMap::default() }
	}
}

impl ApplyRest {
	pub fn push_effect<P: AsRef<Path>>(&mut self, source: Option<P>, effect: &RestEffect, character: &Character) {
		match effect {
			RestEffect::RestoreCurrentHP => {
				let current = character.get_hp(HitPoint::Current);
				let max_hp = character.get_hp(HitPoint::Max);
				let temp = character.get_hp(HitPoint::Temp);
				let delta = (max_hp as i32) - (current as i32);
				self.changes.push(HealOrDamage {
					delta, current: max_hp, temp, clear_saves: true
				}.into());
			}
			RestEffect::ClearTempHP => {
				self.changes.push(TempHP(0).into());
			}
			RestEffect::GrantTempHP(rolls) => {}
			RestEffect::ClearDeathSaves => {
				let successes = character.hit_points().saves[DeathSave::Success] as i8;
				let failures = character.hit_points().saves[DeathSave::Failure] as i8;
				self.changes.push(DeathSaves { save: DeathSave::Success, delta: -successes, value: 0 }.into());
				self.changes.push(DeathSaves { save: DeathSave::Failure, delta: -failures, value: 0 }.into());
			}
			RestEffect::RestoreHitDice(hit_dice) => {}
			RestEffect::UseHitDice(hit_dice) => {}
			RestEffect::GrantSpellSlots(rank_amounts) => {}
			RestEffect::RestoreResourceUses { .. } => {}
			RestEffect::GrantCondition(condition) => {}
		}
	}
}

impl Change for ApplyRest {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {

		// RestoreHitDice
		let hit_die_paths = {
			let iter = persistent.hit_points.hit_dice_selectors.iter();
			let iter = iter.filter_map(|(_die, selector)| selector.get_data_path());
			iter.collect::<Vec<_>>()
		};
		for data_path in hit_die_paths {
			persistent.set_selected(data_path, None);
		}
		
	}
}

impl FromKdl<NodeContext> for ApplyRest {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rest = node.next_str_req_t()?;
		let changes = node.query_all_t("scope() > change")?;
		Ok(Self { rest, changes })
	}
}

impl AsKdl for ApplyRest {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.rest.to_string());
		node.children(("change", &self.changes));
		node
	}
}
