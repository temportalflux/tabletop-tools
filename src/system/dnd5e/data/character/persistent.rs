use super::{Conditions, HitPoints, SelectedSpells, Settings};
use crate::{
	kdl_ext::NodeContext,
	path_map::PathMap,
	system::{
		dnd5e::data::{character::Character, item::container::Inventory, Ability, Bundle, Class, Condition},
		mutator::{self, ReferencePath},
		Block, SourceId,
	},
};
use enum_map::EnumMap;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};
use multimap::MultiMap;
use serde::{Deserialize, Serialize};
use std::{path::Path, str::FromStr};

mod description;
pub use description::*;
mod notes;
pub use notes::*;

pub static MAX_SPELL_RANK: u8 = 9;

/// Core character data which is (de)serializable and
/// from which the derived data can be compiled.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct Persistent {
	pub id: SourceId,
	pub classes: Vec<Class>,
	pub bundles: Vec<Bundle>,
	pub description: Description,
	pub ability_scores: EnumMap<Ability, u32>,
	pub selected_values: PathMap<String>,
	pub selected_spells: SelectedSpells,
	pub inventory: Inventory,
	pub conditions: Conditions,
	pub hit_points: HitPoints,
	pub inspiration: bool,
	pub settings: Settings,
	pub notes: Notes,
	pub attunement_slots: u32,
}

kdlize::impl_kdl_node!(Persistent, "character");

impl mutator::Group for Persistent {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		for bundle in &self.bundles {
			bundle.set_data_path(parent);
		}
		for group in &self.classes {
			group.set_data_path(parent);
		}
		self.inventory.set_data_path(parent);
		self.conditions.set_data_path(parent);
		for (_die, selector) in &self.hit_points.hit_dice_selectors {
			selector.set_data_path(parent);
		}
	}

	fn apply_mutators(&self, stats: &mut Character, parent: &ReferencePath) {
		for (ability, score) in &self.ability_scores {
			stats.ability_scores_mut().push_bonus(ability, (*score).into(), "Base Score".into());
		}
		stats.apply(&super::FinalizeAbilityScores.into(), parent);
		{
			// Add the reset data for spell slots (shared by multiple classes when multiclassing).
			// Non-casters will still have this entry, but since they can't cast/don't have any slots,
			// there will be no slots that show up or actual data to reset.
			let (rest, entry) = self.selected_spells.reset_on_rest();
			stats.rest_resets_mut().add(rest, entry);
		}

		for bundle in &self.bundles {
			stats.apply_from(bundle, parent);
		}
		for class in &self.classes {
			stats.apply_from(class, parent);
		}
		stats.apply_from(&self.conditions, parent);
		stats.apply_from(&self.inventory, parent);
	}
}

impl Persistent {
	pub fn add_class(&mut self, class: Class) {
		self.classes.push(class);
	}

	pub fn level(&self, class_name: Option<&str>) -> usize {
		match class_name {
			Some(class_name) => {
				for class in &self.classes {
					if class.name == class_name {
						return class.current_level;
					}
				}
				return 0;
			}
			None => self.classes.iter().map(|class| class.current_level).sum(),
		}
	}

	pub fn hit_points(&self) -> &HitPoints {
		&self.hit_points
	}

	pub fn hit_points_mut(&mut self) -> &mut HitPoints {
		&mut self.hit_points
	}

	pub fn get_selections_at(&self, path: impl AsRef<Path>) -> Option<&Vec<String>> {
		self.selected_values.get(path.as_ref())
	}

	pub fn get_first_selection(&self, path: impl AsRef<Path>) -> Option<&String> {
		self.get_selections_at(path).map(|all| all.first()).flatten()
	}

	pub fn get_first_selection_at<T>(&self, data_path: impl AsRef<Path>) -> Option<Result<T, <T as FromStr>::Err>>
	where
		T: Clone + 'static + FromStr,
	{
		let selections = self.get_selections_at(data_path);
		selections.map(|all| all.first()).flatten().map(|selected| T::from_str(&selected))
	}

	pub fn set_selected_value(&mut self, key: impl AsRef<Path>, value: impl Into<String>) {
		self.selected_values.set(key, value.into());
	}

	pub fn set_selected(&mut self, key: impl AsRef<Path>, value: Option<String>) {
		match value {
			Some(value) => {
				self.selected_values.set(key, value);
			}
			None => {
				let _ = self.selected_values.remove(key);
			}
		}
	}

	pub fn insert_selection(&mut self, key: impl AsRef<Path>, value: impl Into<String>) {
		self.selected_values.insert(key, value.into());
	}

	pub fn remove_selection(&mut self, key: impl AsRef<Path>, index: usize) -> Option<String> {
		let Some(values) = self.selected_values.get_mut(key) else {
			return None;
		};
		if index < values.len() { Some(values.remove(index)) } else { None }
	}

	pub fn remove_selected_value(&mut self, key: impl AsRef<Path>, value: impl Into<String>) {
		let Some(values) = self.selected_values.get_mut(key) else {
			return;
		};
		let target: String = value.into();
		values.retain(|value| *value != target);
	}

	pub fn export_as_kdl(&self) -> kdl::KdlDocument {
		let mut doc = kdl::KdlDocument::new();
		doc.nodes_mut().push(self.as_kdl().build("character"));
		doc
	}
}

#[derive(PartialEq, Serialize, Deserialize)]
pub struct PersistentMetadata {
	pub name: String,
	pub pronouns: Vec<String>,
	pub level: usize,
	pub classes: Vec<String>,
	pub bundles: MultiMap<String, String>,
}

impl Block for Persistent {
	fn to_metadata(self) -> serde_json::Value {
		let mut level = 0;
		let mut classes = Vec::with_capacity(self.classes.len());
		for class in &self.classes {
			level += class.current_level;
			classes.push(class.name.clone());
		}
		let metadata = PersistentMetadata {
			name: self.description.name.clone(),
			pronouns: self.description.iter_pronouns().cloned().collect(),
			level,
			classes,
			bundles: self.bundles.iter().map(|bundle| (bundle.category.clone(), bundle.name.clone())).collect(),
		};
		serde_json::json!(metadata)
	}
}
impl FromKdl<NodeContext> for Persistent {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let id = node.context().id().clone();

		let description = node.query_req_t::<Description>("scope() > description")?;

		let mut settings = Settings::default();
		for node in &mut node.query_all("scope() > setting")? {
			settings.insert_from_kdl(node)?;
		}

		let mut ability_scores = EnumMap::default();
		for node in &mut node.query_all("scope() > ability")? {
			let ability = node.next_str_req_t::<Ability>()?;
			let score = node.next_i64_req()? as u32;
			ability_scores[ability] = score;
		}

		let hit_points = node.query_req_t::<HitPoints>("scope() > hit_points")?;

		let inspiration = node.query_bool_opt("scope() > inspiration", 0)?.unwrap_or_default();

		let mut conditions = Conditions::default();
		for condition in node.query_all_t::<Condition>("scope() > condition")? {
			conditions.insert(condition);
		}

		let inventory = node.query_opt_t::<Inventory>("scope() > inventory")?.unwrap_or_default();

		let selected_spells = node.query_opt_t::<SelectedSpells>("scope() > spells")?.unwrap_or_default();

		let bundles = node.query_all_t::<Bundle>("scope() > bundle")?;
		let classes = node.query_all_t::<Class>("scope() > class")?;

		let mut selected_values = PathMap::<String>::default();
		if let Some(selections) = node.query_opt("scope() > selections")? {
			for mut node in selections.query_all("scope() > value")? {
				let key_str = node.next_str_req()?;
				let value = node.next_str_req()?.to_owned();
				selected_values.insert(Path::new(key_str), value);
			}
		}

		let notes = node.query_opt_t("scope() > notes")?.unwrap_or_default();

		Ok(Self {
			id,
			description,
			settings,
			ability_scores,
			hit_points,
			inspiration,
			conditions,
			inventory,
			selected_spells,
			bundles,
			classes,
			selected_values,
			notes,
			attunement_slots: 3,
		})
	}
}
impl AsKdl for Persistent {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();

		node.child(("description", &self.description));
		self.settings.export_as_kdl(&mut node);

		for (ability, score) in self.ability_scores {
			node.child(
				NodeBuilder::default().with_entry(ability.long_name()).with_entry(score as i64).build("ability"),
			);
		}

		node.child(("hit_points", &self.hit_points));
		node.child(("inspiration", &self.inspiration));

		node.children(("condition", self.conditions.iter()));

		node.child(("inventory", &self.inventory, OmitIfEmpty));
		node.child(("spells", &self.selected_spells, OmitIfEmpty));

		node.children(("bundle", self.bundles.iter(), OmitIfEmpty));
		node.children(("class", self.classes.iter(), OmitIfEmpty));

		node.child((
			{
				let mut node = NodeBuilder::default();
				for (path, value) in self.selected_values.as_vec() {
					node.child(
						NodeBuilder::default()
							.with_entry({
								let path_str = path.display().to_string();
								path_str.replace("\\", "/")
							})
							.with_entry(value.clone())
							.build("value"),
					);
				}
				node.build("selections")
			},
			OmitIfEmpty,
		));

		node.child(("notes", &self.notes, OmitIfEmpty));

		node
	}
}
