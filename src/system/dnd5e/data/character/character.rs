use crate::{
	path_map::PathMap,
	system::dnd5e::{
		data::{
			action::{Action, ActionSource, AttackCheckKind},
			character::{
				AbilityScores, Defenses, Derived, DerivedDescription, MaxHitPoints, Persistent,
				SavingThrows, Senses, Skills, Speeds,
			},
			item::{self, weapon, ItemKind},
			mutator::Flag,
			proficiency, Ability, ArmorClass, BoxedFeature, OtherProficiencies,
		},
		BoxedCriteria, BoxedMutator,
	},
	utility::{Dependencies, MutatorGroup, Selector},
};
use enum_map::{Enum, EnumMap};
use enumset::EnumSetType;
use std::{
	path::{Path, PathBuf},
	rc::Rc,
	str::FromStr,
};

use super::HitPoints;

#[derive(Clone, Copy, PartialEq)]
pub enum ActionEffect {
	Recompile,
}

/// The pairing of `Character` and `Derived` to form a singlular reference
/// structure for all character data.
#[derive(Clone, PartialEq, Debug)]
pub struct Character {
	character: Persistent,
	derived: Derived,
	mutators: Vec<MutatorEntry>,
}
#[derive(Clone, PartialEq, Debug)]
struct MutatorEntry {
	node_id: &'static str,
	parent_path: PathBuf,
	dependencies: Dependencies,
	mutator: BoxedMutator,
}
impl From<Persistent> for Character {
	fn from(persistent: Persistent) -> Self {
		persistent.set_data_path(&PathBuf::new());
		let mut character = Self {
			character: persistent.clone(),
			derived: Derived::default(),
			mutators: Vec::new(),
		};
		character.apply_from(&persistent, &PathBuf::new());
		character.apply_cached_mutators();
		character
	}
}
impl yew::Reducible for Character {
	type Action = Box<dyn FnOnce(&mut Persistent, &Rc<Self>) -> Option<ActionEffect>>;

	fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
		let mut full = (*self).clone();
		Rc::new(match action(&mut full.character, &self) {
			None => full,
			Some(ActionEffect::Recompile) => Self::from(full.character.clone()),
		})
	}
}
impl Character {
	pub fn apply_from(&mut self, container: &impl MutatorGroup<Target = Self>, parent: &Path) {
		container.apply_mutators(self, parent);
	}

	pub fn apply(&mut self, mutator: &BoxedMutator, parent: &Path) {
		self.insert_mutator(MutatorEntry {
			node_id: mutator.get_id(),
			parent_path: parent.to_owned(),
			dependencies: mutator.dependencies(),
			mutator: mutator.clone(),
		});
	}

	fn insert_mutator(&mut self, incoming: MutatorEntry) {
		let idx = self.mutators.binary_search_by(|existing| {
			match (&*existing.dependencies, &*incoming.dependencies) {
				// neither have dependencies, so they are considered equal, might as well order by node_id.
				(None, None) => existing.node_id.cmp(&incoming.node_id),
				// existing has deps, and incoming does not. incoming must come first
				(Some(_), None) => std::cmp::Ordering::Greater,
				// incoming has deps, and existing does not. existing must come first
				(None, Some(_)) => std::cmp::Ordering::Less,
				// both have deps, determine if either requires the other
				(Some(existing_deps), Some(incoming_deps)) => {
					let existing_reqs_incoming = existing_deps.contains(&incoming.node_id);
					let incoming_reqs_existing = incoming_deps.contains(&existing.node_id);
					match (existing_reqs_incoming, incoming_reqs_existing) {
						// existing requires incoming, incoming must come first
						(true, false) => std::cmp::Ordering::Greater,
						// incoming requires existing, existing must come first
						(false, true) => std::cmp::Ordering::Less,
						// existing is not required for new, might as well sort by node_id.
						(false, false) => existing.node_id.cmp(&incoming.node_id),
						(true, true) => panic!(
							"circular mutator dependency between {:?} and {:?}",
							existing.node_id, incoming.node_id
						),
					}
				}
			}
		});
		let idx = match idx {
			Ok(idx) => idx,
			Err(idx) => idx,
		};
		self.mutators.insert(idx, incoming);
	}

	fn apply_cached_mutators(&mut self) {
		let mutators = self.mutators.drain(..).collect::<Vec<_>>();
		for entry in mutators.into_iter() {
			/*
			log::debug!(
				target: "character",
				"applying mutator: {:?} deps:{:?}",
				entry.node_id,
				entry.dependencies
			);
			*/
			entry.mutator.apply(self, &entry.parent_path);
		}
	}

	fn get_selections_at(&self, path: impl AsRef<Path>) -> Option<&Vec<String>> {
		self.character.selected_values.get(path.as_ref())
	}

	pub fn get_first_selection(&self, path: impl AsRef<Path>) -> Option<&String> {
		self.get_selections_at(path)
			.map(|all| all.first())
			.flatten()
	}

	pub fn get_first_selection_at<T>(
		&self,
		data_path: impl AsRef<Path>,
	) -> Option<Result<T, <T as FromStr>::Err>>
	where
		T: Clone + 'static + FromStr,
	{
		let selections = self.get_selections_at(data_path);
		selections
			.map(|all| all.first())
			.flatten()
			.map(|selected| T::from_str(&selected))
	}

	pub fn get_selector_value<T>(&self, selector: &Selector<T>) -> Option<T>
	where
		T: Clone + 'static + ToString + FromStr,
	{
		if let Selector::Specific(value) = selector {
			return Some(value.clone());
		}
		let path_to_data = selector
			.get_data_path()
			.expect("non-specific selectors must have a data path");
		self.get_first_selection_at::<T>(&path_to_data)
			.map(|res| res.ok())
			.flatten()
	}

	pub fn resolve_selector<T>(&mut self, selector: &Selector<T>) -> Option<T>
	where
		T: Clone + 'static + ToString + FromStr,
	{
		if let Selector::Specific(value) = selector {
			return Some(value.clone());
		}
		let path_to_data = selector
			.get_data_path()
			.expect("non-specific selectors must have a data path");
		let value = match self.get_first_selection_at::<T>(&path_to_data) {
			Some(Ok(value)) => Some(value),
			Some(Err(_)) => None,
			None => {
				self.derived.missing_selections.push(path_to_data);
				None
			}
		};
		value
	}
}

impl Character {
	pub fn evaluate(&self, criteria: &BoxedCriteria) -> Result<(), String> {
		criteria.evaluate(&self)
	}

	pub fn selected_values_in(&self, parent: impl AsRef<Path>) -> Option<&PathMap<String>> {
		self.character.selected_values.get_all(parent)
	}

	pub fn missing_selections_in(&self, parent: impl AsRef<Path>) -> Vec<&Path> {
		self.derived
			.missing_selections
			.iter()
			.filter_map(|path| path.strip_prefix(&parent).ok())
			.collect::<Vec<_>>()
	}

	pub fn persistent(&self) -> &Persistent {
		&self.character
	}

	pub fn flags(&self) -> &EnumMap<Flag, bool> {
		&self.derived.flags
	}

	pub fn flags_mut(&mut self) -> &mut EnumMap<Flag, bool> {
		&mut self.derived.flags
	}

	pub fn ability_scores(&self) -> &AbilityScores {
		&self.derived.ability_scores
	}

	pub fn ability_scores_mut(&mut self) -> &mut AbilityScores {
		&mut self.derived.ability_scores
	}

	pub fn ability_modifier(
		&self,
		ability: Ability,
		proficiency: Option<proficiency::Level>,
	) -> i32 {
		let modifier = self.ability_scores().get(ability).score().modifier();
		let bonus = match proficiency {
			Some(proficiency) => proficiency * self.proficiency_bonus(),
			None => 0,
		};
		modifier + bonus
	}

	pub fn saving_throws(&self) -> &SavingThrows {
		&self.derived.saving_throws
	}

	pub fn saving_throws_mut(&mut self) -> &mut SavingThrows {
		&mut self.derived.saving_throws
	}

	pub fn skills(&self) -> &Skills {
		&self.derived.skills
	}

	pub fn skills_mut(&mut self) -> &mut Skills {
		&mut self.derived.skills
	}

	pub fn armor_class(&self) -> &ArmorClass {
		&self.derived.armor_class
	}

	pub fn armor_class_mut(&mut self) -> &mut ArmorClass {
		&mut self.derived.armor_class
	}

	pub fn speeds(&self) -> &Speeds {
		&self.derived.speeds
	}

	pub fn speeds_mut(&mut self) -> &mut Speeds {
		&mut self.derived.speeds
	}

	pub fn senses(&self) -> &Senses {
		&self.derived.senses
	}

	pub fn senses_mut(&mut self) -> &mut Senses {
		&mut self.derived.senses
	}

	pub fn level(&self, class_name: Option<&str>) -> usize {
		self.character.level(class_name)
	}

	pub fn proficiency_bonus(&self) -> i32 {
		proficiency::proficiency_bonus(self.character.level(None))
	}

	pub fn initiative_bonus(&self) -> i32 {
		self.ability_modifier(Ability::Dexterity, None)
	}

	pub fn inspiration(&self) -> bool {
		self.character.inspiration
	}

	pub fn get_hp(&self, kind: HitPoint) -> u32 {
		match kind {
			HitPoint::Current => self.character.hit_points.current,
			HitPoint::Max => self.derived.max_hit_points.value(),
			HitPoint::Temp => self.character.hit_points.temp,
		}
	}

	pub fn max_hit_points(&self) -> &MaxHitPoints {
		&self.derived.max_hit_points
	}

	pub fn max_hit_points_mut(&mut self) -> &mut MaxHitPoints {
		&mut self.derived.max_hit_points
	}

	pub fn hit_points(&self) -> &HitPoints {
		self.character.hit_points()
	}

	pub fn hit_points_mut(&mut self) -> &mut HitPoints {
		self.character.hit_points_mut()
	}

	pub fn defenses(&self) -> &Defenses {
		&self.derived.defenses
	}

	pub fn defenses_mut(&mut self) -> &mut Defenses {
		&mut self.derived.defenses
	}

	pub fn other_proficiencies(&self) -> &OtherProficiencies {
		&self.derived.other_proficiencies
	}

	pub fn other_proficiencies_mut(&mut self) -> &mut OtherProficiencies {
		&mut self.derived.other_proficiencies
	}

	pub fn add_feature(&mut self, feature: &BoxedFeature, parent_path: &Path) {
		self.derived.features.insert(parent_path, feature.clone());
		self.apply_from(feature.inner(), parent_path);
	}

	pub fn features(&self) -> &PathMap<BoxedFeature> {
		&self.derived.features
	}

	pub fn actions(&self) -> &Vec<Action> {
		&self.derived.actions
	}

	pub fn actions_mut(&mut self) -> &mut Vec<Action> {
		&mut self.derived.actions
	}

	pub fn inventory(&self) -> &item::Inventory {
		&self.character.inventory
	}

	pub fn inventory_mut(&mut self) -> &mut item::Inventory {
		&mut self.character.inventory
	}

	pub fn iter_actions_mut_for(
		&mut self,
		restriction: &Option<weapon::Restriction>,
	) -> Vec<&mut Action> {
		let mut actions = self.derived.actions.iter_mut().collect::<Vec<_>>();
		if let Some(weapon::Restriction {
			weapon_kind,
			attack_kind,
			ability,
		}) = restriction
		{
			if !weapon_kind.is_empty() {
				actions = actions
					.into_iter()
					.filter_map(|action| {
						let Some(ActionSource::Item(item_id)) = &action.source else { return None; };
						let Some(item) = self.character.inventory.get_item(item_id) else { return None; };
						let ItemKind::Equipment(equipment) = &item.kind else { return None; };
						let Some(weapon) = &equipment.weapon else { return None; };
						weapon_kind.contains(&weapon.kind).then_some(action)
					})
					.collect();
			}

			if !attack_kind.is_empty() {
				actions = actions
					.into_iter()
					.filter_map(|action| {
						let Some(attack) = &action.attack else { return None; };
						attack_kind.contains(&attack.kind.kind()).then_some(action)
					})
					.collect();
			}

			if !ability.is_empty() {
				actions = actions.into_iter().filter_map(|action| {
					let Some(attack) = &action.attack else { return None; };
					let AttackCheckKind::AttackRoll { ability: atk_roll_ability, .. } = &attack.check else { return None; };
					ability.contains(atk_roll_ability).then_some(action)
				}).collect();
			}
		}
		actions
	}

	pub fn derived_description_mut(&mut self) -> &mut DerivedDescription {
		&mut self.derived.description
	}
}

#[derive(Debug, EnumSetType, Enum)]
pub enum HitPoint {
	Current,
	Max,
	Temp,
}
