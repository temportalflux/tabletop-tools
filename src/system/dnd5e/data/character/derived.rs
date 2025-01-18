use super::PersonalityKind;
use crate::{
	system::{
		dnd5e::{
			data::{
				action::AttackQuery,
				roll::{Modifier, Roll, RollSet},
				Ability, ArmorClass, Condition, DamageType, Indirect, OtherProficiencies, Rest, Spell,
			},
			mutator::{Defense, Flag},
		},
		mutator::ReferencePath,
	},
	utility::NotInList,
};
use enum_map::{enum_map, EnumMap};
use std::{
	collections::{BTreeMap, HashSet},
	path::{Path, PathBuf},
};

mod ability_score;
pub use ability_score::*;
mod actions;
pub use actions::*;
mod initiative;
pub use initiative::*;
mod hit_die;
pub use hit_die::*;
mod object_cache;
pub use object_cache::*;
mod resource_depot;
pub use resource_depot::*;
mod saving_throw;
pub use saving_throw::*;
mod size;
pub use size::*;
mod skill;
pub use skill::*;
pub mod spellcasting;
pub use spellcasting::Spellcasting;
mod starting_equipment;
pub use starting_equipment::*;
mod stat;
pub use stat::*;
mod user_tags;
pub use user_tags::*;

/// Data derived from the `Persistent`, such as bonuses to abilities/skills,
/// proficiencies, and actions. This data all lives within `Persistent` in
/// its various features and subtraits, and is compiled into one flat
/// structure for easy reference when displaying the character information.
#[derive(Clone, PartialEq, Debug)]
pub struct Derived {
	pub missing_selections: Vec<PathBuf>,
	pub ability_scores: AbilityScores,
	pub saving_throws: SavingThrows,
	pub skills: Skills,
	pub other_proficiencies: OtherProficiencies,
	pub speeds: Stat,
	pub senses: Stat,
	pub defenses: Defenses,
	pub max_hit_points: MaxHitPoints,
	pub initiative: Initiative,
	pub attack_bonuses: AttackBonuses,
	pub armor_class: ArmorClass,
	pub features: Features,
	pub description: DerivedDescription,
	pub flags: EnumMap<Flag, bool>,
	pub spellcasting: Spellcasting,
	pub starting_equipment: Vec<(Vec<StartingEquipment>, PathBuf)>,
	pub hit_dice: HitDice,
	pub rest_resets: RestResets,
	pub resource_depot: ResourceDepot,
	pub attunement_count: u32,
	pub user_tags: UserTags,
}

impl Default for Derived {
	fn default() -> Self {
		Self {
			missing_selections: Default::default(),
			ability_scores: Default::default(),
			saving_throws: Default::default(),
			skills: Default::default(),
			other_proficiencies: Default::default(),
			speeds: Default::default(),
			senses: Default::default(),
			defenses: Default::default(),
			max_hit_points: Default::default(),
			initiative: Default::default(),
			attack_bonuses: Default::default(),
			armor_class: Default::default(),
			features: Default::default(),
			description: Default::default(),
			flags: enum_map! {
				Flag::ArmorStrengthRequirement => true,
			},
			spellcasting: Default::default(),
			starting_equipment: Default::default(),
			hit_dice: Default::default(),
			rest_resets: Default::default(),
			resource_depot: Default::default(),
			attunement_count: 0,
			user_tags: Default::default(),
		}
	}
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Defenses(EnumMap<Defense, Vec<DefenseEntry>>);
#[derive(Clone, PartialEq, Debug)]
pub struct DefenseEntry {
	pub damage_type: Option<DamageType>,
	pub context: Option<String>,
	pub source: PathBuf,
}
impl Defenses {
	pub fn push(
		&mut self, kind: Defense, damage_type: Option<DamageType>, context: Option<String>, source: &ReferencePath,
	) {
		self.0[kind].push(DefenseEntry { damage_type, context, source: source.display.clone() });
	}
}
impl std::ops::Deref for Defenses {
	type Target = EnumMap<Defense, Vec<DefenseEntry>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct DerivedDescription {
	pub life_expectancy: i32,
	pub size_formula: SizeFormula,
	pub personality_suggestions: EnumMap<PersonalityKind, Vec<String>>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct MaxHitPoints(i32, BTreeMap<PathBuf, i32>);
impl MaxHitPoints {
	pub fn push(&mut self, bonus: i32, source: &ReferencePath) {
		self.0 = self.0.saturating_add(bonus);
		self.1.insert(source.display.clone(), bonus);
	}

	pub fn value(&self) -> u32 {
		self.0.max(0) as u32
	}

	pub fn sources(&self) -> &BTreeMap<PathBuf, i32> {
		&self.1
	}
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct AttackBonuses {
	attack_roll: Vec<AttackRollBonus>,
	attack_damage: Vec<AttackDamageBonus>,
	attack_ability: Vec<AttackAbility>,
	spell_damage: Vec<SpellDamageBonus>,
	spell_range: Vec<ModificationSpellRange>,
	spell_healing: Vec<ModificationSpellHealing>,
}
#[derive(Clone, PartialEq, Debug)]
struct AttackRollBonus {
	bonus: i32,
	modifier: Option<Modifier>,
	queries: Vec<AttackQuery>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
struct AttackDamageBonus {
	amount: Roll,
	damage_type: Option<DamageType>,
	queries: Vec<AttackQuery>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
struct SpellDamageBonus {
	amount: Roll,
	queries: Vec<spellcasting::Filter>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
struct ModificationSpellRange {
	minimum: u32,
	queries: Vec<spellcasting::Filter>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
struct AttackAbility {
	ability: Ability,
	queries: Vec<AttackQuery>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
struct ModificationSpellHealing {
	bonuses: Vec<SpellHealingBonus>,
	queries: Vec<spellcasting::Filter>,
	source: PathBuf,
}
#[derive(Clone, PartialEq, Debug)]
pub enum SpellHealingBonus {
	// A fixed amount
	Roll(RollSet),
	// Adds a value equivalent to the coeffecient * the casted spell's rank
	RankScale(i32),
}
impl AttackBonuses {
	pub fn add_to_weapon_attacks(&mut self, bonus: i32, queries: Vec<AttackQuery>, source: &ReferencePath) {
		self.attack_roll.push(AttackRollBonus { bonus, modifier: None, queries, source: source.display.clone() });
	}

	pub fn modify_weapon_attacks(&mut self, modifier: Modifier, queries: Vec<AttackQuery>, source: &ReferencePath) {
		self.attack_roll.push(AttackRollBonus {
			bonus: 0,
			modifier: Some(modifier),
			queries,
			source: source.display.clone(),
		});
	}

	pub fn add_to_weapon_damage(
		&mut self, amount: Roll, damage_type: Option<DamageType>, queries: Vec<AttackQuery>, source: &ReferencePath,
	) {
		self.attack_damage.push(AttackDamageBonus { amount, damage_type, queries, source: source.display.clone() });
	}

	pub fn add_ability_modifier(&mut self, ability: Ability, queries: Vec<AttackQuery>, source: &ReferencePath) {
		self.attack_ability.push(AttackAbility { ability, queries, source: source.display.clone() });
	}

	pub fn add_to_spell_damage(&mut self, amount: Roll, queries: Vec<spellcasting::Filter>, source: &ReferencePath) {
		self.spell_damage.push(SpellDamageBonus { amount, queries, source: source.display.clone() });
	}

	pub fn modify_spell_range(&mut self, minimum: u32, queries: Vec<spellcasting::Filter>, source: &ReferencePath) {
		self.spell_range.push(ModificationSpellRange { minimum, queries, source: source.display.clone() });
	}

	pub fn modify_spell_healing(
		&mut self, bonuses: Vec<SpellHealingBonus>, queries: Vec<spellcasting::Filter>, source: &ReferencePath,
	) {
		self.spell_healing.push(ModificationSpellHealing { bonuses, queries, source: source.display.clone() });
	}

	pub fn get_weapon_attack(
		&self, action: &crate::system::dnd5e::data::action::Action,
	) -> Vec<(i32, Option<Modifier>, &Path)> {
		let mut bonuses = Vec::new();
		let Some(attack) = &action.attack else {
			return bonuses;
		};
		// Iterate over each bonus group, gathering any which have any query which matches the attack
		for bonus in &self.attack_roll {
			// If any query in that bonus matches the attack
			'iter_query: for query in &bonus.queries {
				if query.is_attack_valid(attack) {
					// then add the bonus and early-exit iteration on this bonus
					bonuses.push((bonus.bonus, bonus.modifier, bonus.source.as_path()));
					break 'iter_query;
				}
			}
		}
		bonuses
	}

	pub fn get_weapon_damage(
		&self, action: &crate::system::dnd5e::data::action::Action,
	) -> Vec<(&Roll, &Option<DamageType>, &Path)> {
		let mut bonuses = Vec::new();
		let Some(attack) = &action.attack else {
			return bonuses;
		};
		// Iterate over each bonus group, gathering any which have any query which matches the action
		for bonus in &self.attack_damage {
			// If any query in that bonus matches the attack
			'iter_query: for query in &bonus.queries {
				if query.is_attack_valid(attack) {
					// then add the bonus and early-exit iteration on this bonus
					bonuses.push((&bonus.amount, &bonus.damage_type, bonus.source.as_path()));
					break 'iter_query;
				}
			}
		}
		bonuses
	}

	pub fn get_attack_ability_variants(&self, attack: &crate::system::dnd5e::data::action::Attack) -> HashSet<Ability> {
		// TODO: this doesnt report out the sources for the ability variants
		let mut abilities = HashSet::default();
		for bonus in &self.attack_ability {
			'iter_query: for query in &bonus.queries {
				if query.is_attack_valid(attack) {
					abilities.insert(bonus.ability);
					break 'iter_query;
				}
			}
		}
		abilities
	}

	pub fn get_spell_damage(&self, spell: &Spell) -> Vec<(&Roll, &Path)> {
		let mut bonuses = Vec::new();
		for bonus in &self.spell_damage {
			// Filter out any bonuses which do not meet the restriction
			'iter_query: for query in &bonus.queries {
				if query.matches(spell) {
					bonuses.push((&bonus.amount, bonus.source.as_path()));
					break 'iter_query;
				}
			}
		}
		bonuses
	}

	fn iter_spell_range<'this>(
		&'this self, spell: &'this Spell,
	) -> impl Iterator<Item = &'this ModificationSpellRange> + '_ {
		self.spell_range.iter().filter(|modification| modification.queries.iter().any(|query| query.matches(spell)))
	}

	pub fn get_spell_range_minimum(&self, spell: &Spell) -> Option<u32> {
		self.iter_spell_range(spell).map(|modification| modification.minimum).max()
	}

	fn iter_spell_healing<'this>(
		&'this self, spell: &'this Spell,
	) -> impl Iterator<Item = &'this ModificationSpellHealing> + '_ {
		self.spell_healing.iter().filter(|modification| modification.queries.iter().any(|query| query.matches(spell)))
	}

	pub fn get_spell_healing_bonuses<'this>(
		&'this self, spell: &'this Spell,
	) -> impl Iterator<Item = (&'this SpellHealingBonus, &'this PathBuf)> + '_ {
		let iter = self.iter_spell_healing(spell);
		let iter = iter.map(|modification| modification.bonuses.iter().map(|bonus| (bonus, &modification.source)));
		iter.flatten()
	}
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct RestResets {
	entries: EnumMap<Rest, Vec<RestEntry>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct RestEntry {
	pub effects: Vec<RestEffect>,
	pub source: PathBuf,
}
impl RestResets {
	pub fn add(&mut self, rest: Rest, entry: RestEntry) {
		self.entries[rest].push(entry);
	}

	pub fn get(&self, rest: Rest) -> &Vec<RestEntry> {
		&self.entries[rest]
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum RestEffect {
	RestoreCurrentHP,
	ClearTempHP,
	GrantTempHP(RollSet),
	ClearDeathSaves,
	RestoreHitDice(Option<RollSet>),
	UseHitDice(RollSet),
	// None => All to max
	// [#: None] => rank to max
	// [#: #] => add an amount of slots for that rank
	GrantSpellSlots(Option<BTreeMap<u8, Option<u32>>>),
	RestoreResourceUses { data_path: PathBuf, amount: Option<RollSet> },
	GrantCondition(Indirect<Condition>),
}
impl kdlize::FromKdl<crate::kdl_ext::NodeContext> for RestEffect {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"RestoreCurrentHP" => Ok(Self::RestoreCurrentHP),
			"ClearTempHP" => Ok(Self::ClearTempHP),
			"GrantTempHP" => Ok(Self::GrantTempHP(node.next_str_req_t()?)),
			"ClearDeathSaves" => Ok(Self::ClearDeathSaves),
			"RestoreHitDice" => Ok(Self::RestoreHitDice(node.next_str_opt_t()?)),
			"UseHitDice" => Ok(Self::UseHitDice(node.next_str_req_t()?)),
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
				let amount = node.next_str_opt_t()?;
				Ok(Self::RestoreResourceUses { data_path, amount })
			}
			"GrantCondition" => Ok(Self::GrantCondition(Indirect::from_kdl(node)?)),
			id => Err(NotInList(id.to_owned(), vec![
				"RestoreCurrentHP",
				"ClearTempHP",
				"GrantTempHP",
				"ClearDeathSaves",
				"RestoreHitDice",
				"UseHitDice",
				"GrantSpellSlots",
				"RestoreResourceUses",
				"GrantCondition",
			])
			.into()),
		}
	}
}
impl kdlize::AsKdl for RestEffect {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		match self {
			Self::RestoreCurrentHP => {
				node.entry("RestoreCurrentHP");
			}
			Self::ClearTempHP => {
				node.entry("ClearTempHP");
			}
			Self::GrantTempHP(hit_dice) => {
				node.entry("GrantTempHP");
				node.entry(hit_dice.to_string());
			}
			Self::ClearDeathSaves => {
				node.entry("ClearDeathSaves");
			}
			Self::RestoreHitDice(hit_dice) => {
				node.entry("RestoreHitDice");
				node.entry(hit_dice.as_ref().map(RollSet::to_string));
			}
			Self::UseHitDice(hit_dice) => {
				node.entry("UseHitDice");
				node.entry(hit_dice.to_string());
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
				node.entry(amount.as_ref().map(RollSet::to_string));
			}
			Self::GrantCondition(condition) => {
				node.entry("GrantCondition");
				node += condition.as_kdl();
			}
		}
		node
	}
}
