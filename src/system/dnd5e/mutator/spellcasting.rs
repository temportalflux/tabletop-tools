use crate::{
	kdl_ext::{DocumentExt, FromKDL, NodeExt},
	system::{
		core::SourceId,
		dnd5e::data::{
			action::LimitedUses,
			character::{
				spellcasting::{Caster, Restriction, Slots, SpellCapacity},
				Character,
			},
			description, Ability, spell,
		},
	},
	utility::{Mutator, NotInList},
};
use std::{collections::{BTreeMap, HashSet}, str::FromStr};

#[derive(Clone, Debug, PartialEq)]
pub struct Spellcasting {
	ability: Ability,
	operation: Operation,
}

crate::impl_trait_eq!(Spellcasting);
crate::impl_kdl_node!(Spellcasting, "spellcasting");

#[derive(Clone, Debug, PartialEq)]
enum Operation {
	Caster(Caster),
	/// Spells added to the list of spells that a caster can know or prepare.
	/// These DO count against the character's known/prepared spell capacity limits.
	AddSource,
	/// Spells that are always available to be cast, and
	/// DO NOT count against the character's known/prepared spell capacity limits.
	AddPrepared {
		/// The spells this feature provides, with any additional metadata.
		spells: Vec<PreparedSpell>,
		/// If provided, the specified spells are cast by using a specific usage criteria.
		/// If a provided spell also allows it to be cast through a slot, then both methods are valid.
		/// Otherwise, if both this is None and it cannot be cast via a slot, then the spell is cast at-will.
		limited_uses: Option<LimitedUses>,
	},
}
#[derive(Clone, Debug, PartialEq)]
struct PreparedSpell {
	/// The spell id that is prepared
	selector: SpellSelector,
	/// If the spell can be cast using a spell slot.
	/// If false, the spell is either cast At-Will or through a LimitedUses.
	can_cast_through_slot: bool,
	/// If present, the spell must be cast using the specified casting range.
	range: Option<spell::Range>,
	/// If present, the spell can only be cast at this rank using this feature.
	cast_at_rank: Option<u8>,
}
#[derive(Clone, Debug, PartialEq)]
enum SpellSelector {
	/// A specific spell id
	Specific(SourceId),
	/// A spell the user can select, which might be limited based on a provided filter.
	Any(Option<SpellFilter>),
}
#[derive(Clone, Debug, PartialEq)]
struct SpellFilter {
	// If provided, the selected spell must already be castable by the provided caster class.
	can_cast: Option<String>,
	/// The selected spell must be of one of these ranks.
	ranks: HashSet<u8>,
	/// The selected spell must have all of these tags
	tags: HashSet<String>,
}

impl Mutator for Spellcasting {
	type Target = Character;

	fn description(&self) -> description::Section {
		description::Section {
			title: Some("Spellcasting".into()),
			content: format!("{:?}", self),
			..Default::default()
		}
	}

	fn set_data_path(&self, parent: &std::path::Path) {
		match &self.operation {
			Operation::AddPrepared { spells: _, limited_uses: Some(limited_uses) } => {
				limited_uses.set_data_path(parent);
			}
			_ => {}
		}
	}

	fn apply(&self, stats: &mut Character, parent: &std::path::Path) {
		match &self.operation {
			Operation::Caster(caster) => {
				stats.spellcasting_mut().add_caster(caster.clone());
			}
			Operation::AddSource => {}
			Operation::AddPrepared { spells, limited_uses } => {
				/*
				stats.spellcasting_mut().add_prepared(
					spell_ids,
					self.ability,
					limited_uses.as_ref(),
					parent,
				);
				*/
			}
		}
	}
}

impl FromKDL for Spellcasting {
	fn from_kdl(
		node: &kdl::KdlNode,
		ctx: &mut crate::kdl_ext::NodeContext,
	) -> anyhow::Result<Self> {
		let ability = Ability::from_str(node.get_str_req("ability")?)?;
		let operation = match node.get_str_opt(ctx.consume_idx())? {
			None => {
				let class_name = node.get_str_req("class")?.to_owned();
				let restriction = {
					let node = node.query_req("scope() > restriction")?;
					let _ctx = ctx.next_node();
					let tags = node
						.query_str_all("scope() > tag", 0)?
						.into_iter()
						.map(str::to_owned)
						.collect::<Vec<_>>();
					Restriction { tags }
				};

				let cantrip_capacity = match node.query_opt("scope() > cantrips")? {
					None => None,
					Some(node) => {
						let ctx = ctx.next_node();

						let mut level_map = BTreeMap::new();
						for node in node.query_all("scope() > level")? {
							let mut ctx = ctx.next_node();
							let level = node.get_i64_req(ctx.consume_idx())? as usize;
							let capacity = node.get_i64_req(ctx.consume_idx())? as usize;
							level_map.insert(level, capacity);
						}

						Some(level_map)
					}
				};

				let slots =
					Slots::from_kdl(node.query_req("scope() > slots")?, &mut ctx.next_node())?;

				let spell_capacity = {
					let node = node.query_req("scope() > kind")?;
					let mut ctx = ctx.next_node();
					match node.get_str_req(ctx.consume_idx())? {
						"Prepared" => {
							let capacity = {
								let node = node.query_req("scope() > capacity")?;
								ctx.parse_evaluator::<Character, i32>(node)?
							};
							SpellCapacity::Prepared(capacity)
						}
						"Known" => {
							let capacity = {
								let node = node.query_req("scope() > capacity")?;
								let ctx = ctx.next_node();
								let mut capacity = BTreeMap::new();
								for node in node.query_all("scope() > level")? {
									let mut ctx = ctx.next_node();
									let level = node.get_i64_req(ctx.consume_idx())? as usize;
									let amount = node.get_i64_req(ctx.consume_idx())? as usize;
									capacity.insert(level, amount);
								}
								capacity
							};
							SpellCapacity::Known(capacity)
						}
						name => {
							return Err(NotInList(name.into(), vec!["Known", "Prepared"]).into());
						}
					}
				};

				Operation::Caster(Caster {
					class_name,
					ability,
					restriction,
					cantrip_capacity,
					slots,
					spell_capacity,
				})
			}
			Some("add_source") => {
				let mut spells = Vec::new();
				for s in node.query_str_all("scope() > spell", 0)? {
					spells.push(SourceId::from_str(s)?.with_basis(ctx.id()));
				}
				Operation::AddSource
			}
			Some("add_prepared") => {
				let mut spells = Vec::new();
				for node in node.query_all("scope() > spell")? {
					let mut ctx = ctx.next_node();
					let selector = match node.get_str_req(ctx.consume_idx())? {
						"Any" => {
							let filter = match node.query_opt("scope() > filter")? {
								None => None,
								Some(node) => {
									let can_cast = node.get_str_opt("can_cast")?.map(str::to_owned);
									let ranks = node.query_i64_all("scope() > rank", 0)?;
									let ranks = ranks.into_iter().map(|v| v as u8).collect::<HashSet<_>>();
									let tags = node.query_str_all("scope() > tag", 0)?;
									let tags = tags.into_iter().map(str::to_owned).collect::<HashSet<_>>();
									Some(SpellFilter { can_cast, ranks, tags })
								}
							};
							SpellSelector::Any(filter)
						}
						id_str => {
							SpellSelector::Specific(SourceId::from_str(id_str)?.with_basis(ctx.id()))
						}
					};
					let can_cast_through_slot = node.get_bool_opt("use_slot")?.unwrap_or_default();
					let cast_at_rank = node.get_i64_opt("rank")?.map(|v| v as u8);
					let range = match node.query_opt("scope() > range")? {
						None => None,
						Some(node) => Some(spell::Range::from_kdl(node, &mut ctx.next_node())?),
					};
					spells.push(PreparedSpell { selector, can_cast_through_slot, range, cast_at_rank });
				}
				let limited_uses = match node.query_opt("scope() > limited_use")? {
					None => None,
					Some(node) => Some(LimitedUses::from_kdl(node, &mut ctx.next_node())?),
				};
				Operation::AddPrepared { spells, limited_uses }
			}
			Some(name) => {
				return Err(NotInList(name.into(), vec!["add_source", "add_prepared"]).into())
			}
		};
		Ok(Self { ability, operation })
	}
}
