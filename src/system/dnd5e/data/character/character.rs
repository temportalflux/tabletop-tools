use crate::{
	path_map::PathMap,
	system::{
		dnd5e::{
			data::{
				character::{
					spellcasting, AbilityScores, AdditionalObjectCache, AttackBonuses, DefaultsBlock, Defenses,
					Derived, DerivedDescription, Features, HitDice, HitPoint, HitPoints, Initiative, MaxHitPoints,
					ObjectCacheProvider, Persistent, ResourceDepot, RestResets, SavingThrows, SelectedSpells, Skills,
					Spellcasting, StartingEquipment, Stat, UserTags,
				},
				item::container::Inventory,
				proficiency, Ability, ArmorClass, Feature, OtherProficiencies,
			},
			mutator::Flag,
			BoxedCriteria, BoxedMutator,
		},
		mutator::{Group, ReferencePath},
		SourceId,
	},
	utility::{selector, Dependencies},
};
use enum_map::EnumMap;
use std::{
	path::{Path, PathBuf},
	str::FromStr,
};

#[derive(Clone, PartialEq)]
pub enum ActionEffect {
	Reset(Persistent, Vec<DefaultsBlock>),
	Recompile,
}

/// The pairing of `Character` and `Derived` to form a singular reference
/// structure for all character data.
#[derive(Clone, PartialEq, Debug)]
pub struct Character {
	// Data applied to all characters based on the modules present in the character's game system
	default_blocks: Vec<DefaultsBlock>,
	// The persistent data that is actually saved uniquely for this character.
	character: Persistent,
	// Data built by processing default_blocks and the persistent character data.
	// Will be cleared and recompiled every time there is a mutation.
	derived: Derived,
	// Mutators encountered when processing default_blocks and persistent character data during derived data recompile.
	// This is a sorted list according to the dependencies of each mutator. This list is a temporary state used during recompile,
	// so that mutators which result in additional mutators can be tracked.
	mutators: Vec<MutatorEntry>,
	// Objects normally stored in the database, which have been cached in local memory.
	// Objects will be cached for varying lengths of time, generlaly correlated with number of recompiles or time duration.
	additional_objects: AdditionalObjectCache,
}
#[derive(Clone, PartialEq, Debug)]
struct MutatorEntry {
	node_id: &'static str,
	parent_path: ReferencePath,
	dependencies: Dependencies,
	mutator: BoxedMutator,
}
#[cfg(test)]
impl From<Persistent> for Character {
	fn from(persistent: Persistent) -> Self {
		let mut character = Self::new(persistent, Vec::new());
		character.recompile_minimal();
		character
	}
}
impl Character {
	pub fn new(persistent: Persistent, default_blocks: Vec<DefaultsBlock>) -> Self {
		Self {
			default_blocks,
			character: persistent,
			derived: Derived::default(),
			mutators: Vec::new(),
			additional_objects: Default::default(),
		}
	}

	pub fn clear_derived(&mut self) {
		self.derived = Derived::default();
		self.mutators.clear();
	}

	#[cfg(test)]
	fn recompile_minimal(&mut self) {
		self.initiaize_recompile();
		self.insert_mutators();
		self.apply_cached_mutators();
	}

	fn initiaize_recompile(&mut self) {
		self.character.set_data_path(&ReferencePath::new());
		self.clear_derived();
	}

	fn insert_mutators(&mut self) {
		for defaults in self.default_blocks.clone() {
			self.apply_from(&defaults, &ReferencePath::new());
		}
		self.apply_from(&self.character.clone(), &ReferencePath::new());
	}

	// TODO: Decouple database+system_depot from dnd data - there can be an abstraction/trait
	// which specifies the minimal API for getting a cached object from a database of content
	pub async fn recompile(&mut self, provider: &ObjectCacheProvider) -> anyhow::Result<()> {
		self.initiaize_recompile();
		self.insert_mutators();

		// TODO: recompiling should be an operation that happens in the background. Mutations can be applied
		// (like editing tags, casting spells, changing the number of items, equipping an item),
		// those mutations are instantaneous, and then a recompile should be asynchronous/background
		// and not prevent the UI from repopulating.

		let mut cache_loops = 0usize;
		// Take all objects that have already been fetched, leaving only the list of pending ids.
		let mut cache = self.additional_objects.take_cached_objects();
		// As long as there are pending objects to fetch, try to do a round of queries.
		while self.additional_objects.has_pending_objects() {
			if cache_loops > 3 {
				log::error!(target: "derived",
					"Hit max number of recursive bundle processing loops. \
					There is likely a recursive loop of bundles and mutators adding each \
					other causing excessive adding of additional bundles."
				);
				break;
			}
			// Move the pending lists into the new cache (where the already fetched objects are).
			cache += std::mem::take(&mut self.additional_objects);
			// Do any additional fetching, resolving all pending objects
			cache.update_objects(&provider).await?;
			// Apply any newly fetched objects (where more pending objects could be requested).
			cache.apply_mutators(self);
			cache_loops += 1;
		}
		// Move the set of all fetched objects back onto the character
		self.additional_objects = cache;

		self.apply_cached_mutators();

		self.inventory_mut().resolve_indirection(&provider).await?;
		self.persistent_mut().conditions.resolve_indirection(&provider).await?;
		self.derived.spellcasting.initialize_ritual_cache(&self.character);

		Ok(())
	}

	pub fn apply_from(&mut self, container: &impl Group<Target = Self>, parent: &ReferencePath) {
		container.apply_mutators(self, parent);
	}

	pub fn apply(&mut self, mutator: &BoxedMutator, parent: &ReferencePath) {
		self.insert_mutator(MutatorEntry {
			node_id: mutator.get_id(),
			parent_path: parent.clone(),
			dependencies: mutator.dependencies(),
			mutator: mutator.clone(),
		});
		mutator.on_insert(self, parent);
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
		let idx = idx.unwrap_or_else(|idx| idx);
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
		if !self.mutators.is_empty() {
			log::warn!(target: "character",
				"Additional mutators were added during the application phase. \
				In order to preserve mutator dependency chain integrity, \
				all mutators should be added during the insertion phase (on_insert).\n\n{:?}",
				self.mutators
			);
		}
	}

	pub fn get_selections_at(&self, path: impl AsRef<Path>) -> Option<&Vec<String>> {
		self.character.get_selections_at(path)
	}

	pub fn get_first_selection(&self, path: impl AsRef<Path>) -> Option<&String> {
		self.character.get_first_selection(path)
	}

	pub fn get_first_selection_at<T>(&self, data_path: impl AsRef<Path>) -> Option<Result<T, <T as FromStr>::Err>>
	where
		T: Clone + 'static + FromStr,
	{
		self.character.get_first_selection_at(data_path)
	}

	pub fn get_selector_value<T>(&self, selector: &selector::Value<Self, T>) -> Option<T>
	where
		T: Clone + 'static + ToString + FromStr,
	{
		if let selector::Value::Specific(value) = selector {
			return Some(value.clone());
		}
		let path_to_data = selector.get_data_path().expect("non-specific selectors must have a data path");
		self.get_first_selection_at::<T>(&path_to_data).map(|res| res.ok()).flatten()
	}

	pub fn resolve_selector<T>(&mut self, selector: &selector::Value<Self, T>) -> Option<T>
	where
		T: Clone + 'static + ToString + FromStr,
	{
		if let selector::Value::Specific(value) = selector {
			return Some(value.clone());
		}
		let path_to_data = selector.get_data_path().expect("non-specific selectors must have a data path");
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

	pub fn export_as_kdl(&self) -> kdl::KdlDocument {
		self.persistent().export_as_kdl()
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
		self.derived.missing_selections.iter().filter_map(|path| path.strip_prefix(&parent).ok()).collect::<Vec<_>>()
	}

	pub fn default_blocks(&self) -> &Vec<DefaultsBlock> {
		&self.default_blocks
	}

	pub fn persistent(&self) -> &Persistent {
		&self.character
	}

	pub fn persistent_mut(&mut self) -> &mut Persistent {
		&mut self.character
	}

	pub fn id(&self) -> &SourceId {
		&self.character.id
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

	pub fn ability_modifier(&self, ability: Ability, proficiency: Option<proficiency::Level>) -> i32 {
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

	pub fn speeds(&self) -> &Stat {
		&self.derived.speeds
	}

	pub fn speeds_mut(&mut self) -> &mut Stat {
		&mut self.derived.speeds
	}

	pub fn senses(&self) -> &Stat {
		&self.derived.senses
	}

	pub fn senses_mut(&mut self) -> &mut Stat {
		&mut self.derived.senses
	}

	pub fn level(&self, class_name: Option<&str>) -> usize {
		self.character.level(class_name)
	}

	pub fn proficiency_bonus(&self) -> i32 {
		proficiency::proficiency_bonus(self.character.level(None))
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

	pub fn initiative(&self) -> &Initiative {
		&self.derived.initiative
	}

	pub fn initiative_mut(&mut self) -> &mut Initiative {
		&mut self.derived.initiative
	}

	pub fn attack_bonuses(&self) -> &AttackBonuses {
		&self.derived.attack_bonuses
	}

	pub fn attack_bonuses_mut(&mut self) -> &mut AttackBonuses {
		&mut self.derived.attack_bonuses
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

	pub fn add_bundles(&mut self, object_data: super::AdditionalObjectData) {
		self.additional_objects.insert(object_data);
	}

	pub fn add_feature(&mut self, feature: Feature, parent_path: &ReferencePath) {
		self.apply_from(&feature, parent_path);
		self.features_mut().path_map.insert(parent_path.join(&feature.name, None).display, feature);
	}

	pub fn features(&self) -> &Features {
		&self.derived.features
	}

	pub fn features_mut(&mut self) -> &mut Features {
		&mut self.derived.features
	}

	pub fn inventory(&self) -> &Inventory {
		&self.character.inventory
	}

	pub fn inventory_mut(&mut self) -> &mut Inventory {
		&mut self.character.inventory
	}

	pub fn derived_description(&self) -> &DerivedDescription {
		&self.derived.description
	}

	pub fn derived_description_mut(&mut self) -> &mut DerivedDescription {
		&mut self.derived.description
	}

	pub fn spellcasting(&self) -> &Spellcasting {
		&self.derived.spellcasting
	}

	pub fn spellcasting_mut(&mut self) -> &mut Spellcasting {
		&mut self.derived.spellcasting
	}

	pub fn cantrip_capacity(&self) -> Vec<(usize, &spellcasting::Restriction)> {
		self.spellcasting().cantrip_capacity(&self.character)
	}

	pub fn selected_spells(&self) -> &SelectedSpells {
		&self.character.selected_spells
	}

	pub fn starting_equipment(&self) -> &Vec<(Vec<StartingEquipment>, PathBuf)> {
		&self.derived.starting_equipment
	}

	pub fn add_starting_equipment(&mut self, entries: &Vec<StartingEquipment>, source: &ReferencePath) {
		self.derived.starting_equipment.push((entries.clone(), source.display.clone()));
	}

	pub fn rest_resets_mut(&mut self) -> &mut RestResets {
		&mut self.derived.rest_resets
	}

	pub fn rest_resets(&self) -> &RestResets {
		&self.derived.rest_resets
	}

	pub fn resources(&self) -> &ResourceDepot {
		&self.derived.resource_depot
	}

	pub fn resources_mut(&mut self) -> &mut ResourceDepot {
		&mut self.derived.resource_depot
	}

	pub fn attunement(&self) -> u32 {
		self.derived.attunement_count
	}

	pub fn attunement_mut(&mut self) -> &mut u32 {
		&mut self.derived.attunement_count
	}

	pub fn hit_dice(&self) -> &HitDice {
		&self.derived.hit_dice
	}

	pub fn hit_dice_mut(&mut self) -> &mut HitDice {
		&mut self.derived.hit_dice
	}

	pub fn user_tags(&self) -> &UserTags {
		&self.derived.user_tags
	}

	pub fn user_tags_mut(&mut self) -> &mut UserTags {
		&mut self.derived.user_tags
	}
}
