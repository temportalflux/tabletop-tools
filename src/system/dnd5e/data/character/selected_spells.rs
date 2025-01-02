use super::{RestEntry, MAX_SPELL_RANK};
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{Rest, Spell},
		SourceId,
	},
};
use itertools::Itertools;
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

#[derive(Clone, PartialEq, Default, Debug)]
pub struct SelectedSpells {
	cache_by_caster: HashMap<String, SelectedSpellsData>,
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct SelectedSpellsData {
	/// The number of rank 0 spells selected.
	pub num_cantrips: usize,
	/// The number of spells selected whose rank is > 0.
	pub num_spells: usize,
	selections: HashMap<SourceId, Spell>,
}

impl FromKdl<NodeContext> for SelectedSpells {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let mut cache_by_caster = HashMap::new();
		for node in &mut node.query_all("scope() > caster")? {
			let caster_name = node.next_str_req()?;
			let mut selection_data = SelectedSpellsData::default();
			for mut node in &mut node.query_all("scope() > spell")? {
				let spell = Spell::from_kdl(&mut node)?;
				selection_data.insert(spell);
			}
			cache_by_caster.insert(caster_name.to_owned(), selection_data);
		}

		Ok(Self { cache_by_caster })
	}
}

impl AsKdl for SelectedSpells {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		// Casters
		let iter_casters = self.cache_by_caster.iter();
		let iter_casters = iter_casters.sorted_by_key(|(name, _)| *name);
		for (caster_name, selected_spells) in iter_casters {
			if selected_spells.selections.is_empty() {
				continue;
			}
			let mut node_caster = NodeBuilder::default();

			node_caster.entry(caster_name.clone());

			let iter_spells = selected_spells.selections.values();
			let iter_spells = iter_spells.sorted_by(|a, b| a.rank.cmp(&b.rank).then(a.name.cmp(&b.name)));
			node_caster.children(("spell", iter_spells));

			node.child(node_caster.build("caster"));
		}
		node
	}
}

impl SelectedSpells {
	pub fn insert(&mut self, caster_id: &impl AsRef<str>, spell: Spell) {
		let selected_spells = match self.cache_by_caster.get_mut(caster_id.as_ref()) {
			Some(existing) => existing,
			None => {
				self.cache_by_caster.insert(caster_id.as_ref().to_owned(), SelectedSpellsData::default());
				self.cache_by_caster.get_mut(caster_id.as_ref()).unwrap()
			}
		};
		selected_spells.insert(spell);
	}

	pub fn remove(&mut self, caster_id: &impl AsRef<str>, spell_id: &SourceId) {
		let Some(caster_list) = self.cache_by_caster.get_mut(caster_id.as_ref()) else {
			return;
		};
		caster_list.remove(spell_id);
	}

	pub fn get(&self, caster_id: &impl AsRef<str>) -> Option<&SelectedSpellsData> {
		self.cache_by_caster.get(caster_id.as_ref())
	}

	pub fn get_spell(&self, caster_id: &impl AsRef<str>, spell_id: &SourceId) -> Option<&Spell> {
		let Some(data) = self.cache_by_caster.get(caster_id.as_ref()) else {
			return None;
		};
		let Some(spell) = data.selections.get(spell_id) else {
			return None;
		};
		Some(spell)
	}

	pub fn iter_caster_ids(&self) -> impl Iterator<Item = &String> {
		self.cache_by_caster.keys()
	}

	pub fn iter_caster(&self, caster_id: &impl AsRef<str>) -> Option<impl Iterator<Item = &Spell>> {
		let Some(caster) = self.cache_by_caster.get(caster_id.as_ref()) else {
			return None;
		};
		Some(caster.selections.values())
	}

	pub fn iter_selected(&self) -> impl Iterator<Item = (/*caster id*/ &String, /*spell id*/ &SourceId, &Spell)> {
		let iter = self.cache_by_caster.iter();
		let iter = iter.map(|(caster_id, selected_per_caster)| {
			let iter = selected_per_caster.selections.iter();
			iter.map(|(spell_id, spell)| (&*caster_id, spell_id, spell))
		});
		iter.flatten()
	}

	pub fn has_selected(&self, caster_id: &impl AsRef<str>, spell_id: &SourceId) -> bool {
		let Some(data) = self.cache_by_caster.get(caster_id.as_ref()) else {
			return false;
		};
		data.selections.contains_key(spell_id)
	}

	pub fn consumed_slots_path(&self, rank: u8) -> std::path::PathBuf {
		Path::new("SpellSlots").join(rank.to_string())
	}

	pub fn reset_on_rest(&self) -> (Rest, RestEntry) {
		let data_paths =
			(1..=MAX_SPELL_RANK).into_iter().map(|rank| self.consumed_slots_path(rank)).collect::<Vec<_>>();
		let entry =
			RestEntry { restore_amount: None, data_paths, source: PathBuf::from("Standard Spellcasting Slots") };
		(Rest::Long, entry)
	}
}

impl SelectedSpellsData {
	fn insert(&mut self, spell: Spell) {
		match spell.rank {
			0 => self.num_cantrips += 1,
			_ => self.num_spells += 1,
		}
		self.selections.insert(spell.id.unversioned(), spell);
	}

	fn remove(&mut self, id: &SourceId) {
		if let Some(spell) = self.selections.remove(id) {
			match spell.rank {
				0 => self.num_cantrips -= 1,
				_ => self.num_spells -= 1,
			}
		}
	}

	pub fn len(&self) -> usize {
		self.selections.len()
	}
}
