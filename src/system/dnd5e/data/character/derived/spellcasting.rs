use crate::{
	database::{entry::EntryInSystemWithType, Criteria, Query},
	system::{
		dnd5e::{
			data::{
				character::{Character, ObjectCacheProvider, Persistent},
				spell::Spell,
			},
			DnD5e,
		},
		mutator::ReferencePath,
		SourceId, System,
	},
	utility::AddAssignMap,
};
use derivative::Derivative;
use multimap::MultiMap;
use std::{
	collections::{BTreeMap, HashMap, HashSet},
	path::PathBuf,
};

mod caster;
pub use caster::*;
mod cantrips;
pub use cantrips::*;
mod entry;
pub use entry::*;
mod filter;
pub use filter::*;
mod slots;
pub use slots::*;

#[derive(Clone, PartialEq, Debug, Derivative)]
#[derivative(Default)]
pub struct Spellcasting {
	/// The spellcasting features available to a character.
	/// Each feature contains things like the spellcasting ability,
	/// ritual casting, cantrip capacity, leveled-spell capacity, slot scaling, etc.
	/// Keyed by the Caster's name/id.
	casters: HashMap<String, Caster>,
	/// Any additional spells that are available to a particular casting feature/class.
	/// Each entry is a list of additional Spell SourceIds, and the feature which granted that access.
	/// Spells in this list are made available to be selected (known or prepared) by casters,
	/// even if that spell is not tagged with this caster/class name.
	/// When these spells are selected, they are treated as if they are spells that class/caster can traditionally cast.
	/// Keyed by the Caster's name/id.
	additional_caster_spells: AdditionalAccess,
	/// Map of Spell SourceIds to the cached spell and the sources+entries which granted access.
	/// Spells in this list are always castable/prepared.
	always_prepared: HashMap<SourceId, AlwaysPreparedSpell>,
	/// A cache of spells queried from the data provider which casters can ritual cast.
	ritual_spells: RitualSpellCache,
	// Manual system flag for preventing spellcasting when wearing armor that isnt proficient
	#[derivative(Default(value = "true"))]
	pub can_cast_any: bool,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct AdditionalAccess {
	by_caster: MultiMap<String, SourceId>,
	sources: MultiMap<SourceId, PathBuf>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct AlwaysPreparedSpell {
	pub spell: Option<Spell>,
	pub entries: HashMap<PathBuf, SpellEntry>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct RitualSpellCache {
	// Maps id to the cached spell object
	pub spells: HashMap<SourceId, Spell>,
	pub caster_lists: MultiMap<String, SourceId>,
	pub casters_which_prepare_from_item: HashSet<String>,
}

impl Spellcasting {
	pub fn add_caster(&mut self, caster: Caster) {
		self.casters.insert(caster.name().clone(), caster);
	}

	pub fn add_spell_access(&mut self, caster_name: &String, spell_ids: &Vec<SourceId>, source: &ReferencePath) {
		for spell_id in spell_ids {
			self.additional_caster_spells.by_caster.insert(caster_name.clone(), spell_id.clone());
			self.additional_caster_spells.sources.insert(spell_id.clone(), source.display.clone());
		}
	}

	pub fn add_prepared(&mut self, spell_id: &SourceId, entry: SpellEntry) {
		let spell_id = spell_id.unversioned();
		if !self.always_prepared.contains_key(&spell_id) {
			self.always_prepared.insert(spell_id.clone(), AlwaysPreparedSpell::default());
		}
		self.always_prepared.get_mut(&spell_id).unwrap().entries.insert(entry.source.clone(), entry);
	}

	pub fn add_prepared_spell(&mut self, spell: &Spell, entry: SpellEntry) {
		let id = spell.id.unversioned();
		self.add_prepared(&id, entry);
		self.always_prepared.get_mut(&id).unwrap().spell = Some(spell.clone());
	}

	pub async fn fetch_spell_objects(
		&mut self, provider: &ObjectCacheProvider, persistent: &Persistent,
	) -> anyhow::Result<()> {
		self.fetch_always_prepared(provider).await?;
		self.ritual_spells = self.fetch_rituals(provider, persistent).await?;
		Ok(())
	}

	async fn fetch_always_prepared(&mut self, provider: &ObjectCacheProvider) -> anyhow::Result<()> {
		for (id, spell_entry) in &mut self.always_prepared {
			spell_entry.spell =
				provider.database.get_typed_entry::<Spell>(id.clone(), provider.system_depot.clone(), None).await?;
		}
		Ok(())
	}

	async fn fetch_rituals(
		&self, provider: &ObjectCacheProvider, persistent: &Persistent,
	) -> anyhow::Result<RitualSpellCache> {
		let mut caster_query_criteria = Vec::new();
		let mut caster_filters = HashMap::new();
		let mut casters_which_prepare_from_item = HashSet::new();
		for caster in self.iter_casters() {
			let Some(ritual_capability) = &caster.ritual_capability else {
				continue;
			};
			if !ritual_capability.available_spells {
				continue;
			}

			let mut filter = self.get_filter(caster.name(), persistent).unwrap_or_default();
			// each spell the filter matches must be a ritual
			filter.ritual = Some(true);

			let criteria = filter.as_criteria();
			caster_query_criteria.push(criteria.clone());
			caster_filters.insert(caster.name(), criteria);

			// We only store the caster filter if the caster doesnt prepare from the item.
			// Casters skipped here are handled manually in the spells panel.
			if caster.prepare_from_item {
				casters_which_prepare_from_item.insert(caster.name().clone());
			}
		}
		let criteria = Criteria::Any(caster_query_criteria);

		let index = EntryInSystemWithType::new::<Spell>(DnD5e::id());
		let query = Query::subset(&provider.database, Some(index)).await?;
		let query = query.filter_by(criteria);
		let mut query = query.parse_as::<Spell>(&provider.system_depot);

		let mut ritual_spell_cache = HashMap::new();
		let mut caster_ritual_list_cache = MultiMap::new();
		while let Some((entry, spell)) = query.next().await {
			for (caster_id, criteria) in &caster_filters {
				if criteria.is_relevant(&entry.metadata) {
					caster_ritual_list_cache.insert((*caster_id).clone(), spell.id.unversioned());
				}
			}
			ritual_spell_cache.insert(spell.id.unversioned(), spell);
		}

		Ok(RitualSpellCache {
			spells: ritual_spell_cache,
			caster_lists: caster_ritual_list_cache,
			casters_which_prepare_from_item,
		})
	}

	pub fn iter_ritual_spells(&self) -> impl Iterator<Item = (&String, &Spell, &SpellEntry)> + '_ {
		let iter = self.casters.iter();
		let iter = iter.filter_map(|(caster_id, caster)| {
			if self.ritual_spells.casters_which_prepare_from_item.contains(caster_id) {
				return None;
			}
			match self.ritual_spells.caster_lists.get_vec(caster_id) {
				Some(spell_ids) => Some((caster_id, &caster.spell_entry, spell_ids)),
				None => None,
			}
		});
		let iter = iter.map(|(caster_id, caster_spell_entry, spell_ids)| {
			let iter = spell_ids.iter();
			let iter = iter.filter_map(|spell_id| self.ritual_spells.spells.get(spell_id));
			iter.map(move |spell| (caster_id, spell, caster_spell_entry))
		});
		iter.flatten()
	}

	pub fn get_ritual_spell_for(&self, caster_id: &String, spell_id: &SourceId) -> Option<&Spell> {
		let Some(spell_ids) = self.ritual_spells.caster_lists.get_vec(caster_id) else {
			return None;
		};
		if !spell_ids.contains(spell_id) {
			return None;
		}
		self.ritual_spells.spells.get(spell_id)
	}

	pub fn cantrip_capacity(&self, persistent: &Persistent) -> Vec<(usize, &Restriction)> {
		let mut total_capacity = Vec::new();
		for (_id, caster) in &self.casters {
			let value = caster.cantrip_capacity(persistent);
			if value > 0 {
				total_capacity.push((value, &caster.restriction));
			}
		}
		total_capacity
	}

	/// Returns the spell slots the character has to cast from.
	/// If there are multiple caster features, the spell slots are determined from multiclassing rules.
	pub fn spell_slots(&self, character: &Character) -> Option<BTreeMap<u8, usize>> {
		// https://www.dndbeyond.com/sources/basic-rules/customization-options#MulticlassSpellcaster
		lazy_static::lazy_static! {
			static ref MULTICLASS_SLOTS: BTreeMap<usize, BTreeMap<u8, usize>> = BTreeMap::from([
				( 1, [ (1, 2) ].into()),
				( 2, [ (1, 3) ].into()),
				( 3, [ (1, 4), (2, 2) ].into()),
				( 4, [ (1, 4), (2, 3) ].into()),
				( 5, [ (1, 4), (2, 3), (3, 2) ].into()),
				( 6, [ (1, 4), (2, 3), (3, 3) ].into()),
				( 7, [ (1, 4), (2, 3), (3, 3), (4, 1) ].into()),
				( 8, [ (1, 4), (2, 3), (3, 3), (4, 2) ].into()),
				( 9, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 1) ].into()),
				(10, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2) ].into()),
				(11, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1) ].into()),
				(12, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1) ].into()),
				(13, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1), (7, 1) ].into()),
				(14, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1), (7, 1) ].into()),
				(15, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1), (7, 1), (8, 1) ].into()),
				(16, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1), (7, 1), (8, 1) ].into()),
				(17, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 2), (6, 1), (7, 1), (8, 1), (9, 1) ].into()),
				(18, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 3), (6, 1), (7, 1), (8, 1), (9, 1) ].into()),
				(19, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 3), (6, 2), (7, 1), (8, 1), (9, 1) ].into()),
				(20, [ (1, 4), (2, 3), (3, 3), (4, 3), (5, 3), (6, 2), (7, 2), (8, 1), (9, 1) ].into()),
			]);
		}

		if self.casters.is_empty() {
			return None;
		}

		if self.casters.len() == 1 {
			let (_id, caster) = self.casters.iter().next().unwrap();
			let current_level = character.level(Some(&caster.class_name));
			caster.all_slots().remove(&current_level)
		} else {
			let mut total_level = 0;
			for (_id, caster) in &self.casters {
				let current_level = character.level(Some(&caster.class_name));
				total_level += match &caster.standard_slots {
					Some(Slots::Standard { multiclass_half_caster: false, .. }) => current_level,
					Some(Slots::Standard { multiclass_half_caster: true, .. }) => current_level / 2,
					_ => 0,
				};
			}
			let mut slots = MULTICLASS_SLOTS.get(&total_level).cloned().unwrap_or_default();
			for (_id, caster) in &self.casters {
				let current_level = character.level(Some(&caster.class_name));
				for bonus_slots in &caster.bonus_slots {
					if let Some(ranks) = bonus_slots.capacity().get(&current_level) {
						slots.add_assign_map(ranks);
					}
				}
			}
			(!slots.is_empty()).then(|| slots)
		}
	}

	pub fn prepared_spells(&self) -> &HashMap<SourceId, AlwaysPreparedSpell> {
		&self.always_prepared
	}

	pub fn has_casters(&self) -> bool {
		!self.casters.is_empty()
	}

	pub fn get_caster(&self, id: &str) -> Option<&Caster> {
		self.casters.get(id)
	}

	pub fn iter_casters(&self) -> impl Iterator<Item = &Caster> {
		self.casters.iter().map(|(_id, caster)| caster)
	}

	pub fn get_ritual(&self, spell_id: &SourceId) -> Option<&Spell> {
		self.ritual_spells.spells.get(spell_id)
	}

	pub fn get_filter(&self, id: &str, persistent: &Persistent) -> Option<Filter> {
		let Some(caster) = self.get_caster(id) else {
			return None;
		};
		let additional_ids = match self.additional_caster_spells.by_caster.get_vec(id) {
			None => HashSet::new(),
			Some(entries) => entries.iter().cloned().collect::<HashSet<_>>(),
		};
		Some(Filter {
			tags: caster.restriction.tags.iter().cloned().collect(),
			rank_range: (None, caster.max_spell_rank(persistent)),
			additional_ids,
			..Default::default()
		})
	}
}
