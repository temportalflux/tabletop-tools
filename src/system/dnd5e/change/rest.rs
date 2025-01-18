use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::{Character, Persistent, MAX_SPELL_RANK},
			roll::{Die, RollSet},
			Ability, Condition, HitPoint, Rest,
		},
		Change, SourceId,
	},
	utility::NotInList,
};
use enum_map::EnumMap;
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use multimap::MultiMap;
use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyRest {
	pub rest: Rest,
	pub additional_effects: MultiMap<Option<PathBuf>, ApplyRestEffect>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyRestEffect {
	// Restores hit-points by consuming hit-dice, and increasing current-hp by the rolled amount.
	UseHitDice(EnumMap<Die, u32>, u32),
	// Restores spell slots that were used.
	// None => clear all used spell slots (so all will be available)
	// [#: None] => clear all usages of a rank of spell slot
	// [#: #] => restore some number of slots in a given rank
	GrantSpellSlots(Option<BTreeMap<u8, Option<u32>>>),
	// Restores all or some number of uses to a selection value (resource).
	RestoreResourceUses { data_path: PathBuf, amount: Option<u32> },
	// Adds a condition to the character
	GrantCondition(Condition),
	// Removes a condition by its id, optionally providing a "degree"/level if there are multiple levels applied to the character.
	RemoveCondition(SourceId, Option<u32>),
}

crate::impl_trait_eq!(ApplyRest);
kdlize::impl_kdl_node!(ApplyRest, "rest");

impl From<Rest> for ApplyRest {
	fn from(rest: Rest) -> Self {
		Self { rest, additional_effects: MultiMap::default() }
	}
}

impl ApplyRest {
	pub fn push_effect(&mut self, source: Option<&Path>, effect: ApplyRestEffect) {
		self.additional_effects.insert(source.map(Path::to_owned), effect);
	}
}

impl Change for ApplyRest {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let max_hp = character.get_hp(HitPoint::Max);
		match self.rest {
			Rest::Short => {}
			Rest::Long => {
				character.hit_points_mut().current = max_hp;
				character.hit_points_mut().set_temp_hp(0);
				character.hit_points_mut().saves = EnumMap::default();

				let hit_die_paths = {
					let iter = character.persistent().hit_points.hit_dice_selectors.iter();
					let iter = iter.filter_map(|(_die, selector)| selector.get_data_path());
					iter.collect::<Vec<_>>()
				};
				for data_path in hit_die_paths {
					character.persistent_mut().set_selected(data_path, None);
				}
			}
		}

		let make_gained_uses = |persistent: &Persistent, data_path: &Path, gained_uses: &Option<u32>| match gained_uses
		{
			None => None,
			Some(gained_uses) => {
				let prev_value = persistent.get_first_selection_at::<u32>(data_path);
				let consumed_uses = prev_value.map(Result::ok).flatten().unwrap_or(0);
				let new_value = consumed_uses.saturating_sub(*gained_uses);
				(new_value > 0).then(|| new_value.to_string())
			}
		};

		for (_source, effects) in self.additional_effects.iter_all() {
			'effects: for effect in effects {
				match effect {
					ApplyRestEffect::UseHitDice(hit_dice, rolled_hp) => {
						let constitution_mod = character.ability_modifier(Ability::Constitution, None);
						let constitution_mod = constitution_mod.max(0) as u32;
						let roll_count = {
							let iter = hit_dice.iter();
							let iter = iter.map(|(_die, amount)| *amount);
							iter.filter(|amount| *amount > 0).sum::<u32>()
						};
						let hp_gained = *rolled_hp + roll_count * constitution_mod;

						// Preprocessing hit dice to ensure that all dice must resolve to valid data paths,
						// else the effect will be discarded.
						let mut hit_dice_selections = Vec::with_capacity(hit_dice.len());
						for (die, amount_used) in hit_dice {
							let selector = &character.hit_points().hit_dice_selectors[die];
							let Some(data_path) = selector.get_data_path() else { continue 'effects };

							let prev_value = character.persistent().get_first_selection_at::<u32>(&data_path);
							let prev_value = prev_value.map(Result::ok).flatten().unwrap_or(0);
							let new_value = prev_value.saturating_add(*amount_used).to_string();

							hit_dice_selections.push((data_path, new_value));
						}

						// Consume the hit dice
						for (data_path, new_value) in hit_dice_selections {
							character.persistent_mut().set_selected(data_path, Some(new_value));
						}
						// Apply the gained hit points
						let current_hp = &mut character.hit_points_mut().current;
						*current_hp = (*current_hp + hp_gained).min(max_hp);
					}
					ApplyRestEffect::GrantSpellSlots(rank_amounts) => {
						let rank_range = match rank_amounts {
							None => (1..=MAX_SPELL_RANK).into_iter().map(|rank| (rank, None)).collect::<Vec<_>>(),
							Some(rank_amounts) => {
								rank_amounts.iter().map(|(rank, amount)| (*rank, amount.clone())).collect::<Vec<_>>()
							}
						};
						for (rank, gained_slots) in rank_range {
							let data_path = {
								let selected_spells = &character.persistent().selected_spells;
								selected_spells.consumed_slots_path(rank)
							};
							let new_value = make_gained_uses(&character.persistent(), &data_path, &gained_slots);
							character.persistent_mut().set_selected(data_path, new_value);
						}
					}
					ApplyRestEffect::RestoreResourceUses { data_path, amount } => {
						let new_value = make_gained_uses(&character.persistent(), &data_path, amount);
						character.persistent_mut().set_selected(data_path, new_value);
					}
					ApplyRestEffect::GrantCondition(condition) => {
						let conditions = &mut character.persistent_mut().conditions;
						if let Some(condition_id) = &condition.id {
							for existing_condition in conditions.iter_mut() {
								let Some(existing_id) = &existing_condition.id else { continue };
								if existing_id.unversioned() == condition_id.unversioned() {
									// TODO: actually cannonicalize condition levels!!!
									// we have concatenated the incoming condition with the existing one,
									// so we are done processing this effect
									//continue 'effects;
								}
							}
						}
						conditions.insert(condition.clone());
					}
					ApplyRestEffect::RemoveCondition(condition_id, _levels) => {
						let conditions = &mut character.persistent_mut().conditions;
						conditions.remove_by_id(condition_id);
					}
				}
			}
		}
	}
}

impl FromKdl<NodeContext> for ApplyRest {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rest = node.next_str_req_t()?;
		let mut additional_effects = MultiMap::default();
		for mut node in node.query_all("scope() > group")? {
			let key = node.next_str_opt_t()?;
			let values = node.query_all_t("scope() > effect")?;
			additional_effects.insert_many(key, values);
		}
		Ok(Self { rest, additional_effects })
	}
}

impl AsKdl for ApplyRest {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.rest.to_string());
		for (source, effects) in &self.additional_effects {
			let mut group = NodeBuilder::default();
			group.entry(source.as_ref().map(|path| path.to_str().unwrap()));
			for effect in effects {
				group.child(("effect", effect));
			}
			node.child(("group", group));
		}
		node
	}
}

impl FromKdl<NodeContext> for ApplyRestEffect {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"UseHitDice" => {
				let dice = node.next_str_req_t::<RollSet>()?.into();
				let rolled_hp = node.next_i64_req()? as u32;
				Ok(Self::UseHitDice(dice, rolled_hp))
			}
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
				let data_path = node.next_str_req_t()?;
				let amount = node.next_i64_opt()?.map(|v| v as u32);
				Ok(Self::RestoreResourceUses { amount, data_path })
			}
			"GrantCondition" => Ok(Self::GrantCondition(Condition::from_kdl(node)?)),
			"RemoveCondition" => {
				let condition_id = node.next_str_req_t()?;
				let level = node.next_i64_opt()?.map(|v| v as u32);
				Ok(Self::RemoveCondition(condition_id, level))
			}
			id => Err(NotInList(id.to_owned(), vec![
				"UseHitDice",
				"GrantSpellSlots",
				"RestoreResourceUses",
				"GrantCondition",
				"RemoveCondition",
			])
			.into()),
		}
	}
}

impl AsKdl for ApplyRestEffect {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::UseHitDice(dice, rolled_hp) => {
				node.entry("UseHitDice");
				node.entry(RollSet::from(dice).to_string());
				node.entry(*rolled_hp as i64);
			}
			Self::GrantSpellSlots(rank_amounts) => {
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
			Self::RestoreResourceUses { data_path, amount } => {
				node.entry("RestoreResourceUses");
				node.entry(data_path.to_str().unwrap());
				node.entry(amount.as_ref().map(|v| *v as i64));
			}
			Self::GrantCondition(condition) => {
				node.entry("GrantCondition");
				node += condition.as_kdl();
			}
			Self::RemoveCondition(condition_id, level) => {
				node.entry("RemoveCondition");
				node.entry(condition_id.to_string());
				node.entry(level.as_ref().map(|v| *v as i64));
			}
		}
		node
	}
}
