use crate::{
	kdl_ext::NodeContext,
	system::dnd5e::data::{character::Character, item, ArmorExtended},
};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder};
use std::{collections::HashSet, str::FromStr};

/// Checks if the character has armor equipped.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct HasArmorEquipped {
	/// By default, this criteria checks if a piece of armor is equipped.
	/// If this flag is true, the criteria checks if no armor is equipped (or no armor of a particular set of types).
	pub inverted: bool,
	/// The list of armor types to check. If empty, all armor is considered.
	pub kinds: HashSet<ArmorExtended>,
}
impl HasArmorEquipped {
	fn kind_list(&self, joiner: &str) -> Option<String> {
		if self.kinds.is_empty() {
			return None;
		}
		let mut sorted_kinds = self.kinds.iter().collect::<Vec<_>>();
		sorted_kinds.sort();
		let kinds = sorted_kinds
			.into_iter()
			.map(|kind| match kind {
				ArmorExtended::Kind(kind) => format!("{kind:?}").to_lowercase(),
				ArmorExtended::Shield => "shield".into(),
			})
			.collect::<Vec<_>>();
		crate::utility::list_as_english(kinds, joiner)
	}
}

crate::impl_trait_eq!(HasArmorEquipped);
impl crate::system::Evaluator for HasArmorEquipped {
	type Context = Character;
	type Item = Result<(), String>;

	fn description(&self) -> Option<String> {
		Some(match (self.inverted, self.kind_list("or")) {
			(true, None) => format!("you have no armor equipped"),
			(true, Some(kind_desc)) => format!("you don't have {kind_desc} armor equipped"),
			(false, desc) => format!("you have {} armor equipped", desc.unwrap_or("any".into())),
		})
	}

	fn evaluate(&self, character: &Self::Context) -> Result<(), String> {
		for item::container::item::EquipableEntry { item, status, .. } in character.inventory().entries() {
			if !item.is_equipable() || !status.is_equipped() {
				continue;
			}
			let item::Kind::Equipment(equipment) = &item.kind else {
				continue;
			};
			if equipment.armor.is_none() && equipment.shield.is_none() {
				continue;
			}

			let mut in_filter = false;
			if let Some(armor) = &equipment.armor {
				in_filter = in_filter || self.kinds.contains(&ArmorExtended::Kind(armor.kind));
			}
			if equipment.shield.is_some() {
				in_filter = in_filter || self.kinds.contains(&ArmorExtended::Shield);
			}

			if self.kinds.is_empty() || in_filter {
				return match self.inverted {
					false => Ok(()),
					true => Err(format!("\"{}\" is equipped.", item.name)),
				};
			}
		}
		match self.inverted {
			false => Err(format!("No {}armor equipped", match self.kind_list("or") {
				None => String::new(),
				Some(kind_list) => format!("{kind_list} "),
			})),
			true => Ok(()),
		}
	}
}

kdlize::impl_kdl_node!(HasArmorEquipped, "has_armor_equipped");

impl FromKdl<NodeContext> for HasArmorEquipped {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let inverted = node.get_bool_opt("inverted")?.unwrap_or_default();
		let mut kinds = HashSet::new();
		for kind_str in node.query_str_all("scope() > kind", 0)? {
			kinds.insert(ArmorExtended::from_str(kind_str)?);
		}
		Ok(Self { inverted, kinds })
	}
}

impl AsKdl for HasArmorEquipped {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		if self.inverted {
			node.entry(("inverted", true));
		}
		for armor_ext in &self.kinds {
			node.child(("kind", armor_ext.to_string()));
		}
		node
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::system::{
		dnd5e::data::{
			character::Persistent,
			item::{
				armor::{self, Armor},
				equipment::Equipment,
				Item,
			},
		},
		Evaluator,
	};

	mod kdl {
		use super::*;
		use crate::{kdl_ext::test_utils::*, system::dnd5e::evaluator::test::test_utils};

		test_utils!(HasArmorEquipped);

		#[test]
		fn simple() -> anyhow::Result<()> {
			let doc = "evaluator \"has_armor_equipped\"";
			let data = HasArmorEquipped::default();
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}

		#[test]
		fn inverted() -> anyhow::Result<()> {
			let doc = "evaluator \"has_armor_equipped\" inverted=true";
			let data = HasArmorEquipped { inverted: true, ..Default::default() };
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}

		#[test]
		fn with_kinds() -> anyhow::Result<()> {
			let doc = "
				|evaluator \"has_armor_equipped\" {
				|    kind \"Light\"
				|}
			";
			let data =
				HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Light)].into(), ..Default::default() };
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}

		#[test]
		fn with_not_kinds() -> anyhow::Result<()> {
			let doc = "
				|evaluator \"has_armor_equipped\" inverted=true {
				|    kind \"Heavy\"
				|}
			";
			let data = HasArmorEquipped {
				inverted: true,
				kinds: [ArmorExtended::Kind(armor::Kind::Heavy)].into(),
				..Default::default()
			};
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}

		#[test]
		fn with_shield() -> anyhow::Result<()> {
			let doc = "
				|evaluator \"has_armor_equipped\" {
				|    kind \"Shield\"
				|}
			";
			let data = HasArmorEquipped { kinds: [ArmorExtended::Shield].into(), ..Default::default() };
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}
	}

	mod kind_list {
		use super::*;

		#[test]
		fn to_kindlist_0() {
			assert_eq!(HasArmorEquipped::default().kind_list("and"), None);
		}

		#[test]
		fn to_kindlist_1() {
			assert_eq!(
				HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Medium)].into(), ..Default::default() }
					.kind_list("and"),
				Some("medium".into())
			);
		}

		#[test]
		fn to_kindlist_2() {
			assert_eq!(
				HasArmorEquipped {
					kinds: [ArmorExtended::Kind(armor::Kind::Light), ArmorExtended::Kind(armor::Kind::Medium)].into(),
					..Default::default()
				}
				.kind_list("and"),
				Some("light and medium".into())
			);
		}

		#[test]
		fn to_kindlist_3plus() {
			assert_eq!(
				HasArmorEquipped {
					kinds: [
						ArmorExtended::Kind(armor::Kind::Light),
						ArmorExtended::Kind(armor::Kind::Medium),
						ArmorExtended::Kind(armor::Kind::Heavy),
						ArmorExtended::Shield,
					]
					.into(),
					..Default::default()
				}
				.kind_list("and"),
				Some("light, medium, heavy, and shield".into())
			);
		}
	}

	mod evaluate {
		use super::*;
		use item::container::item::{EquipStatus, ItemContainerTrait};

		fn character(kinds: &[(armor::Kind, EquipStatus)], shield: Option<EquipStatus>) -> Character {
			let mut persistent = Persistent::default();
			for (kind, equipped) in kinds {
				let id = persistent.inventory.insert(Item {
					name: format!("Armor{}", kind.to_string()),
					kind: item::Kind::Equipment(Equipment {
						armor: Some(Armor { kind: *kind, formula: Default::default(), min_strength_score: None }),
						..Default::default()
					}),
					..Default::default()
				});
				persistent.inventory.set_equipped(&id, *equipped);
			}
			if let Some(equipped) = shield {
				let id = persistent.inventory.insert(Item {
					name: format!("Shield"),
					kind: item::Kind::Equipment(Equipment { shield: Some(2), ..Default::default() }),
					..Default::default()
				});
				persistent.inventory.set_equipped(&id, equipped);
			}
			Character::from(persistent)
		}

		fn character_with_armor(kinds: &[(armor::Kind, EquipStatus)]) -> Character {
			character(kinds, None)
		}

		mod any {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator = HasArmorEquipped::default();
				let character = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&character), Err("No armor equipped".into()));
			}

			#[test]
			fn unequipped() {
				let evaluator = HasArmorEquipped::default();
				let with_medium = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Unequipped)]);
				assert_eq!(evaluator.evaluate(&with_medium), Err("No armor equipped".into()));
			}

			#[test]
			fn equipped() {
				let evaluator = HasArmorEquipped::default();
				let with_light = character_with_armor(&[(armor::Kind::Light, EquipStatus::Equipped)]);
				let with_medium = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Equipped)]);
				let with_heavy = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Ok(()));
				assert_eq!(evaluator.evaluate(&with_medium), Ok(()));
				assert_eq!(evaluator.evaluate(&with_heavy), Ok(()));
			}
		}

		mod single {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator =
					HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Light)].into(), ..Default::default() };
				let with_light = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light armor equipped".into()));
			}

			#[test]
			fn unequipped() {
				let evaluator =
					HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Light)].into(), ..Default::default() };
				let with_light = character_with_armor(&[(armor::Kind::Light, EquipStatus::Unequipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light armor equipped".into()));
			}

			#[test]
			fn wrong() {
				let evaluator =
					HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Light)].into(), ..Default::default() };
				let with_light = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light armor equipped".into()));
			}

			#[test]
			fn equipped() {
				let evaluator =
					HasArmorEquipped { kinds: [ArmorExtended::Kind(armor::Kind::Light)].into(), ..Default::default() };
				let with_light = character_with_armor(&[(armor::Kind::Light, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Ok(()));
			}
		}

		mod multiple {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator = HasArmorEquipped {
					kinds: [ArmorExtended::Kind(armor::Kind::Light), ArmorExtended::Kind(armor::Kind::Medium)].into(),
					..Default::default()
				};
				let with_light = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light or medium armor equipped".into()));
			}

			#[test]
			fn unequipped() {
				let evaluator = HasArmorEquipped {
					kinds: [ArmorExtended::Kind(armor::Kind::Light), ArmorExtended::Kind(armor::Kind::Medium)].into(),
					..Default::default()
				};
				let with_light = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Unequipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light or medium armor equipped".into()));
			}

			#[test]
			fn wrong() {
				let evaluator = HasArmorEquipped {
					kinds: [ArmorExtended::Kind(armor::Kind::Light), ArmorExtended::Kind(armor::Kind::Medium)].into(),
					..Default::default()
				};
				let with_light = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Err("No light or medium armor equipped".into()));
			}

			#[test]
			fn equipped() {
				let evaluator = HasArmorEquipped {
					kinds: [ArmorExtended::Kind(armor::Kind::Light), ArmorExtended::Kind(armor::Kind::Medium)].into(),
					..Default::default()
				};
				let with_light = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&with_light), Ok(()));
			}
		}

		mod none_allowed {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator = HasArmorEquipped { inverted: true, ..Default::default() };
				let character = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn unequipped() {
				let evaluator = HasArmorEquipped { inverted: true, ..Default::default() };
				let character = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Unequipped)]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn equipped() {
				let evaluator = HasArmorEquipped { inverted: true, ..Default::default() };
				let character = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&character), Err("\"ArmorHeavy\" is equipped.".into()));
			}

			#[test]
			fn shield_equipped() {
				let evaluator = HasArmorEquipped { inverted: true, ..Default::default() };
				let character = character(&[], Some(EquipStatus::Equipped));
				assert_eq!(evaluator.evaluate(&character), Err("\"Shield\" is equipped.".into()));
			}

			#[test]
			fn weapon_equipped() {
				let evaluator = HasArmorEquipped { inverted: true, ..Default::default() };
				let mut character = character(&[], None);
				let id = character.persistent_mut().inventory.insert(Item {
					name: format!("Staff"),
					kind: item::Kind::Equipment(Equipment {
						weapon: Some(item::weapon::Weapon {
							kind: item::weapon::Kind::Simple,
							classification: "Quarterstaff".into(),
							damage: None,
							properties: vec![],
							range: None,
							attack_roll_bonus: 0,
						}),
						..Default::default()
					}),
					..Default::default()
				});
				character.persistent_mut().inventory.set_equipped(&id, EquipStatus::Equipped);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}
		}

		mod no_single {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn unequipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Unequipped)]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn equipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[(armor::Kind::Heavy, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&character), Err("\"ArmorHeavy\" is equipped.".into()));
			}

			#[test]
			fn otherequipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}
		}

		mod no_multiple {
			use super::*;

			#[test]
			fn no_equipment() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Medium), ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn unequipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Medium), ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[
					(armor::Kind::Heavy, EquipStatus::Unequipped),
					(armor::Kind::Medium, EquipStatus::Unequipped),
				]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}

			#[test]
			fn equipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Medium), ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[(armor::Kind::Medium, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&character), Err("\"ArmorMedium\" is equipped.".into()));
			}

			#[test]
			fn otherequipped() {
				let evaluator = HasArmorEquipped {
					inverted: true,
					kinds: [ArmorExtended::Kind(armor::Kind::Medium), ArmorExtended::Kind(armor::Kind::Heavy)].into(),
					..Default::default()
				};
				let character = character_with_armor(&[(armor::Kind::Light, EquipStatus::Equipped)]);
				assert_eq!(evaluator.evaluate(&character), Ok(()));
			}
		}
	}
}
