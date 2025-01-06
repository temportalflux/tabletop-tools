use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::{
				spellcasting::{AbilityOrStat, CastingMethod, SpellEntry},
				Character,
			},
			spell::CastingDuration,
			Indirect, Spell,
		},
		mutator::ReferencePath,
		SourceId,
	},
};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct SpellContainer {
	pub can_transcribe_from: bool,
	pub can_prepare_from: bool,
	pub capacity: Capacity,
	pub casting: Option<Casting>,
	pub spells: Vec<ContainerSpell>,
}

// Describes how many spells and of what types this container can hold.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Capacity {
	// How many individual spells can be contained.
	pub max_count: Option<usize>,
	// The minimum allowed rank of any individual spell.
	pub rank_min: Option<u8>,
	// The maximum allowed rank of any individual spell.
	pub rank_max: Option<u8>,
	// The max total value allowed when the ranks of all contained spells are summed.
	pub rank_total: Option<usize>,
}

// Describes the conditions under which all spells are cast, unless overriden on a given entry.
#[derive(Clone, PartialEq, Debug)]
pub struct Casting {
	// How long it takes to cast from this container.
	// If not provided, the duration is defined by the spell itself.
	pub duration: Option<CastingDuration>,
	// If casting tue last spell in the container will consume the item (destroy it).
	// If the spell is transcribed from this container and this property is enabled,
	// the item is destroyed (TODO: transcribing spells is not a feature yet).
	pub consume_item: bool,
	// If casting any spell from the container will consume the spell in the container.
	pub consume_spell: bool,
	// What the save DC is for a spell cast from the container.
	pub save_dc: Option<u8>,
	// What the attack bonus is for a spell cast from the container.
	pub attack_bonus: Option<i32>,
}

// A spell which is stored in a Spell Container.
#[derive(Clone, PartialEq, Debug)]
pub struct ContainerSpell {
	pub spell: Indirect<Spell>,
	// The overridden rank of the spell. If provided, the spell must be cast at this rank.
	pub rank: Option<u8>,
	// The spell save DC that must be used for this spell.
	pub save_dc: Option<u8>,
	// The spell attack bonus that must be used for this spell.
	pub attack_bonus: Option<i32>,
}
impl ContainerSpell {
	pub fn spell_id(&self) -> &SourceId {
		match &self.spell {
			Indirect::Id(id) => id,
			Indirect::Custom(spell) => &spell.id,
		}
	}
}

impl SpellContainer {
	pub fn remove(&mut self, spell_id: &SourceId) -> usize {
		let mut count_removed = 0usize;
		self.spells.retain_mut(|contained| {
			let contained_id = match &contained.spell {
				Indirect::Id(id) => id,
				Indirect::Custom(spell) => &spell.id,
			}
			.unversioned();
			match contained_id == *spell_id {
				// if not matching, then retain
				false => true,
				// if matching, discard and update count
				true => {
					count_removed += 1;
					false
				}
			}
		});
		count_removed
	}

	pub fn get_spell_entry(&self, contained: &ContainerSpell, default_values: Option<(i32, u8)>) -> Option<SpellEntry> {
		let Some(casting) = &self.casting else {
			return None;
		};
		let Some(atk_bonus) =
			casting.attack_bonus.or(contained.attack_bonus).or(default_values.map(|(bonus, _)| bonus))
		else {
			return None;
		};
		let Some(save_dc) = casting.save_dc.or(contained.save_dc).or(default_values.map(|(_, dc)| dc)) else {
			return None;
		};
		Some(SpellEntry {
			source: PathBuf::new(),
			classified_as: None,
			method: CastingMethod::FromContainer {
				item_id: Vec::new(),
				consume_spell: casting.consume_spell,
				consume_item: casting.consume_item,
			},
			attack_bonus: AbilityOrStat::Stat(atk_bonus),
			save_dc: AbilityOrStat::Stat(save_dc),
			// TODO: Should this also be an abilityorstat for the caster's original ability modifier?
			ability: None,
			casting_duration: casting.duration.clone(),
			rank: contained.rank,
			range: None,
		})
	}

	pub fn add_spellcasting(&self, stats: &mut Character, item_id: &Vec<uuid::Uuid>, parent: &ReferencePath) {
		for contained in &self.spells {
			let Some(mut entry) = self.get_spell_entry(contained, None) else {
				continue;
			};
			entry.source = parent.display.clone();
			if let CastingMethod::FromContainer { item_id: id_path, .. } = &mut entry.method {
				*id_path = item_id.clone();
			}
			match &contained.spell {
				Indirect::Id(id) => {
					stats.spellcasting_mut().add_prepared(&id, entry);
				}
				Indirect::Custom(spell) => {
					stats.spellcasting_mut().add_prepared_spell(spell, entry);
				}
			}
		}
	}
}

impl FromKdl<NodeContext> for SpellContainer {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let can_transcribe_from = node.get_bool_opt("transcribe")?.unwrap_or_default();
		let can_prepare_from = node.get_bool_opt("prepare_from")?.unwrap_or_default();
		let capacity = node.query_opt_t::<Capacity>("scope() > capacity")?.unwrap_or_default();
		let casting = node.query_opt_t::<Casting>("scope() > casting")?;
		let spells = node.query_all_t::<ContainerSpell>("scope() > spell")?;
		Ok(Self { can_transcribe_from, can_prepare_from, capacity, casting, spells })
	}
}
impl AsKdl for SpellContainer {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();

		if self.can_transcribe_from {
			node.entry(("transcribe", true));
		}
		if self.can_prepare_from {
			node.entry(("prepare_from", true));
		}

		node.child(("capacity", &self.capacity, OmitIfEmpty));
		node.child(("casting", &self.casting));
		node.children(("spell", self.spells.iter()));

		node
	}
}

impl FromKdl<NodeContext> for Capacity {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let max_count = node.next_i64_opt()?.map(|v| v as usize);
		let rank_min = node.query_i64_opt("scope() > rank", "min")?.map(|v| v as u8);
		let rank_max = node.query_i64_opt("scope() > rank", "max")?.map(|v| v as u8);
		let rank_total = node.query_i64_opt("scope() > rank", "total")?.map(|v| v as usize);
		Ok(Self { max_count, rank_min, rank_max, rank_total })
	}
}
impl AsKdl for Capacity {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();

		if let Some(num) = &self.max_count {
			node.entry(*num as i64);
		}

		node.child((
			{
				let mut node = NodeBuilder::default();
				if let Some(num) = &self.rank_min {
					node.entry(("min", *num as i64));
				}
				if let Some(num) = &self.rank_max {
					node.entry(("max", *num as i64));
				}
				if let Some(num) = &self.rank_total {
					node.entry(("total", *num as i64));
				}
				node.build("rank")
			},
			OmitIfEmpty,
		));

		node
	}
}

impl FromKdl<NodeContext> for Casting {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let duration = match node.peak_opt().is_some() {
			true => Some(CastingDuration::from_kdl(node)?),
			false => None,
		};
		let consume_item = node.query_bool_opt("scope() > consume_item", 0)?.unwrap_or_default();
		let consume_spell = node.query_bool_opt("scope() > consume_spell", 0)?.unwrap_or_default();
		let save_dc = node.query_i64_opt("scope() > save_dc", 0)?.map(|v| v as u8);
		let attack_bonus = node.query_i64_opt("scope() > atk_bonus", 0)?.map(|v| v as i32);
		Ok(Self { duration, consume_item, consume_spell, save_dc, attack_bonus })
	}
}
impl AsKdl for Casting {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();

		if let Some(kind) = &self.duration {
			node += kind.as_kdl();
		}

		if self.consume_item {
			node.child(("consume_item", true));
		}
		if self.consume_spell {
			node.child(("consume_spell", true));
		}

		node.child(("save_dc", &self.save_dc));
		node.child(("atk_bonus", &self.attack_bonus));

		node
	}
}

impl From<SourceId> for ContainerSpell {
	fn from(id: SourceId) -> Self {
		Self { spell: Indirect::Id(id), rank: None, save_dc: None, attack_bonus: None }
	}
}

impl FromKdl<NodeContext> for ContainerSpell {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let spell = Indirect::from_kdl(node)?;
		let rank = node.get_i64_opt("rank")?.map(|v| v as u8);
		let save_dc = node.get_i64_opt("save_dc")?.map(|v| v as u8);
		let attack_bonus = node.get_i64_opt("atk_bonus")?.map(|v| v as i32);
		Ok(Self { spell, rank, save_dc, attack_bonus })
	}
}
impl AsKdl for ContainerSpell {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = self.spell.as_kdl();

		if let Some(num) = &self.rank {
			node.entry(("rank", *num as i64));
		}

		if let Some(num) = &self.save_dc {
			node.entry(("save_dc", *num as i64));
		}
		if let Some(num) = &self.attack_bonus {
			node.entry(("atk_bonus", *num as i64));
		}

		node
	}
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use super::*;
	use crate::kdl_ext::test_utils::*;

	static NODE_NAME: &str = "spells";

	#[test]
	fn empty_spellbook() -> anyhow::Result<()> {
		let doc = "
			|spells prepare_from=true
		";
		let data = SpellContainer { can_prepare_from: true, ..Default::default() };
		assert_eq_fromkdl!(SpellContainer, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}

	#[test]
	fn looted_spellbook() -> anyhow::Result<()> {
		let doc = "
			|spells transcribe=true {
			|    spell \"local://basic-rules@dnd5e/spells/arcaneLock.kdl\"
			|    spell \"local://basic-rules@dnd5e/spells/glyphOfWarding.kdl\"
			|    spell \"local://basic-rules@dnd5e/spells/identify.kdl\"
			|    spell \"local://basic-rules@dnd5e/spells/falseLife.kdl\"
			|    spell \"local://basic-rules@dnd5e/spells/teleport.kdl\"
			|}
		";
		let data = SpellContainer {
			can_transcribe_from: true,
			spells: vec![
				ContainerSpell::from(SourceId::from_str("local://basic-rules@dnd5e/spells/arcaneLock.kdl")?),
				ContainerSpell::from(SourceId::from_str("local://basic-rules@dnd5e/spells/glyphOfWarding.kdl")?),
				ContainerSpell::from(SourceId::from_str("local://basic-rules@dnd5e/spells/identify.kdl")?),
				ContainerSpell::from(SourceId::from_str("local://basic-rules@dnd5e/spells/falseLife.kdl")?),
				ContainerSpell::from(SourceId::from_str("local://basic-rules@dnd5e/spells/teleport.kdl")?),
			],
			..Default::default()
		};
		assert_eq_fromkdl!(SpellContainer, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}

	#[test]
	fn container_with_capacity() -> anyhow::Result<()> {
		let doc = "
			|spells {
			|    capacity {
			|        rank min=1 max=5 total=5
			|    }
			|    spell \"local://basic-rules@dnd5e/spells/identify.kdl\" rank=2 save_dc=15 atk_bonus=2
			|}
		";
		let data = SpellContainer {
			capacity: Capacity { max_count: None, rank_min: Some(1), rank_max: Some(5), rank_total: Some(5) },
			spells: vec![ContainerSpell {
				spell: Indirect::Id(SourceId::from_str("local://basic-rules@dnd5e/spells/identify.kdl")?),
				rank: Some(2),
				save_dc: Some(15),
				attack_bonus: Some(2),
			}],
			..Default::default()
		};
		assert_eq_fromkdl!(SpellContainer, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}

	#[test]
	fn single_use() -> anyhow::Result<()> {
		let doc = "
			|spells {
			|    capacity 1 {
			|        rank min=3 max=3
			|    }
			|    casting {
			|        consume_item true
			|        save_dc 13
			|        atk_bonus 5
			|    }
			|    spell \"local://basic-rules@dnd5e/spells/fireball.kdl\"
			|}
		";
		let data = SpellContainer {
			capacity: Capacity { max_count: Some(1), rank_min: Some(3), rank_max: Some(3), rank_total: None },
			casting: Some(Casting {
				duration: None,
				consume_item: true,
				consume_spell: false,
				save_dc: Some(13),
				attack_bonus: Some(5),
			}),
			spells: vec![ContainerSpell {
				spell: Indirect::Id(SourceId::from_str("local://basic-rules@dnd5e/spells/fireball.kdl")?),
				rank: None,
				save_dc: None,
				attack_bonus: None,
			}],
			..Default::default()
		};
		assert_eq_fromkdl!(SpellContainer, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}

	#[test]
	fn refillable() -> anyhow::Result<()> {
		let doc = "
			|spells {
			|    capacity 1 {
			|        rank max=0
			|    }
			|    casting \"Action\" {
			|        consume_spell true
			|    }
			|}
		";
		let data = SpellContainer {
			capacity: Capacity { max_count: Some(1), rank_min: None, rank_max: Some(0), rank_total: None },
			casting: Some(Casting {
				duration: Some(CastingDuration::Action),
				consume_item: false,
				consume_spell: true,
				save_dc: None,
				attack_bonus: None,
			}),
			..Default::default()
		};
		assert_eq_fromkdl!(SpellContainer, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}
}
