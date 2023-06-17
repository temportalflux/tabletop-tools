use super::roll::EvaluatedRoll;
use crate::{
	kdl_ext::{AsKdl, DocumentExt, FromKDL, NodeBuilder, NodeExt},
	utility::InvalidEnumStr,
};
use enumset::EnumSetType;
use std::{path::PathBuf, str::FromStr};

#[derive(Clone, PartialEq, Default, Debug)]
pub struct DamageRoll {
	pub roll: Option<EvaluatedRoll>,
	pub base_bonus: i32,
	pub damage_type: DamageType,
	// generated (see BonusDamage mutator)
	pub additional_bonuses: Vec<(i32, PathBuf)>,
}

impl FromKDL for DamageRoll {
	fn from_kdl(
		node: &kdl::KdlNode,
		ctx: &mut crate::kdl_ext::NodeContext,
	) -> anyhow::Result<Self> {
		let roll = match node.query_opt("scope() > roll")? {
			None => None,
			Some(node) => Some(EvaluatedRoll::from_kdl(node, &mut ctx.next_node())?),
		};
		let base_bonus = node.get_i64_opt("base")?.unwrap_or(0) as i32;
		let damage_type = DamageType::from_str(node.query_str_req("scope() > damage_type", 0)?)?;
		Ok(Self {
			roll,
			base_bonus,
			damage_type,
			additional_bonuses: Vec::new(),
		})
	}
}
// TODO AsKdl: tests for DamageRoll
impl AsKdl for DamageRoll {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		if self.base_bonus != 0 {
			node.push_entry(self.base_bonus as i64);
		}
		if let Some(roll) = &self.roll {
			node.push_child_t("roll", roll);
		}
		node.push_child_entry_typed("damage_type", "DamageType", self.damage_type.to_string());
		node
	}
}

#[derive(Debug, Default, EnumSetType)]
pub enum DamageType {
	Acid,
	Bludgeoning,
	Cold,
	#[default]
	Fire,
	Force,
	Lightning,
	Necrotic,
	Piercing,
	Poison,
	Psychic,
	Radiant,
	Slashing,
	Thunder,
}

impl FromStr for DamageType {
	type Err = InvalidEnumStr<Self>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Acid" => Ok(Self::Acid),
			"Bludgeoning" => Ok(Self::Bludgeoning),
			"Cold" => Ok(Self::Cold),
			"Fire" => Ok(Self::Fire),
			"Force" => Ok(Self::Force),
			"Lightning" => Ok(Self::Lightning),
			"Necrotic" => Ok(Self::Necrotic),
			"Piercing" => Ok(Self::Piercing),
			"Poison" => Ok(Self::Poison),
			"Psychic" => Ok(Self::Psychic),
			"Radiant" => Ok(Self::Radiant),
			"Slashing" => Ok(Self::Slashing),
			"Thunder" => Ok(Self::Thunder),
			_ => Err(InvalidEnumStr::from(s).into()),
		}
	}
}

impl ToString for DamageType {
	fn to_string(&self) -> String {
		self.display_name().into()
	}
}

impl DamageType {
	pub fn display_name(&self) -> &'static str {
		match self {
			Self::Acid => "Acid",
			Self::Bludgeoning => "Bludgeoning",
			Self::Cold => "Cold",
			Self::Fire => "Fire",
			Self::Force => "Force",
			Self::Lightning => "Lightning",
			Self::Necrotic => "Necrotic",
			Self::Piercing => "Piercing",
			Self::Poison => "Poison",
			Self::Psychic => "Psychic",
			Self::Radiant => "Radiant",
			Self::Slashing => "Slashing",
			Self::Thunder => "Thunder",
		}
	}

	pub fn description(&self) -> &'static str {
		match self {
			Self::Acid => "The corrosive spray of an adult black dragon's breath and the dissolving \
			enzymes secreted by a black pudding deal acid damage.",
			Self::Bludgeoning => "Blunt force attacks--hammers, falling, constriction, \
			and the like--deal bludgeoning damage.",
			Self::Cold => "The infernal chill radiating from an ice devil's spear and the frigid blast \
			of a young white dragon's breath deal cold damage.",
			Self::Fire => "Ancient red dragons breathe fire, and many spells conjure flames to deal fire damage.",
			Self::Force => "Force is pure magical energy focused into a damaging form. \
			Most effects that deal force damage are spells, including magic missile and spiritual weapon.",
			Self::Lightning => "A lightning bolt spell and a blue dragon wyrmling's breath deal lightning damage.",
			Self::Necrotic => "Necrotic damage, dealt by certain undead and a spell such \
			as chill touch, withers matter and even the soul.",
			Self::Piercing => "Puncturing and impaling attacks, including spears and \
			monsters' bites, deal piercing damage.",
			Self::Poison => "Venomous stings and the toxic gas of an adult green dragon's breath deal poison damage.",
			Self::Psychic => "Mental abilities such as a psionic blast deal psychic damage.",
			Self::Radiant => "Radiant damage, dealt by a cleric's flame strike spell or an angel's \
			smiting weapon, sears the flesh like fire and overloads the spirit with power.",
			Self::Slashing => "Swords, axes, and monsters' claws deal slashing damage.",
			Self::Thunder => "A concussive burst of sound, such as the effect of the thunderwave spell, deals thunder damage.",
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod from_kdl {
		use super::*;
		use crate::{kdl_ext::NodeContext, system::dnd5e::data::roll::Die};

		fn from_doc(doc: &str) -> anyhow::Result<DamageRoll> {
			let document = doc.parse::<kdl::KdlDocument>()?;
			let node = document
				.query("scope() > damage")?
				.expect("missing damage node");
			DamageRoll::from_kdl(node, &mut NodeContext::default())
		}

		#[test]
		fn empty() -> anyhow::Result<()> {
			let doc = "damage {
				damage_type \"Force\"
			}";
			let expected = DamageRoll {
				roll: None,
				base_bonus: 0,
				damage_type: DamageType::Force,
				additional_bonuses: vec![],
			};
			assert_eq!(from_doc(doc)?, expected);
			Ok(())
		}

		#[test]
		fn flat_damage() -> anyhow::Result<()> {
			let doc = "damage base=5 {
				damage_type \"Force\"
			}";
			let expected = DamageRoll {
				roll: None,
				base_bonus: 5,
				damage_type: DamageType::Force,
				additional_bonuses: vec![],
			};
			assert_eq!(from_doc(doc)?, expected);
			Ok(())
		}

		#[test]
		fn roll_only() -> anyhow::Result<()> {
			let doc = "damage {
				roll (Roll)\"2d4\"
				damage_type \"Force\"
			}";
			let expected = DamageRoll {
				roll: Some(EvaluatedRoll::from((2, Die::D4))),
				base_bonus: 0,
				damage_type: DamageType::Force,
				additional_bonuses: vec![],
			};
			assert_eq!(from_doc(doc)?, expected);
			Ok(())
		}

		#[test]
		fn combined() -> anyhow::Result<()> {
			let doc = "damage base=2 {
				roll (Roll)\"1d6\"
				damage_type \"Bludgeoning\"
			}";
			let expected = DamageRoll {
				roll: Some(EvaluatedRoll::from((1, Die::D6))),
				base_bonus: 2,
				damage_type: DamageType::Bludgeoning,
				additional_bonuses: vec![],
			};
			assert_eq!(from_doc(doc)?, expected);
			Ok(())
		}
	}
}
