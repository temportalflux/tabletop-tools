use std::str::FromStr;

use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::{
			data::{
				action::AttackQuery,
				character::{spellcasting, Character, SpellHealingBonus},
				description,
				roll::{EvaluatedRoll, Modifier, RollSet},
				Ability, DamageType, Skill,
			},
			Value,
		},
		mutator::ReferencePath,
		Mutator,
	},
	utility::{selector, Dependencies, NotInList},
};
use kdlize::{
	ext::{DocumentExt, EntryExt, ValueExt},
	AsKdl, FromKdl, NodeBuilder,
};
use num_traits::Signed;

#[derive(Clone, PartialEq, Debug)]
pub enum Modify {
	Ability {
		ability: Option<selector::Value<Character, Ability>>,
		modifier: Option<Modifier>,
		bonus: Option<i64>,
		context: Option<String>,
	},
	SavingThrow {
		ability: Option<selector::Value<Character, Ability>>,
		modifier: Option<Modifier>,
		bonus: Option<i64>,
		context: Option<String>,
	},
	Skill {
		skill: selector::Value<Character, Skill>,
		modifier: Option<Modifier>,
		bonus: Option<i64>,
		context: Option<String>,
	},
	Initiative {
		modifier: Option<Modifier>,
		bonus: Option<i64>,
		context: Option<String>,
	},
	ArmorClass {
		bonus: i32,
		context: Option<String>,
	},
	AttackRoll {
		bonus: Option<i64>,
		modifier: Option<Modifier>,
		ability: Option<Ability>,
		query: Vec<AttackQuery>,
	},
	AttackDamage {
		damage: EvaluatedRoll,
		damage_type: Option<DamageType>,
		query: Vec<AttackQuery>,
	},
	SpellDamage {
		damage: EvaluatedRoll,
		query: Vec<spellcasting::Filter>,
	},
	SpellRange {
		distance: Value<i32>,
		query: Vec<spellcasting::Filter>,
	},
	SpellHealing {
		amounts: Vec<SpellHealingBonus>,
		query: Vec<spellcasting::Filter>,
	},
}

crate::impl_trait_eq!(Modify);
kdlize::impl_kdl_node!(Modify, "modify");

impl Mutator for Modify {
	type Target = Character;

	fn description(&self, state: Option<&Character>) -> description::Section {
		match self {
			Self::Ability { ability, modifier, bonus, context } => {
				let mut mods = Vec::with_capacity(2);
				if let Some(modifier) = modifier {
					mods.push(modifier.display_name().to_owned());
				}
				if let Some(bonus) = bonus {
					mods.push(format!("{}{}", if *bonus >= 0 { "+" } else { "-" }, bonus.abs()));
				}

				let mut desc = format!("You have {} on ", mods.join(" & "));
				desc.push_str(&match ability {
					None => format!("ability checks"),
					Some(selector::Value::Specific(ability)) => format!("{} checks", ability.long_name()),
					Some(selector::Value::Options(selector::ValueOptions { options, .. })) => {
						format!(
							"any single ability check{}",
							(!options.is_empty())
								.then(|| {
									format!(
										" (of: {})",
										options.iter().map(Ability::long_name).collect::<Vec<_>>().join(", ")
									)
								})
								.unwrap_or_default()
						)
					}
				});
				if let Some(ctx) = &context {
					desc.push(' ');
					desc.push_str(ctx.as_str());
				}
				desc.push('.');

				let selectors = match ability {
					None => Default::default(),
					Some(selector) => selector::DataList::default().with_enum("Ability", selector, state),
				};

				description::Section { content: desc.into(), children: vec![selectors.into()], ..Default::default() }
			}
			Self::SavingThrow { ability, modifier, bonus, context } => {
				let mut mods = Vec::with_capacity(2);
				if let Some(modifier) = modifier {
					mods.push(modifier.display_name().to_owned());
				}
				if let Some(bonus) = bonus {
					mods.push(format!("{}{}", if *bonus >= 0 { "+" } else { "-" }, bonus.abs()));
				}

				let mut desc = format!("You have {} on ", mods.join(" & "));
				desc.push_str(&match ability {
					None => format!("saving throws"),
					Some(selector::Value::Specific(ability)) => format!("{} saving throws", ability.long_name()),
					Some(selector::Value::Options(selector::ValueOptions { options, .. })) => {
						format!(
							"any single saving throw{}",
							(!options.is_empty())
								.then(|| {
									format!(
										" (of: {})",
										options.iter().map(Ability::long_name).collect::<Vec<_>>().join(", ")
									)
								})
								.unwrap_or_default()
						)
					}
				});
				if let Some(ctx) = &context {
					desc.push_str(" against ");
					desc.push_str(ctx.as_str());
				}
				desc.push('.');

				let selectors = match ability {
					None => Default::default(),
					Some(selector) => selector::DataList::default().with_enum("Ability", selector, state),
				};

				description::Section { content: desc.into(), children: vec![selectors.into()], ..Default::default() }
			}
			Self::Skill { skill, modifier, bonus, context } => {
				let mut mods = Vec::with_capacity(2);
				if let Some(modifier) = modifier {
					mods.push(modifier.display_name().to_owned());
				}
				if let Some(bonus) = bonus {
					mods.push(format!("{}{}", if *bonus >= 0 { "+" } else { "-" }, bonus.abs()));
				}

				let mut desc = format!("You have {} on ", mods.join(" & "));
				desc.push_str(&match skill {
					selector::Value::Specific(skill) => {
						format!("{} ({}) checks", skill.ability().long_name(), skill.display_name())
					}
					selector::Value::Options(selector::ValueOptions { options, .. }) => {
						format!(
							"any single ability skill check{}",
							(!options.is_empty())
								.then(|| {
									format!(
										" (of: {})",
										options.iter().map(Skill::display_name).collect::<Vec<_>>().join(", ")
									)
								})
								.unwrap_or_default()
						)
					}
				});
				if let Some(ctx) = &context {
					desc.push_str(" ");
					desc.push_str(ctx.as_str());
				}
				desc.push('.');

				let selectors = selector::DataList::default().with_enum("Skill", skill, state);

				description::Section { content: desc.into(), children: vec![selectors.into()], ..Default::default() }
			}
			Self::Initiative { modifier, bonus, context } => {
				let mut mods = Vec::with_capacity(2);
				if let Some(modifier) = modifier {
					mods.push(modifier.display_name().to_owned());
				}
				if let Some(bonus) = bonus {
					mods.push(format!("{}{}", if *bonus >= 0 { "+" } else { "-" }, bonus.abs()));
				}
				let mut desc = format!("You have {} on initiative checks", mods.join(" & "));
				if let Some(ctx) = &context {
					desc.push_str(" ");
					desc.push_str(ctx.as_str());
				}
				desc.push('.');

				description::Section { content: desc.into(), ..Default::default() }
			}
			Self::ArmorClass { .. } => {
				description::Section { content: format!("TODO: modified armor class").into(), ..Default::default() }
			}
			Self::AttackRoll { .. } => {
				description::Section { content: format!("TODO: modified attack check").into(), ..Default::default() }
			}
			Self::AttackDamage { .. } => {
				description::Section { content: format!("TODO: modified attack damage").into(), ..Default::default() }
			}
			Self::SpellDamage { .. } => {
				description::Section { content: format!("TODO: modified spell damage").into(), ..Default::default() }
			}
			Self::SpellRange { .. } => {
				description::Section { content: format!("TODO: modified spell range").into(), ..Default::default() }
			}
			Self::SpellHealing { .. } => {
				description::Section { content: format!("TODO: modified spell healing").into(), ..Default::default() }
			}
		}
	}

	fn dependencies(&self) -> Dependencies {
		use kdlize::NodeId;
		let mut deps = Dependencies::from([super::AddFeature::id()]);
		match self {
			Self::Ability { .. } => {}
			Self::SavingThrow { .. } => {}
			Self::Skill { .. } => {}
			Self::Initiative { .. } => {}
			Self::ArmorClass { .. } => {}
			Self::AttackRoll { .. } => {}
			Self::AttackDamage { damage, .. } => {
				deps += damage.dependencies();
			}
			Self::SpellDamage { damage, .. } => {
				deps += damage.dependencies();
			}
			Self::SpellRange { distance, .. } => {
				deps += distance.dependencies();
			}
			Self::SpellHealing { .. } => {}
		}
		deps
	}

	fn set_data_path(&self, parent: &ReferencePath) {
		match self {
			Self::Ability { ability: Some(selector), .. } => {
				selector.set_data_path(parent);
			}
			Self::SavingThrow { ability: Some(selector), .. } => {
				selector.set_data_path(parent);
			}
			Self::Skill { skill: selector, .. } => {
				selector.set_data_path(parent);
			}
			Self::Ability { ability: None, .. } => {}
			Self::SavingThrow { ability: None, .. } => {}
			Self::Initiative { .. } => {}
			Self::ArmorClass { .. } => {}
			Self::AttackRoll { .. } => {}
			Self::AttackDamage { .. } => {}
			Self::SpellDamage { .. } => {}
			Self::SpellRange { .. } => {}
			Self::SpellHealing { .. } => {}
		}
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		match self {
			Self::Ability { ability, modifier, bonus, context } => {
				let ability = match ability {
					None => None,
					Some(ability) => stats.resolve_selector(ability),
				};
				for (_ability, entry) in stats.skills_mut().iter_ability_mut(ability) {
					if let Some(modifier) = modifier {
						entry.modifiers_mut().push(*modifier, context.clone(), parent.display.clone());
					}
					if let Some(bonus) = bonus {
						entry.bonuses_mut().push(*bonus, context.clone(), parent.display.clone());
					}
				}
			}
			Self::SavingThrow { ability, modifier, bonus, context } => {
				let ability = match ability {
					None => None,
					Some(ability) => stats.resolve_selector(ability),
				};
				if let Some(modifier) = modifier {
					stats.saving_throws_mut().add_modifier(ability, *modifier, context.clone(), parent);
				}
				if let Some(bonus) = bonus {
					stats.saving_throws_mut().add_bonus(ability, *bonus, context.clone(), parent);
				}
			}
			Self::Skill { skill, modifier, bonus, context } => {
				let Some(skill) = stats.resolve_selector(skill) else {
					return;
				};
				let entry = &mut stats.skills_mut()[skill];
				if let Some(modifier) = modifier {
					entry.modifiers_mut().push(*modifier, context.clone(), parent.display.clone());
				}
				if let Some(bonus) = bonus {
					entry.bonuses_mut().push(*bonus, context.clone(), parent.display.clone());
				}
			}
			Self::Initiative { modifier, bonus, context } => {
				if let Some(modifier) = modifier {
					stats.initiative_mut().modifiers_mut().push(*modifier, context.clone(), parent.display.clone());
				}
				if let Some(bonus) = bonus {
					stats.initiative_mut().bonuses_mut().push(*bonus, context.clone(), parent.display.clone());
				}
			}
			Self::ArmorClass { bonus, context } => {
				stats.armor_class_mut().push_bonus(*bonus, context.clone(), parent);
			}
			Self::AttackRoll { modifier, bonus, ability, query } => {
				if let Some(bonus) = bonus {
					stats.attack_bonuses_mut().add_to_weapon_attacks(*bonus as i32, query.clone(), parent);
				}
				if let Some(modifier) = modifier {
					stats.attack_bonuses_mut().modify_weapon_attacks(*modifier, query.clone(), parent);
				}
				if let Some(ability) = ability {
					stats.attack_bonuses_mut().add_ability_modifier(*ability, query.clone(), parent);
				}
			}
			Self::AttackDamage { damage, damage_type, query } => {
				let bonus = damage.evaluate(stats);
				stats.attack_bonuses_mut().add_to_weapon_damage(bonus, damage_type.clone(), query.clone(), parent);
			}
			Self::SpellDamage { damage, query } => {
				let bonus = damage.evaluate(stats);
				stats.attack_bonuses_mut().add_to_spell_damage(bonus, query.clone(), parent);
			}
			Self::SpellRange { distance, query } => {
				let distance = distance.evaluate(stats).max(0) as u32;
				stats.attack_bonuses_mut().modify_spell_range(distance, query.clone(), parent);
			}
			Self::SpellHealing { amounts, query } => {
				stats.attack_bonuses_mut().modify_spell_healing(amounts.clone(), query.clone(), parent);
			}
		}
	}
}

impl FromKdl<NodeContext> for Modify {
	type Error = anyhow::Error;

	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.peak_req()?.type_opt() {
			Some("Ability") => {
				let ability = match node.peak_str_req()? {
					"All" => {
						let _consume_all = node.next_req()?;
						None
					}
					_ => Some(selector::Value::from_kdl(node)?),
				};
				let modifier = node.get_str_opt_t::<Modifier>("modifier")?;
				let bonus = node.get_i64_opt("bonus")?;
				let context = node.get_str_opt("context")?.map(str::to_owned);
				Ok(Self::Ability { ability, modifier, bonus, context })
			}
			Some("SavingThrow") => {
				let ability = match node.peak_str_req()? {
					"All" => {
						let _consume_all = node.next_req()?;
						None
					}
					_ => Some(selector::Value::from_kdl(node)?),
				};
				let modifier = node.get_str_opt_t::<Modifier>("modifier")?;
				let bonus = node.get_i64_opt("bonus")?;
				let context = node.get_str_opt("context")?.map(str::to_owned);
				Ok(Self::SavingThrow { ability, modifier, bonus, context })
			}
			Some("Skill") => {
				let skill = selector::Value::from_kdl(node)?;
				let modifier = node.get_str_opt_t::<Modifier>("modifier")?;
				let bonus = node.get_i64_opt("bonus")?;
				let context = node.get_str_opt("context")?.map(str::to_owned);
				Ok(Self::Skill { skill, modifier, bonus, context })
			}
			Some("Attack") => match node.next_str_req()? {
				"Roll" => {
					let bonus = node.query_i64_opt("scope() > bonus", 0)?;
					// NOTE: This ability property will probably need to be upgraded to a selector
					let ability = node.query_str_opt_t("scope() > ability", 0)?;
					// NOTE: This modifier will probably need an optional context
					let modifier = node.query_str_opt_t("scope() > modifier", 0)?;
					let query = node.query_all_t("scope() > query")?;
					Ok(Self::AttackRoll { ability, modifier, bonus, query })
				}
				"Damage" => {
					let damage = node.query_req_t("scope() > damage")?;
					let damage_type = node.query_str_opt_t("scope() > damage_type", 0)?;
					let query = node.query_all_t("scope() > query")?;
					Ok(Self::AttackDamage { damage, damage_type, query })
				}
				s => Err(NotInList(s.into(), vec!["Roll", "Damage"]).into()),
			},
			Some("Spell") => match node.next_str_req()? {
				"Damage" => {
					let damage = node.query_req_t("scope() > damage")?;
					let query = node.query_all_t("scope() > query")?;
					Ok(Self::SpellDamage { damage, query })
				}
				"Range" => {
					let distance = node.query_req_t("scope() > distance")?;
					let query = node.query_all_t("scope() > query")?;
					Ok(Self::SpellRange { distance, query })
				}
				"Healing" => {
					let amounts = node.query_all_t("scope() > amount")?;
					let query = node.query_all_t("scope() > query")?;
					Ok(Self::SpellHealing { amounts, query })
				}
				s => Err(NotInList(s.into(), vec!["Damage", "Range", "Healing"]).into()),
			},
			None => match node.next_str_req()? {
				"Initiative" => {
					let modifier = node.get_str_opt_t::<Modifier>("modifier")?;
					let bonus = node.get_i64_opt("bonus")?;
					let context = node.get_str_opt("context")?.map(str::to_owned);
					Ok(Self::Initiative { modifier, bonus, context })
				}
				"ArmorClass" => {
					let bonus = node.next_i64_req()? as i32;
					let context = node.get_str_opt("context")?.map(str::to_owned);
					Ok(Self::ArmorClass { bonus, context })
				}
				s => Err(NotInList(s.into(), vec!["Initiative", "ArmorClass"]).into()),
			},
			type_id => Err(NotInList(
				format!("{}{}", type_id.map(|id| format!("({id})")).unwrap_or_default(), node.peak_str_req()?),
				vec![
					"(Ability)*",
					"(SavingThrow)*",
					"(Skill)*",
					"Initiative",
					"(Attack)Roll",
					"(Attack)Damage",
					"(Spell)Damage",
					"(Spell)Range",
					"(Spell)Healing",
					"ArmorClass",
				],
			)
			.into()),
		}
	}
}

impl AsKdl for Modify {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Ability { ability, modifier, bonus, context } => {
				match ability {
					None => {
						node.entry_typed("Ability", "All");
					}
					Some(ability) => {
						node += ("Ability", ability.as_kdl());
					}
				}
				node.entry(("modifier", modifier.as_ref().map(Modifier::to_string)));
				node.entry(("bonus", *bonus));
				node.entry(("context", context.clone()));
			}
			Self::SavingThrow { ability, modifier, bonus, context } => {
				match ability {
					None => {
						node.entry_typed("SavingThrow", "All");
					}
					Some(ability) => {
						node += ("SavingThrow", ability.as_kdl());
					}
				}
				node.entry(("modifier", modifier.as_ref().map(Modifier::to_string)));
				node.entry(("bonus", *bonus));
				node.entry(("context", context.clone()));
			}
			Self::Skill { skill, modifier, bonus, context } => {
				node += ("Skill", skill.as_kdl());
				node.entry(("modifier", modifier.as_ref().map(Modifier::to_string)));
				node.entry(("bonus", *bonus));
				node.entry(("context", context.clone()));
			}
			Self::Initiative { modifier, bonus, context } => {
				node.entry("Initiative");
				node.entry(("modifier", modifier.as_ref().map(Modifier::to_string)));
				node.entry(("bonus", *bonus));
				node.entry(("context", context.clone()));
			}
			Self::AttackRoll { bonus, modifier, ability, query } => {
				node.entry_typed("Attack", "Roll");
				node.child(("ability", ability));
				node.child(("bonus", bonus));
				node.child(("modifier", modifier.as_ref().map(ToString::to_string).as_ref()));
				node.children(("query", query));
			}
			Self::AttackDamage { damage, damage_type, query } => {
				node.entry_typed("Attack", "Damage");
				node.child(("damage", damage));
				node.child(("damage_type", &damage_type.as_ref().map(DamageType::to_string)));
				node.children(("query", query.iter()));
			}
			Self::SpellDamage { damage, query } => {
				node.entry_typed("Spell", "Damage");
				node.child(("damage", damage));
				node.children(("query", query));
			}
			Self::SpellRange { distance, query } => {
				node.entry_typed("Spell", "Range");
				node.child(("distance", distance));
				node.children(("query", query));
			}
			Self::SpellHealing { amounts, query } => {
				node.entry_typed("Spell", "Healing");
				node.children(("amount", amounts));
				node.children(("query", query));
			}
			Self::ArmorClass { bonus, context } => {
				node.entry("ArmorClass");
				node.entry(*bonus as i64);
				if let Some(context) = context {
					node.entry(("context", context.clone()));
				}
			}
		}
		node
	}
}

impl FromKdl<NodeContext> for SpellHealingBonus {
	type Error = anyhow::Error;

	fn from_kdl(node: &mut crate::kdl_ext::NodeReader) -> Result<Self, Self::Error> {
		let entry = node.next_req()?;
		match entry.type_opt() {
			Some("SpellRank") => {
				let coefficient = entry.as_i64_req()?;
				Ok(Self::RankScale(coefficient as i32))
			}
			_ => {
				let roll_str = entry.as_str_req()?;
				Ok(Self::Roll(RollSet::from_str(roll_str)?))
			}
		}
	}
}

impl AsKdl for SpellHealingBonus {
	fn as_kdl(&self) -> NodeBuilder {
		let node = NodeBuilder::default();
		match self {
			Self::Roll(roll_set) => node.with_entry_typed(roll_set.to_minified_string(), "Roll"),
			Self::RankScale(coefficient) => node.with_entry_typed(*coefficient as i64, "SpellRank"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		kdl_ext::test_utils::{assert_eq_askdl, assert_eq_fromkdl, from_doc, raw_doc},
		system::{
			dnd5e::{
				data::{action::AttackKind, character::Persistent, item::weapon, roll::Die, Ability, Bundle},
				evaluator::{GetAbilityModifier, GetLevelInt},
				mutator::test::test_utils,
				Value,
			},
			generics,
		},
	};
	use std::path::PathBuf;

	test_utils!(Modify, node_reg());

	fn node_reg() -> generics::Registry {
		let mut node_reg = generics::Registry::default();
		node_reg.register_mutator::<Modify>();
		node_reg.register_evaluator::<GetLevelInt>();
		node_reg.register_evaluator::<GetAbilityModifier>();
		node_reg
	}

	fn character(mutators: Vec<Modify>) -> Character {
		Character::from(Persistent {
			bundles: vec![
				Bundle {
					name: "TestMutator".into(),
					mutators: mutators.into_iter().map(|mutator| mutator.into()).collect(),
					..Default::default()
				}
				.into(),
			],
			..Default::default()
		})
	}

	mod ability {
		use super::*;

		mod kdl {
			use super::*;

			#[test]
			fn specific_noctx() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" (Ability)\"Specific\" \"Dexterity\" modifier=\"Advantage\"";
				let data = Modify::Ability {
					ability: Some(selector::Value::Specific(Ability::Dexterity)),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: None,
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}

			#[test]
			fn anyof_ctx() -> anyhow::Result<()> {
				let doc = "
					|mutator \"modify\" (Ability)\"Any\" modifier=\"Advantage\" context=\"which use smell\" {
					|    option \"Strength\"
					|    option \"Wisdom\"
					|}
				";
				let data = Modify::Ability {
					ability: Some(selector::Value::Options(selector::ValueOptions {
						options: [Ability::Strength, Ability::Wisdom].into(),
						..Default::default()
					})),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("which use smell".into()),
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}
		}

		mod mutate {
			use super::*;

			#[test]
			fn all() {
				let character = character(vec![
					Modify::Ability {
						ability: Some(selector::Value::Specific(Ability::Dexterity)),
						modifier: Some(Modifier::Advantage),
						bonus: None,
						context: None,
					},
					Modify::Ability {
						ability: None,
						modifier: Some(Modifier::Advantage),
						bonus: None,
						context: Some("when climbing".into()),
					},
				]);
				let modifiers = &character.skills()[Ability::Dexterity].modifiers()[Modifier::Advantage];
				assert_eq!(*modifiers, vec![
					(Some("when climbing".into()), PathBuf::from("TestMutator")).into(),
					(None, PathBuf::from("TestMutator")).into(),
				]);
			}

			#[test]
			fn specific() {
				let character = character(vec![Modify::Ability {
					ability: Some(selector::Value::Specific(Ability::Dexterity)),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: None,
				}]);
				let modifiers = &character.skills()[Ability::Dexterity].modifiers()[Modifier::Advantage];
				assert_eq!(*modifiers, vec![(None, PathBuf::from("TestMutator")).into()]);
			}
		}
	}

	mod saving_throw {
		use super::*;

		mod kdl {
			use super::*;

			#[test]
			fn all() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" (SavingThrow)\"All\" modifier=\"Advantage\" context=\"Magic\"";
				let data = Modify::SavingThrow {
					ability: None,
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("Magic".into()),
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}

			#[test]
			fn any_selected() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" (SavingThrow)\"Any\" modifier=\"Advantage\"";
				let data = Modify::SavingThrow {
					ability: Some(selector::Value::Options(selector::ValueOptions::default())),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: None,
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}
		}

		mod mutate {
			use super::*;

			#[test]
			fn all() {
				let character = character(vec![Modify::SavingThrow {
					ability: None,
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("Poison".into()),
				}]);
				let expected = vec![(Some("Poison".into()), PathBuf::from("TestMutator")).into()];
				for ability in enumset::EnumSet::<Ability>::all() {
					let modifiers = &character.saving_throws()[ability].modifiers()[Modifier::Advantage];
					assert_eq!(*modifiers, expected);
				}
			}

			#[test]
			fn specific() {
				let character = character(vec![Modify::SavingThrow {
					ability: Some(selector::Value::Specific(Ability::Constitution)),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("Poison".into()),
				}]);
				let modifiers = &character.saving_throws()[Ability::Constitution].modifiers()[Modifier::Advantage];
				assert_eq!(*modifiers, vec![(Some("Poison".into()), PathBuf::from("TestMutator")).into()]);
			}
		}
	}

	mod skill {
		use super::*;

		mod kdl {
			use super::*;

			#[test]
			fn specific() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" (Skill)\"Specific\" \"Perception\" modifier=\"Advantage\" context=\"using smell\"";
				let data = Modify::Skill {
					skill: selector::Value::Specific(Skill::Perception),
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("using smell".into()),
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}
		}

		mod mutate {
			use super::*;

			#[test]
			fn specific() {
				let character = character(vec![Modify::Skill {
					skill: selector::Value::Specific(Skill::Deception),
					modifier: Some(Modifier::Disadvantage),
					bonus: None,
					context: None,
				}]);
				let modifiers = &character.skills()[Skill::Deception].modifiers()[Modifier::Disadvantage];
				assert_eq!(*modifiers, vec![(None, PathBuf::from("TestMutator")).into()]);
			}
		}
	}

	mod initiative {
		use super::*;

		mod kdl {
			use super::*;

			#[test]
			fn noctx() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" \"Initiative\" modifier=\"Advantage\"";
				let data = Modify::Initiative { modifier: Some(Modifier::Advantage), bonus: None, context: None };
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}

			#[test]
			fn with_context() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" \"Initiative\" modifier=\"Advantage\" context=\"when surprised\"";
				let data = Modify::Initiative {
					modifier: Some(Modifier::Advantage),
					bonus: None,
					context: Some("when surprised".into()),
				};
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}
		}
	}

	mod armor_class {
		use super::*;

		mod kdl {
			use super::*;

			#[test]
			fn no_context() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" \"ArmorClass\" 1";
				let data = Modify::ArmorClass { bonus: 1, context: None };
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}

			#[test]
			fn with_context() -> anyhow::Result<()> {
				let doc = "mutator \"modify\" \"ArmorClass\" 2 context=\"against ranged attacks\"";
				let data = Modify::ArmorClass { bonus: 2, context: Some("against ranged attacks".into()) };
				assert_eq_askdl!(&data, doc);
				assert_eq_fromkdl!(Target, doc, data.into());
				Ok(())
			}
		}
	}

	mod attack {
		use super::*;

		mod roll {
			use super::*;

			mod kdl {
				use super::*;

				#[test]
				fn unrestricted() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Roll\" {
						|    bonus 2
						|}
					";
					let data = Modify::AttackRoll { bonus: Some(2), ability: None, modifier: None, query: vec![] };
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}

				#[test]
				fn restricted() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Roll\" {
						|    bonus 5
						|    query {
						|        attack \"Ranged\"
						|    }
						|}
					";
					let data = Modify::AttackRoll {
						ability: None,
						modifier: None,
						bonus: Some(5),
						query: vec![AttackQuery { attack_kind: AttackKind::Ranged.into(), ..Default::default() }],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}

				#[test]
				fn with_ability() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Roll\" {
						|    ability \"Constitution\"
						|    query {
						|        class \"Shortsword\"
						|    }
						|}
					";
					let data = Modify::AttackRoll {
						ability: Some(Ability::Constitution),
						modifier: None,
						bonus: None,
						query: vec![AttackQuery { classification: ["Shortsword".into()].into(), ..Default::default() }],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}
			}
		}

		mod damage {
			use super::*;

			mod kdl {
				use super::*;

				#[test]
				fn fixed_unrestricted() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Damage\" {
						|    damage 2
						|}
					";
					let data =
						Modify::AttackDamage { damage: EvaluatedRoll::from(2), damage_type: None, query: vec![] };
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}

				#[test]
				fn fixed_restricted() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Damage\" {
						|    damage 5
						|    query {
						|        weapon \"Simple\" \"Martial\"
						|        attack \"Melee\"
						|        ability \"Strength\"
						|        property \"TwoHanded\" false
						|    }
						|}
					";
					let data = Modify::AttackDamage {
						damage: EvaluatedRoll::from(5),
						damage_type: None,
						query: vec![AttackQuery {
							weapon_kind: weapon::Kind::Martial | weapon::Kind::Simple,
							attack_kind: AttackKind::Melee.into(),
							ability: [Ability::Strength].into(),
							properties: [(weapon::Property::TwoHanded, false)].into(),
							classification: [].into(),
						}],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}

				#[test]
				fn roll_typed() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Damage\" {
						|    damage (Roll)\"1d8\"
						|    damage_type \"Radiant\"
						|}
					";
					let data = Modify::AttackDamage {
						damage: EvaluatedRoll::from((1, Die::D8)),
						damage_type: Some(DamageType::Radiant),
						query: vec![],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}

				#[test]
				fn eval_amt() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Attack)\"Damage\" {
						|    damage {
						|        amount (Evaluator)\"get_level\"
						|    }
						|}
					";
					let data = Modify::AttackDamage {
						damage: EvaluatedRoll { amount: Value::Evaluated(GetLevelInt::default().into()), die: None },
						damage_type: None,
						query: vec![],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}
			}
		}
	}

	mod spell {
		use super::*;

		mod damage {
			use super::*;

			mod kdl {
				use super::*;

				#[test]
				fn evocation_wizard() -> anyhow::Result<()> {
					let doc = "
						|mutator \"modify\" (Spell)\"Damage\" {
						|    damage {
						|        amount (Evaluator)\"get_ability_modifier\" (Ability)\"Intelligence\"
						|    }
						|    query {
						|        school \"Evocation\"
						|        tag \"Wizard\"
						|    }
						|}
					";
					let data = Modify::SpellDamage {
						damage: EvaluatedRoll {
							amount: Value::Evaluated(GetAbilityModifier(Ability::Intelligence).into()),
							die: None,
						},
						query: vec![spellcasting::Filter {
							school_tag: Some("Evocation".into()),
							tags: ["Wizard".into()].into(),
							..Default::default()
						}],
					};
					assert_eq_askdl!(&data, doc);
					assert_eq_fromkdl!(Target, doc, data.into());
					Ok(())
				}
			}
		}
	}
}
