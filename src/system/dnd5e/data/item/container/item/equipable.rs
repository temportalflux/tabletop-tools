use super::ItemPath;
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::Character,
			item::{container::item::AsItem, Item, Kind},
		},
		mutator::{self, ReferencePath},
	},
};
use derive_more::Display;
use enum_from_str::ParseEnumVariantError;
use enum_from_str_derive::FromStr;
use enumset::EnumSetType;
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub struct EquipableEntry {
	pub id_path: ItemPath,
	pub item: Item,
	pub status: EquipStatus,
}

#[derive(EnumSetType, Debug, Default, FromStr, Display)]
pub enum EquipStatus {
	#[default]
	#[display("Unequipped")]
	Unequipped,
	#[display("Equipped")]
	Equipped,
	#[display("Attuned")]
	Attuned,
}

impl EquipStatus {
	pub fn is_equipped(&self) -> bool {
		*self != Self::Unequipped
	}
}

impl EquipableEntry {
	pub fn id_as_path(&self) -> PathBuf {
		let iter = self.id_path.iter();
		iter.fold(PathBuf::new(), |path, id| path.join(id.to_string()))
	}
}

impl AsItem for EquipableEntry {
	fn from_item(item: Item) -> Self {
		Self { id_path: ItemPath::default(), item, status: EquipStatus::Unequipped }
	}

	fn set_id_path(&mut self, id: ItemPath) {
		self.item.set_id_path(id.clone());
		self.id_path = id;
	}

	fn id_path(&self) -> Option<&ItemPath> {
		Some(&self.id_path)
	}

	fn into_item(self) -> Item {
		self.item
	}

	fn as_item(&self) -> &Item {
		&self.item
	}

	fn as_item_mut(&mut self) -> &mut Item {
		&mut self.item
	}
}

impl mutator::Group for EquipableEntry {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		let item_name = PathBuf::from(&self.item.name);
		let path_to_item = parent.join(self.id_as_path(), Some(item_name));
		if let Kind::Equipment(equipment) = &self.item.kind {
			equipment.set_data_path(&path_to_item);
		}
	}

	fn apply_mutators(&self, stats: &mut Character, parent: &ReferencePath) {
		let Kind::Equipment(equipment) = &self.item.kind else {
			return;
		};
		if !self.status.is_equipped() {
			return;
		}

		let item_name = PathBuf::from(&self.item.name);
		let path_to_item = parent.join(self.id_as_path(), Some(item_name));
		stats.apply_from(equipment, &path_to_item);
		if let Some(weapon) = &equipment.weapon {
			stats.add_feature(weapon.attack_action(self), &path_to_item);
		}
		if let Some(spell_container) = &self.item.spells {
			spell_container.add_spellcasting(stats, &self.id_path, &path_to_item);
		}

		if let Some(armor) = &equipment.armor {
			use crate::{
				system::{
					dnd5e::{
						data::{action::AttackQuery, roll::Modifier, Ability, ArmorExtended},
						evaluator::HasProficiency,
						mutator::Modify,
					},
					Evaluator,
				},
				utility::selector::Value,
			};
			let is_proficient = HasProficiency::Armor(ArmorExtended::Kind(armor.kind)).evaluate(stats);
			// If equipped and not proficient, apply mutators due to wearing incompatible armor
			if !equipment.always_proficient && !is_proficient {
				// disadvantage on any ability check, saving throw, or attack roll that involves Strength or Dexterity, and you can’t cast spells
				stats.spellcasting_mut().can_cast_any = false;
				let mut mutators = Vec::with_capacity(7);
				for ability in [Ability::Strength, Ability::Dexterity] {
					// Disadvantage to ability & skill checks
					mutators.push(mutator::Generic::from(Modify::Ability {
						ability: Some(Value::Specific(ability)),
						modifier: Some(Modifier::Disadvantage),
						bonus: None,
						context: None,
					}));
					// Disadvantage to saving throws
					mutators.push(mutator::Generic::from(Modify::SavingThrow {
						ability: Some(Value::Specific(ability)),
						modifier: Some(Modifier::Disadvantage),
						bonus: None,
						context: None,
					}));
					// Disadvantage to attack rolls
					mutators.push(mutator::Generic::from(Modify::AttackRoll {
						modifier: Some(Modifier::Disadvantage),
						ability: None, // this adds the ability to the roll
						bonus: None,
						// we need to query for attacks which use the ability
						query: vec![AttackQuery { ability: [ability].into(), ..Default::default() }],
					}));
				}
				for mutator in mutators {
					stats.apply(&mutator, &path_to_item);
				}
			}
		}

		if self.status == EquipStatus::Attuned {
			let Some(attunement) = &equipment.attunement else { return };
			for modifier in &attunement.mutators {
				stats.apply(modifier, &path_to_item);
			}
		}
	}
}

impl FromKdl<NodeContext> for EquipableEntry {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let item = Item::from_kdl(node)?;
		let mut status = node.get_str_opt_t("status")?.unwrap_or_default();
		if node.get_bool_opt("equipped")? == Some(true) {
			status = EquipStatus::Equipped;
		}
		Ok(Self { id_path: ItemPath::default(), status, item })
	}
}

impl AsKdl for EquipableEntry {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = self.item.as_kdl();
		if self.status.is_equipped() {
			node.entry(("status", self.status.to_string()));
		}
		node
	}
}
