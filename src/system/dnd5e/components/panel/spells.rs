use crate::{
	components::{context_menu, database::use_typed_fetch_callback, stop_propagation, Spinner},
	database::{entry::EntryInSystemWithType, Criteria, Database, Entry, Query},
	page::characters::sheet::{
		joined::editor::{CollapsableCard, DescriptionSection},
		CharacterHandle,
	},
	system::{
		self,
		dnd5e::{
			components::{
				glyph::{DamageTypeGlyph, Glyph},
				panel::NotesField,
			},
			data::{
				character::{
					spellcasting::{AbilityOrStat, CasterKind, CastingMethod, RitualCapability, SpellEntry},
					ObjectCacheArc, SpellHealingBonus, MAX_SPELL_RANK,
				},
				item::container::item::ItemPath,
				proficiency,
				roll::Roll,
				spell::{self, CastingDuration, DurationKind},
				AreaOfEffect, Indirect, Spell,
			},
			DnD5e,
		},
		ModuleId, SourceId,
	},
	utility::InputExt,
};
use convert_case::{Case, Casing};
use itertools::Itertools;
use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
	sync::Arc,
};
use yew::prelude::*;

fn rank_suffix(rank: u8) -> &'static str {
	match rank {
		1 => "st",
		2 => "nd",
		3 => "rd",
		4..=9 => "th",
		_ => "",
	}
}

#[function_component]
pub fn Spells() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let mut sections = SpellSections::default();
	sections.insert_slots(&state);
	sections.insert_selected_spells(&state);
	sections.insert_permaprepared_spells(&state);
	sections.insert_available_ritual_spells(&state);

	let sections = {
		let mut html = Vec::new();
		for (rank, section_props) in sections.sections {
			if section_props.spells.is_empty() && (section_props.slot_count.is_none() || rank == 0) {
				continue;
			}
			html.push(spell_section(&state, rank, section_props));
		}
		html! {<>{html}</>}
	};

	let feature_stats = state.spellcasting().has_casters().then(|| {
		use unzip_n::unzip_n;
		unzip_n!(4);
		let (names, modifier, atk_bonus, save_dc) = state
			.spellcasting()
			.iter_casters()
			.map(|caster| {
				let name = caster.name().clone();
				let modifier = state.ability_modifier(caster.ability, None);
				let atk_bonus = state.ability_modifier(caster.ability, Some(proficiency::Level::Full));
				let save_dc = 8 + atk_bonus;
				(name, modifier, atk_bonus, save_dc)
			})
			.unzip_n_vec();
		let names = names.into_iter().sorted().map(|caster_id| {
			html! {
				<ManageCasterButton {caster_id} />
			}
		});
		let names = Itertools::intersperse(names, html! { <span class="mx-1">{"|"}</span> }).collect::<Vec<_>>();
		let modifier = modifier.into_iter().map(|v| format!("{v:+}")).join(" | ");
		let atk_bonus = atk_bonus.into_iter().map(|v| format!("{v:+}")).join(" | ");
		let save_dc = save_dc.into_iter().map(|v| format!("{v}")).join(" | ");
		html! {
			<div class="caster-stats">
				<div class="names">{names}</div>
				<div class="row">
					<div class="col mod">
						<div class="stats">{modifier}</div>
						<div class="title">{"Modifier"}</div>
					</div>
					<div class="col atk">
						<div class="stats">{atk_bonus}</div>
						<div class="title">{"Spell Attack"}</div>
					</div>
					<div class="col dc">
						<div class="stats">{save_dc}</div>
						<div class="title">{"Save DC"}</div>
					</div>
				</div>
			</div>
		}
	});

	html! {
		<div class="panel spells">
			{feature_stats.unwrap_or_default()}

			<div class="input-group search my-2">
				<span class="input-group-text"><i class="bi bi-search"/></span>
				<input
					type="text" class="form-control"
					placeholder="Search spell names, tags, or other properties"
				/>
			</div>

			<div class="sections">
				{sections}
			</div>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct CasterNameProps {
	caster_id: AttrValue,
}
#[function_component]
fn ManageCasterButton(CasterNameProps { caster_id }: &CasterNameProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();
	let open_browser = Callback::from({
		let state = state.clone();
		let context_menu = context_menu.clone();
		let caster_id = caster_id.clone();
		move |_| {
			let Some(caster) = state.spellcasting().get_caster(caster_id.as_str()) else {
				return;
			};
			context_menu.dispatch(context_menu::Action::open_root(
				format!("{} Spellcasting", caster.name()),
				html!(<ManageCasterModal caster_id={caster_id.clone()} />),
			));
		}
	});
	html! {
		<button type="button" class="btn btn-xs btn-outline-theme mb-1" onclick={open_browser}>
			{caster_id.clone()}
		</button>
	}
}

struct SpellSections<'c> {
	sections: BTreeMap<u8, SectionProps<'c>>,
}
impl<'c> Default for SpellSections<'c> {
	fn default() -> Self {
		Self {
			sections: (0..=MAX_SPELL_RANK).map(|slot: u8| (slot, SectionProps::default())).collect::<BTreeMap<_, _>>(),
		}
	}
}
impl<'c> SpellSections<'c> {
	fn insert_slots(&mut self, state: &'c CharacterHandle) {
		if let Some(slots) = state.spellcasting().spell_slots(&*state) {
			for (rank, slot_count) in slots {
				let section = self.sections.get_mut(&rank).expect(&format!(
					"Spell rank {rank} is not supported by UI, must be in the range of [0, {}].",
					MAX_SPELL_RANK
				));
				let data_path = state.persistent().selected_spells.consumed_slots_path(rank);
				let consumed_slots = state.get_first_selection_at::<usize>(&data_path);
				let consumed_slots = consumed_slots.map(Result::ok).flatten().unwrap_or_default();
				section.slot_count = Some((consumed_slots, slot_count));
			}
		}
	}

	fn insert_spell(&mut self, spell: &'c Spell, entry: &'c SpellEntry, location: SpellLocation) {
		let ranks = match (entry.rank, &entry.method) {
			(None, CastingMethod::Cast { can_use_slots: true, .. }) => {
				let max_rank = match spell.rank {
					0 => 0,
					_ => MAX_SPELL_RANK,
				};
				let rank_range = spell.rank..=max_rank;
				let mut locations = Vec::with_capacity(rank_range.len());
				locations.resize(rank_range.len(), location);
				rank_range.zip(locations).collect::<Vec<_>>()
			}
			(Some(rank), _) => vec![(rank, location)],
			(None, _) => vec![(spell.rank, location)],
		};
		for (section_rank, location) in ranks {
			let Some(section) = self.sections.get_mut(&section_rank) else {
				continue;
			};
			let can_cast_from_section = section_rank == 0 || section.slot_count.is_some();
			let uses_cast_method = matches!(&entry.method, CastingMethod::Cast { .. });
			if can_cast_from_section || !uses_cast_method {
				section.insert_spell(spell, entry, location);
			}
		}
	}

	fn insert_selected_spells(&mut self, state: &'c CharacterHandle) {
		for caster_id in state.persistent().selected_spells.iter_caster_ids() {
			let Some(caster) = state.spellcasting().get_caster(caster_id) else {
				continue;
			};
			let Some(iter_spells) = state.persistent().selected_spells.iter_caster(caster_id) else {
				continue;
			};
			for spell in iter_spells {
				self.insert_spell(spell, &caster.spell_entry, SpellLocation::Selected {
					caster_id: caster_id.clone(),
					spell_id: spell.id.unversioned(),
				});
			}
		}
	}

	fn insert_permaprepared_spells(&mut self, state: &'c CharacterHandle) {
		for (_id, spell_entry) in state.spellcasting().prepared_spells() {
			let Some(spell) = &spell_entry.spell else {
				continue;
			};
			for (source, entry) in &spell_entry.entries {
				self.insert_spell(spell, entry, SpellLocation::AlwaysPrepared {
					spell_id: spell.id.unversioned(),
					source: source.clone(),
				});
				self.insert_as_ritual(spell, entry, RitualSpellSource::AlwaysPrepared(source.clone()));
			}
		}
	}

	fn insert_available_ritual_spells(&mut self, state: &'c CharacterHandle) {
		for (caster_id, spell, spell_entry) in state.spellcasting().iter_ritual_spells() {
			self.insert_as_ritual(spell, spell_entry, RitualSpellSource::Caster(caster_id.clone()));
		}

		// for casters which prepare from items, rituals are based on those in the items
		let mut prepare_from_item_casters = Vec::new();
		for caster in state.spellcasting().iter_casters() {
			let Some(ritual_capability) = &caster.ritual_capability else {
				continue;
			};
			if !ritual_capability.available_spells {
				continue;
			}
			if !caster.prepare_from_item {
				continue;
			}
			prepare_from_item_casters.push((caster.name(), &caster.spell_entry));
		}
		for equipable in state.inventory().entries() {
			// gather all spells from containers which are equipped and can be prepared from
			if !equipable.status.is_equipped() {
				continue;
			}
			let Some(spell_container) = &equipable.item.spells else {
				continue;
			};
			if !spell_container.can_prepare_from {
				continue;
			}
			for contained_spell in &spell_container.spells {
				for (caster_id, spell_entry) in &prepare_from_item_casters {
					let spell_id = contained_spell.spell_id().unversioned();
					if let Some(spell) = state.spellcasting().get_ritual_spell_for(caster_id, &spell_id) {
						self.insert_as_ritual(spell, spell_entry, RitualSpellSource::Caster((*caster_id).clone()));
					}
				}
			}
		}
	}

	fn insert_as_ritual(&mut self, spell: &'c Spell, spell_entry: &'c SpellEntry, source: RitualSpellSource) {
		match &spell_entry.method {
			// ritual casting is allowed
			CastingMethod::Cast { can_use_ritual: true, .. } => {}
			// other casting, or any other method is not considered a ritual
			_ => return,
		}
		// ritual-only spells can only be cast at their specified rank
		let Some(section) = self.sections.get_mut(&spell.rank) else {
			return;
		};
		section.insert_spell(spell, spell_entry, SpellLocation::AvailableAsRitual {
			spell_id: spell.id.unversioned(),
			source,
		});
	}
}

#[derive(Default)]
struct SectionProps<'c> {
	slot_count: Option<(/*consumed*/ usize, /*max*/ usize)>,
	spells: Vec<SectionSpell<'c>>,
}
struct SectionSpell<'c> {
	spell: &'c Spell,
	entry: &'c SpellEntry,
	location: SpellLocation,
}
#[derive(Clone, PartialEq, Debug)]
enum SpellLocation {
	Selected { spell_id: SourceId, caster_id: String },
	AlwaysPrepared { spell_id: SourceId, source: std::path::PathBuf },
	AvailableAsRitual { spell_id: SourceId, source: RitualSpellSource },
}
#[derive(Clone, PartialEq, Debug)]
enum RitualSpellSource {
	Caster(String),
	AlwaysPrepared(std::path::PathBuf),
}
impl SpellLocation {
	fn get<'this, 'c>(&'this self, state: &'c CharacterHandle) -> Option<(&'c Spell, &'c SpellEntry)> {
		match self {
			SpellLocation::Selected { caster_id, spell_id } => {
				let Some(caster) = state.spellcasting().get_caster(caster_id) else {
					return None;
				};
				let Some(spell) = state.persistent().selected_spells.get_spell(caster_id, spell_id) else {
					return None;
				};
				Some((spell, &caster.spell_entry))
			}
			SpellLocation::AlwaysPrepared { spell_id, source } => {
				let Some(spell_entry) = state.spellcasting().prepared_spells().get(spell_id) else {
					return None;
				};
				let Some(spell) = &spell_entry.spell else {
					return None;
				};
				let Some(entry) = spell_entry.entries.get(source) else {
					return None;
				};
				Some((spell, entry))
			}
			SpellLocation::AvailableAsRitual { spell_id, source: RitualSpellSource::Caster(caster_id) } => {
				let Some(caster) = state.spellcasting().get_caster(caster_id) else {
					return None;
				};
				let Some(spell) = state.spellcasting().get_ritual(spell_id) else {
					return None;
				};
				Some((spell, &caster.spell_entry))
			}
			SpellLocation::AvailableAsRitual { spell_id, source: RitualSpellSource::AlwaysPrepared(source) } => {
				let Some(spell_entry) = state.spellcasting().prepared_spells().get(spell_id) else {
					return None;
				};
				let Some(spell) = &spell_entry.spell else {
					return None;
				};
				let Some(entry) = spell_entry.entries.get(source) else {
					return None;
				};
				Some((spell, entry))
			}
		}
	}
}
impl<'c> SectionProps<'c> {
	pub fn insert_spell(&mut self, spell: &'c Spell, entry: &'c SpellEntry, location: SpellLocation) {
		let idx = self.spells.binary_search_by(|row| {
			let spell_name = row.spell.name.cmp(&spell.name);
			let source = row.entry.source.cmp(&entry.source);
			spell_name.then(source)
		});
		let idx = idx.unwrap_or_else(|e| e);
		self.spells.insert(idx, SectionSpell { spell, entry, location });
	}
}

fn spell_section<'c>(state: &'c CharacterHandle, rank: u8, section_props: SectionProps<'c>) -> Html {
	let suffix = rank_suffix(rank);
	let rank_text = match rank {
		0 => "cantrip",
		_ => "level",
	};
	let title = match rank {
		0 => rank_text.to_case(Case::Upper),
		rank => format!("{rank}{}", format!("{suffix} {rank_text}").to_case(Case::Upper)),
	};
	let inline_rank_text = format!("{rank}{suffix}-{rank_text}");
	let empty_note = format!(
		"You do not have any {inline_rank_text} spells or spells that scale to \
		{inline_rank_text} available, but you can cast lower level spells \
		using your {inline_rank_text} spell slots."
	);

	let slots = section_props.slot_count.as_ref().map(|(consumed, count)| {
		// consumed ; amount of slots that have been used
		// count ; quantity of slots at the rank
		// available = count - consumed ; amount of slots that can yet be used
		let toggle_slot = state.dispatch_change({
			let available = *count - *consumed;
			move |evt: web_sys::Event| {
				use system::dnd5e::change::ConsumeSpellSlot;
				let Some(consume_slot) = evt.input_checked() else { return None };
				let slots_remaining = available.saturating_add_signed(match consume_slot {
					true => -1,
					false => 1,
				});
				// Sets the number of slots remaining at a given rank. Quantity remaining represents the total number
				// of slots that should be available to the character after the change, and should always be
				// <= the total number of slots at the given rank.
				Some(ConsumeSpellSlot::manual(rank, slots_remaining))
			}
		});
		html! {
			<div class="slots">
				{(0..*count).map(|idx| html! {
					<input
					type="checkbox"
						class={"form-check-input slot"}
						checked={idx < *consumed}
						onchange={toggle_slot.clone()}
					/>
				}).collect::<Vec<_>>()}
				{(*count > 0).then_some(html!(<span class="ms-1">{"SLOTS"}</span>)).unwrap_or_default()}
			</div>
		}
	});

	let contents = match section_props.spells.is_empty() {
		true => html! { <p class="empty-note mx-4">{empty_note}</p> },
		false => {
			html! {<>
				{section_props.spells.into_iter().map(|section_spell| {
					spell_row(SpellRowProps {
						state,
						section_rank: rank,
						slots: section_props.slot_count,
						section_spell,
					})
				}).collect::<Vec<_>>()}
			</>}
		}
	};
	html! {
		<div class="spell-section mb-2">
			<div class="header">
				<div class="title">{title}</div>
				{slots.unwrap_or_default()}
			</div>
			{contents}
		</div>
	}
}

struct SpellRowProps<'c> {
	state: &'c CharacterHandle,
	section_rank: u8,
	slots: Option<(usize, usize)>,
	section_spell: SectionSpell<'c>,
}
fn spell_row<'c>(props: SpellRowProps<'c>) -> Html {
	let SpellRowProps { state, section_rank, slots, section_spell: SectionSpell { spell, entry, location } } = props;

	let (use_kind, src_text_suffix) = match &location {
		SpellLocation::AvailableAsRitual { .. } => (UseSpell::RitualOnly, None),
		_ => match &entry.method {
			CastingMethod::AtWill => (UseSpell::AtWill, None),
			CastingMethod::Cast { can_use_slots: true, .. } if spell.rank == 0 => (UseSpell::AtWill, None),
			CastingMethod::Cast { can_use_slots: true, .. } => {
				let slot = UseSpell::Slot {
					spell_rank: spell.rank,
					slot_rank: section_rank,
					slots: slots.clone(),
					id: spell.id.unversioned(),
				};
				(slot, None)
			}
			CastingMethod::LimitedUses(limited_uses) => {
				let data_path = limited_uses.get_uses_path(state);
				let max_uses = limited_uses.get_max_uses(state) as u32;
				let uses_consumed = limited_uses.get_uses_consumed(state);
				let kind = UseSpell::LimitedUse { data_path, max_uses, uses_consumed };
				let text = html! {
					<span class="ms-1">
						{format!(
							"({}/{max_uses}{})",
							max_uses.saturating_sub(uses_consumed),
							limited_uses.get_reset_rest(state).map(|rest| {
								format!(" per {} rest", rest.to_string())
							}).unwrap_or_default()
						)}
					</span>
				};
				(kind, Some(text))
			}
			CastingMethod::FromContainer { item_id, consume_spell, consume_item } => {
				let use_spell = match *consume_spell {
					false => UseSpell::AtWill,
					true => UseSpell::ConsumeItemSpell {
						item_path: item_id.clone(),
						spell_id: spell.id.unversioned(),
						consume_item: *consume_item,
					},
				};
				(use_spell, None)
			}
			CastingMethod::Cast { can_use_slots: false, .. } => return Html::default(),
		},
	};

	// TODO: tooltip for casting time duration
	// TODO: Tooltips for ritual & concentration icons
	let use_ritual_only = use_kind == UseSpell::RitualOnly;
	html! {
		<SpellModalRowRoot {location}>
			<div class="feature-row border-bottom">
				<div class="column cast-button" onclick={stop_propagation()}>
					<UseSpellButton kind={use_kind} />
				</div>
				<div class="column name-and-source">
					{spell_name_and_icons(&state, spell, Some(entry), use_ritual_only)}
					{spell_source_and_uses(&entry.source, src_text_suffix)}
				</div>
				{spell_overview_info(&state, spell, Some(entry), Some(section_rank))}
			</div>
		</SpellModalRowRoot>
	}
}

pub fn spell_name_and_icons(
	state: &CharacterHandle, spell: &Spell, entry: Option<&SpellEntry>, ritual_only: bool,
) -> Html {
	let can_ritual_cast = spell.casting_time.ritual && {
		ritual_only || {
			let classified = entry.as_ref().map(|entry| entry.classified_as.as_ref()).flatten();
			let caster = classified.map(|id| state.spellcasting().get_caster(id)).flatten();
			let ritual_casting = caster.map(|caster| caster.ritual_capability.as_ref()).flatten();
			let ritual_cast_selected = ritual_casting.map(|ritual| ritual.selected_spells).unwrap_or_default();
			ritual_cast_selected
		}
	};
	html! {
		<div class="name-row">
			{&spell.name}
			{can_ritual_cast.then(|| html!(
				<Glyph tag="div" classes={"ritual ms-1"} />
			)).unwrap_or_default()}
			{spell.duration.concentration.then(|| html!(
				<Glyph tag="div" classes={"concentration ms-1"} />
			)).unwrap_or_default()}
		</div>
	}
}

pub fn spell_source_and_uses(source: &Path, uses_suffix: Option<Html>) -> Html {
	html! {
		<div class="label source-row">
			{crate::data::as_feature_path_text(source)}
			{uses_suffix.unwrap_or_default()}
		</div>
	}
}

pub fn spell_overview_info(
	state: &CharacterHandle, spell: &Spell, entry: Option<&SpellEntry>, override_rank: Option<u8>,
) -> Html {
	let casting_duration =
		entry.map(|entry| entry.casting_duration.as_ref()).flatten().unwrap_or(&spell.casting_time.duration);
	let mut range = entry.map(|entry| entry.range.as_ref()).flatten().unwrap_or(&spell.range).clone();
	let attack_bonus = entry.map(|entry| entry.attack_bonus).unwrap_or(AbilityOrStat::Stat(0));
	let save_dc = entry.map(|entry| entry.save_dc).unwrap_or(AbilityOrStat::Stat(0));
	let cast_at_rank = override_rank.or(entry.map(|entry| entry.rank).flatten());
	let spellcasting_modifier = entry
		.map(|entry| entry.ability)
		.flatten()
		.map(|ability| state.ability_modifier(ability, None))
		.unwrap_or_default();

	if let Some(minimum_range) = state.attack_bonuses().get_spell_range_minimum(spell) {
		if let spell::Range::Unit { distance, unit } = &mut range {
			if minimum_range as u64 > *distance {
				*distance = minimum_range as u64;
				*unit = format!("feet");
			}
		}
	}

	let notes = state.persistent().notes.get_first(&spell.id.path);
	let notes = notes.map(|content| html!(<div class="attribute">{content}</div>));

	html! {
		<div class="column attributes fixed-width">
			<div class="attribute-row">
				<span class="attribute casting-time">
					<span class="label">{"Cast:"}</span>
					{match casting_duration {
						CastingDuration::Action => html!("Action"),
						CastingDuration::Bonus => html!("Bonus Action"),
						CastingDuration::Reaction(_trigger) => html!("Reaction"),
						CastingDuration::Unit(amt, kind) => html!(format!("{amt} {kind}")),
					}}
				</span>
				{match &spell.duration.kind {
					DurationKind::Special => html!(),
					DurationKind::Instantaneous => html! {
						<span class="attribute duration">
							<span class="label">{"Duration:"}</span>
							{"Instant"}
						</span>
					},
					DurationKind::Unit(amt, kind) => html! {
						<span class="attribute duration">
							<span class="label">{"Duration:"}</span>
							{amt}{" "}{kind}
						</span>
					},
				}}
				<span class="attribute range">
					<span class="label">{"Range:"}</span>
					{match range {
						spell::Range::OnlySelf => html!("Self"),
						spell::Range::Touch => html!("Touch"),
						spell::Range::Unit { distance, unit } => html! {<>{distance}{" "}{unit}</>},
						spell::Range::Sight => html!("Sight"),
						spell::Range::Unlimited => html!("Unlimited"),
					}}
				</span>
			</div>
			<div class="attribute-row">
				{match &spell.check {
					None => html!(),
					Some(spell::Check::AttackRoll(_atk_kind)) => {
						let modifier = match attack_bonus {
							AbilityOrStat::Stat(modifier) => modifier,
							AbilityOrStat::Ability(ability) => state.ability_modifier(ability, Some(proficiency::Level::Full)),
						};
						html! {<span class="attribute atk-roll">
							<span class="label">{"Atk Roll:"}</span>
							{format!("{modifier:+}")}
						</span>}
					}
					Some(spell::Check::SavingThrow(ability, fixed_dc)) => {
						let abb_name = ability.abbreviated_name().to_case(Case::Upper);
						let dc = match save_dc {
							AbilityOrStat::Stat(dc) => dc as i32,
							AbilityOrStat::Ability(ability) => match fixed_dc {
								Some(dc) => *dc as i32,
								None => {
									let modifier = state.ability_modifier(ability, Some(proficiency::Level::Full));
									8 + modifier
								}
							},
						};
						html! {<span class="attribute save-dc">
							<span class="label">{"Save DC:"}</span>
							{format!("{abb_name} {dc}")}
						</span>}
					}
				}}
				{match &spell.damage {
					None => html!(),
					Some(damage) => {
						// The difference in rank between the slot being used and the minimum rank of the spell
						let upcast_amt = cast_at_rank.map(|rank| rank - spell.rank).unwrap_or(0);
						// Evaluate the rolls for the damage set
						let mut roll_set = damage.evaluate(&*state, spellcasting_modifier, upcast_amt as u32);
						// Evaluate any extra bonuses granted by modifiers
						for (bonus_dmg_roll, _source) in state.attack_bonuses().get_spell_damage(&spell) {
							roll_set.push(*bonus_dmg_roll);
						}
						// Convert roll set to html elements
						let spans = roll_set.rolls().into_iter().enumerate().map(|(idx, roll)| {
							html! {
								<span>
									{(idx != 0 && roll.min() > 0).then(|| html!("+")).unwrap_or_default()}
									{roll.to_string()}
								</span>
							}
						}).collect::<Vec<_>>();
						html! {<span class="attribute damage">
							<span class="label">{"Damage:"}</span>
							{spans}
							<DamageTypeGlyph value={damage.damage_type()} />
						</span>}
					}
				}}
				{match &spell.healing {
					None => html!(),
					Some(healing) => {
						// The difference in rank between the slot being used and the minimum rank of the spell
						let upcast_amt = cast_at_rank.map(|rank| rank - spell.rank).unwrap_or(0) as i32;
						// Evaluate the rolls for the healing set
						let mut roll_set = healing.evaluate(spellcasting_modifier, upcast_amt);

						// Some spells provide healing as a pool which is assigned to multiple creatures,
						// but the bonuses are per creature. So those spells indicate that the prefered UX is to not conflate the two sources in the overview.
						// We still show the breakdown in the details view of the spell.
						if !healing.hide_bonuses_in_overview {
							// Append derved data bonuses to the roll set for healing
							let rank = cast_at_rank.unwrap_or(spell.rank) as i32;
							for (bonus, _source) in state.attack_bonuses().get_spell_healing_bonuses(&spell) {
								match bonus {
									SpellHealingBonus::Roll(bonus_rolls) => roll_set += *bonus_rolls,
									SpellHealingBonus::RankScale(coefficient) => roll_set += Roll::from(coefficient * rank),
								}
							}
						}

						// Convert roll set to html elements
						let spans = roll_set.rolls().into_iter().enumerate().map(|(idx, roll)| {
							html! {
								<span>
									{(idx != 0 && roll.min() > 0).then(|| html!("+")).unwrap_or_default()}
									{roll.to_string()}
								</span>
							}
						}).collect::<Vec<_>>();
						html! {<span class="attribute healing">
							<span class="label">{"Healing:"}</span>
							{spans}
							<Glyph tag="span" classes={"damage_type healing"} />
						</span>}
					}
				}}
				{match &spell.area_of_effect {
					None => html!(),
					Some(area_of_effect) => html! {
						<span class="attribute area-of-effect">
							<span class="label">{"Area of Effect:"}</span>
							{match area_of_effect {
								AreaOfEffect::Cone { length } => html!{<>{length}{"ft. Cone"}</>},
								AreaOfEffect::Cube { size } => html!{<>{size}{"ft. Cube"}</>},
								AreaOfEffect::Cylinder { radius, height } => html!{<>{radius}{"ft. x "}{height}{"ft. Cylinder"}</>},
								AreaOfEffect::Line { width, length } => html!{<>{width}{"ft. x "}{length}{"ft. Line"}</>},
								AreaOfEffect::Sphere { radius } => html!{<>{radius}{"ft. Sphere"}</>},
							}}
						</span>
					}
				}}
			</div>
			{notes}
		</div>
	}
}

#[function_component]
fn SpellModalRowRoot(SpellModalProps { location, children }: &SpellModalProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();
	let open_browser = Callback::from({
		let state = state.clone();
		let context_menu = context_menu.clone();
		let location = location.clone();
		move |_| {
			let Some((spell, _entry)) = location.get(&state) else {
				return;
			};
			let location = location.clone();
			context_menu
				.dispatch(context_menu::Action::open_root(spell.name.clone(), html!(<SpellModal {location} />)));
		}
	});
	html! {
		<div onclick={open_browser}>
			{children.clone()}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct SpellModalProps {
	location: SpellLocation,
	#[prop_or_default]
	children: Children,
}
#[function_component]
fn SpellModal(SpellModalProps { location, children: _ }: &SpellModalProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some((spell, entry)) = location.get(&state) else {
		log::warn!("Invalid spell at location {location:?}");
		return Html::default();
	};

	html! {<div class="w-100 h-100 scroll-container-y">
		<div class="modal-header">
			<h1 class="modal-title fs-4">{&spell.name}</h1>
			<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
		</div>
		<div class="modal-body">
			{spell_content(spell, Some(entry), &state)}
		</div>
	</div>}
}

#[function_component]
fn ManageCasterModal(CasterNameProps { caster_id }: &CasterNameProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(caster) = state.spellcasting().get_caster(caster_id.as_str()) else {
		return html! {<>
			<div class="modal-header">
				<h1 class="modal-title fs-4">{"Not Found: "}{caster_id}</h1>
				<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
			</div>
			<div class="modal-body">
				{"No spellcasting data found."}
			</div>
		</>};
	};

	let max_cantrips = caster.cantrip_capacity(state.persistent());
	let max_spells = caster.spell_capacity(&state);
	let filter = state.spellcasting().get_filter(caster.name(), state.persistent()).unwrap_or_default();

	let mut num_cantrips = 0;
	let mut num_spells = 0;
	let mut num_all_selections = 0;
	if let Some(selections) = state.persistent().selected_spells.get(caster.name()) {
		num_cantrips = selections.num_cantrips;
		num_spells = selections.num_spells;
		num_all_selections = selections.len();
	}
	let mut selected_spells = Vec::with_capacity(num_all_selections);
	if let Some(iter_selected) = state.persistent().selected_spells.iter_caster(caster.name()) {
		for spell in iter_selected {
			// Insertion sort by rank & name
			let order_idx = selected_spells
				.binary_search_by(|existing_spell: &&Spell| {
					existing_spell.rank.cmp(&spell.rank).then(existing_spell.name.cmp(&spell.name))
				})
				.unwrap_or_else(|err_idx| err_idx);
			selected_spells.insert(order_idx, spell);
		}
	}
	let caster_info = ActionCasterInfo { id: caster_id.clone(), max_cantrips, max_spells };

	// TODO: Display modifier/atk bonus/save dc and how they are calculated for this caster.
	// TODO: Display restriction info for the caster's spell list.
	// TODO: Display rules for when spells can be selected or swapped out.
	html! {<div class="w-100 h-100 scroll-container-y">
		<div class="details spells browse">
			<div>
				<CollapsableCard
					id={"selected-spells"}
					header_content={{html! { {"Selected Spells"} }}}
					body_classes={"spell-list selected"}
				>
					{selected_spells.into_iter().map(|spell| {
						let action = html! {<SpellListAction
							section={SpellListSection::Selected}
							caster={caster_info.clone()}
							spell_id={spell.id.unversioned()}
							rank={spell.rank}
						/>};
						spell_list_item("selected", &state, spell, Some(&caster.spell_entry), action)
					}).collect::<Vec<_>>()}
				</CollapsableCard>
				<CollapsableCard
					id={"available-spells"}
					header_content={{html! { {"Available Spells"} }}}
					body_classes={"spell-list available"}
				>
					<AvailableSpellList
						header_addon={HeaderAddon::from({
							let caster_info = caster_info.clone();
							move |spell: &Spell| -> Html {
								html! {
									<SpellListAction
										caster={caster_info.clone()}
										section={SpellListSection::Available}
										spell_id={spell.id.unversioned()}
										rank={spell.rank}
									/>
								}
							}
						})}
						criteria={filter.as_criteria()}
						entry={caster.spell_entry.clone()}
						source={match caster.prepare_from_item {
							false => SpellSource::Database,
							true => SpellSource::Items,
						}}
					/>
				</CollapsableCard>
			</div>
			<div class="footer">
				<SpellCapacity name={"Cantrips"} num={num_cantrips} max={max_cantrips} />
				<SpellCapacity name={"Spells"} num={num_spells} max={max_spells} />
			</div>
		</div>
	</div>}
}

#[derive(Clone, PartialEq, Properties)]
struct SpellCapacityProps {
	name: AttrValue,
	num: usize,
	max: usize,
}
#[function_component]
fn SpellCapacity(SpellCapacityProps { name, num, max }: &SpellCapacityProps) -> Html {
	let mut classes = classes!("alert");
	if num < max {
		classes.push("alert-warning");
	}
	html! {
		<div class={classes} role="alert">
			{name}{": "}{num}{" / "}{max}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct SpellListActionProps {
	caster: ActionCasterInfo,
	section: SpellListSection,
	spell_id: SourceId,
	rank: u8,
}
#[derive(Clone, PartialEq)]
struct ActionCasterInfo {
	id: AttrValue,
	max_cantrips: usize,
	max_spells: usize,
}
#[derive(Clone, PartialEq)]
enum SpellListSection {
	Selected,
	Available,
}
#[function_component]
fn SpellListAction(SpellListActionProps { caster: info, section, spell_id, rank }: &SpellListActionProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(caster) = state.spellcasting().get_caster(info.id.as_str()) else {
		return Html::default();
	};

	let mut can_select_more = true;
	if let Some(selections) = state.persistent().selected_spells.get(&info.id) {
		can_select_more = match rank {
			0 => selections.num_cantrips < info.max_cantrips,
			_ => selections.num_spells < info.max_spells,
		};
	}

	let is_selected = state.persistent().selected_spells.has_selected(&info.id, &spell_id);

	let mut classes = classes!("btn", "btn-xs", "select");
	let mut disabled = false;
	match section {
		SpellListSection::Selected => {
			classes.push("btn-outline-theme");
		}
		SpellListSection::Available => match is_selected {
			true => {
				classes.push("btn-theme");
			}
			false => match can_select_more {
				true => {
					classes.push("btn-outline-theme");
				}
				false => {
					classes.push("btn-outline-secondary");
					disabled = true;
				}
			},
		},
	}

	let action_name = match (caster.kind(), rank, is_selected) {
		(_, 0, false) => "Learn",
		(_, 0, true) => "Delete",
		(CasterKind::Known, _, false) => "Learn",
		(CasterKind::Known, _, true) => "Delete",
		(CasterKind::Prepared, _, false) => "Prepare",
		(CasterKind::Prepared, _, true) => match section {
			SpellListSection::Selected => "Unprepare",
			SpellListSection::Available => "Prepared",
		},
	};

	let select_spell = use_typed_fetch_callback(
		"Select Spell".into(),
		state.dispatch_change({
			let caster_id = info.id.clone();
			move |spell: Spell| Some(system::dnd5e::change::PrepareSpell::Add { caster: caster_id.to_string(), spell })
		}),
	);
	let deselect_spell = state.dispatch_change({
		let caster_id = info.id.clone();
		move |spell_id: SourceId| {
			Some(system::dnd5e::change::PrepareSpell::Remove { caster: caster_id.to_string(), spell_id })
		}
	});
	let onclick = Callback::from({
		let spell_id = spell_id.clone();
		move |evt: MouseEvent| {
			evt.stop_propagation();
			let target = match is_selected {
				true => &deselect_spell,
				false => &select_spell,
			};
			target.emit(spell_id.clone());
		}
	});
	let onclick = (!disabled).then_some(onclick);

	html! {
		<button type="button" class={classes} {disabled} {onclick}>{action_name}</button>
	}
}

fn spell_list_item(
	section_id: &str, state: &CharacterHandle, spell: &Spell, entry: Option<&SpellEntry>, action: Html,
) -> Html {
	let collapse_id = format!("{section_id}-{}", spell.id.ref_id());
	let can_ritual_cast = spell.casting_time.ritual && {
		let classified = entry.map(|entry| entry.classified_as.as_ref()).flatten();
		let caster = classified.map(|id| state.spellcasting().get_caster(id)).flatten();
		let ritual_casting = caster.map(|caster| caster.ritual_capability.as_ref()).flatten();
		match ritual_casting {
			None => false,
			Some(RitualCapability { selected_spells, available_spells }) => *available_spells || *selected_spells,
		}
	};
	// TODO: Tooltips for ritual & concentration icons
	html! {
		<div class="spell mb-1">
			<div class="header mb-1">
				<button
					role="button" class={"collapse_trigger arrow_left collapsed"}
					data-bs-toggle="collapse"
					data-bs-target={format!("#{collapse_id}")}
				>
					{spell.name.clone()}
					{can_ritual_cast.then(|| html!(
						<Glyph tag="div" classes={"ritual ms-1 my-auto"} />
					)).unwrap_or_default()}
					{spell.duration.concentration.then(|| html!(
						<Glyph tag="div" classes={"concentration ms-1 my-auto"} />
					)).unwrap_or_default()}
					<span class="spell_rank_suffix">
						{"("}
						{match spell.rank {
							0 => "Cantrip".into(),
							n => format!("{n}{}", rank_suffix(n))
						}}
						{")"}
					</span>
				</button>
				{action}
			</div>
			<div class="collapse mb-2" id={collapse_id}>
				<div class="card">
					<div class="card-body px-2 py-1">
						{spell_content(&spell, entry, state)}
					</div>
				</div>
			</div>
		</div>
	}
}

fn spell_content(spell: &Spell, entry: Option<&SpellEntry>, state: &CharacterHandle) -> Html {
	use crate::components::{Tag, Tags};
	let mut sections = Vec::new();
	sections.push(html! {
		<div class="property">
			<strong>{"Rank:"}</strong>
			{match spell.rank {
				0 => html! { "Cantrip" },
				n => html! {<>{n}{rank_suffix(n)}{" Level"}</>},
			}}
		</div>
	});
	if let Some(school) = &spell.school_tag {
		sections.push(html! {
			<div class="property">
				<strong>{"School:"}</strong>
				{school}
			</div>
		});
	}
	sections.push(html! {
		<div class="property">
			<strong>{"Casting Time:"}</strong>
			{match &spell.casting_time.duration {
				CastingDuration::Action => format!("1 action"),
				CastingDuration::Bonus => format!("1 bonus action"),
				CastingDuration::Reaction(None) => format!("1 reaction"),
				CastingDuration::Reaction(Some(trigger)) => format!("1 reaction ({trigger})"),
				CastingDuration::Unit(amt, kind) => format!("{amt} {kind}"),
			}}
			{spell.casting_time.ritual.then(|| html! { {" (ritual)"} }).unwrap_or_default()}
		</div>
	});

	let range = entry.map(|entry| entry.range.as_ref()).flatten();
	let mut range = range.unwrap_or(&spell.range).clone();
	if let Some(minimum_range) = state.attack_bonuses().get_spell_range_minimum(spell) {
		if let spell::Range::Unit { distance, unit } = &mut range {
			if minimum_range as u64 > *distance {
				*distance = minimum_range as u64;
				*unit = format!("feet");
			}
		}
	}
	sections.push(html! {
		<div class="property">
			<strong>{"Range:"}</strong>
			{match range {
				spell::Range::OnlySelf => html!("Self"),
				spell::Range::Touch => html!("Touch"),
				spell::Range::Unit { distance, unit } => html! {<>{distance}{" "}{unit}</>},
				spell::Range::Sight => html!("Sight"),
				spell::Range::Unlimited => html!("Unlimited"),
			}}
		</div>
	});
	if let Some(area_of_effect) = &spell.area_of_effect {
		sections.push(html! {
			<div class="property">
				<strong>{"Area of Effect:"}</strong>
				{match area_of_effect {
					AreaOfEffect::Cone { length } => html!{<>{length}{"ft. Cone"}</>},
					AreaOfEffect::Cube { size } => html!{<>{size}{"ft. Cube"}</>},
					AreaOfEffect::Cylinder { radius, height } => html!{<>{radius}{"ft. x "}{height}{"ft. Cylinder"}</>},
					AreaOfEffect::Line { width, length } => html!{<>{width}{"ft. x "}{length}{"ft. Line"}</>},
					AreaOfEffect::Sphere { radius } => html!{<>{radius}{"ft. Sphere"}</>},
				}}
			</div>
		});
	}
	sections.push(html! {
		<div class="property">
			<strong>{"Duration:"}</strong>
			{match &spell.duration.kind {
				DurationKind::Instantaneous => html!("Instantaneous"),
				DurationKind::Unit(amt, kind) => html! {<>{amt}{" "}{kind}</>},
				DurationKind::Special => html!("Special"),
			}}
			{spell.duration.concentration.then(|| html!(" (requires concentration)")).unwrap_or_default()}
		</div>
	});
	if let Some(healing) = &spell.healing {
		// TODO: Grab bonuses from derived data
		let base_rolls = healing
			.rolls()
			.iter_rolls()
			.enumerate()
			.map(|(idx, roll)| {
				html! {
					<span>
						{(idx > 0).then_some(html!(" "))}
						{match (idx, roll.min() >= 0) {
							(0, true) => html!(),
							(_, positive) => if positive { html!("+ ") } else { html!("- ") },
						}}
						{roll.abs().to_string()}
					</span>
				}
			})
			.collect::<Vec<_>>();
		let upcast_rolls = healing
			.upcast()
			.iter_rolls()
			.enumerate()
			.map(|(idx, roll)| {
				html! {
					<span>
						{(idx > 0).then_some(html!(" "))}
						{match (idx, roll.min() >= 0) {
							(0, true) => html!(),
							(_, positive) => if positive { html!("+ ") } else { html!("- ") },
						}}
						{roll.abs().to_string()}
					</span>
				}
			})
			.collect::<Vec<_>>();
		let spellcasting_ability = entry.map(|entry| entry.ability).flatten();
		let ability_modifier = match healing.uses_ability_modifier() {
			false => html!(),
			true => match spellcasting_ability {
				None => html!(" + your spellcasting ability modifier"),
				Some(ability) => {
					let modifier = state.ability_modifier(ability, None);
					html!(<span>
						{if modifier >= 0 { " + " } else { " - " }}{modifier.abs()}
						{format!(" ({} modifier)", ability.long_name())}
					</span>)
				}
			},
		};

		let bonus_rolls = {
			let iter = state.attack_bonuses().get_spell_healing_bonuses(&spell);
			let iter = iter.enumerate();
			let iter = iter.map(|(idx, (bonus, source))| {
				let bonus_span = match bonus {
					SpellHealingBonus::Roll(bonus_rolls) => {
						let roll_set_str = bonus_rolls.to_string();
						let prefix = match (idx, roll_set_str.starts_with('-')) {
							(0, _) | (_, true) => html!(),
							_ => html!("+ "),
						};
						html!(<span>{prefix}{roll_set_str}</span>)
					}
					SpellHealingBonus::RankScale(coefficient) => {
						let prefix = match (idx, *coefficient < 0) {
							(0, _) | (_, true) => html!(),
							_ => html!("+ "),
						};
						html!(<span>{prefix}{*coefficient}{" * casting rank"}</span>)
					}
				};
				html! {
					<span>
						{(idx > 0).then_some(html!(" "))}
						{bonus_span}
						<span class="label">{" ("}{crate::data::as_feature_path_text(&source)}{")"}</span>
					</span>
				}
			});
			iter.collect::<Vec<_>>()
		};

		sections.push(html!(<>
			<div class="property">
				<strong>{"Healing:"}</strong>
				<span>{base_rolls}{ability_modifier}</span>
			</div>
			<div class="property">
				<strong>{"Upcast Bonus Healing:"}</strong>
				{upcast_rolls}
			</div>
			<div class="property">
				<strong>{"Bonus Healing per Target Creature:"}</strong>
				{bonus_rolls}
			</div>
		</>));
	}
	if !spell.tags.is_empty() {
		sections.push(html! {
			<div class="property d-inline-flex">
				<strong>{"Tags:"}</strong>
				<Tags>
					{spell.tags.iter().map(|tag| html! {
						<Tag>{tag.to_string()}</Tag>
					}).collect::<Vec<_>>()}
				</Tags>
			</div>
		});
	}

	if let Some(module) = &spell.id.module {
		sections.push(html! {
			<div class="property">
				<strong>{"Source:"}</strong>
				{match module {
					ModuleId::Local { name } => format!("{name} (local)"),
					ModuleId::Github { user_org, repository } => format!("{user_org}:{repository} (github)"),
				}}
			</div>
		});
	}
	if let Some(version) = &spell.id.version {
		sections.push(html! {
			<div class="property">
				<strong>{"Version:"}</strong>
				{version}
			</div>
		});
	}

	let mut component_items = Vec::new();
	if spell.components.verbal {
		component_items.push(html!("Verbal"));
	}
	if spell.components.somatic {
		component_items.push(html!("Somatic"));
	}
	for (material, consumed) in &spell.components.materials {
		component_items.push(html! {
			<span>
				{"Material: "}
				{material}
				{consumed.then(|| " (consumed)").unwrap_or_default()}
			</span>
		});
	}
	sections.push(html! {
		<div class="property">
			<strong>{"Components:"}</strong>
			<ul>
				{component_items.into_iter().map(|entry| html! {<li>{entry}</li>}).collect::<Vec<_>>()}
			</ul>
		</div>
	});

	let desc = {
		let (atk_bonus, save_dc) = entry
			.map(|entry| {
				let atk_bonus = match entry.attack_bonus {
					AbilityOrStat::Stat(stat) => stat,
					AbilityOrStat::Ability(ability) => state.ability_modifier(ability, Some(proficiency::Level::Full)),
				};
				let save_dc = match entry.save_dc {
					AbilityOrStat::Stat(stat) => stat as i32,
					AbilityOrStat::Ability(ability) => {
						8 + state.ability_modifier(ability, Some(proficiency::Level::Full))
					}
				};
				(atk_bonus, save_dc)
			})
			.unwrap_or((0, 0));
		let caster_args = std::collections::HashMap::from([
			("{CasterAtk}".into(), format!("{atk_bonus:+}")),
			("{CasterDC}".into(), format!("{save_dc}")),
		]);
		let desc = spell.description.clone().evaluate_with(state, Some(caster_args));
		desc.sections
			.into_iter()
			.map(|section| {
				html! {
					<DescriptionSection {section} show_selectors={false} />
				}
			})
			.collect::<Vec<_>>()
	};

	let notes = html!(<NotesField path={Arc::new(spell.id.path.clone())} />);

	// TODO: Show evaluated attack/save dc + damage + damage type + damage bonuses (and sources)

	html! {<>
		{sections}
		<div class="hr my-2" />
		{desc}
		{notes}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct AvailableSpellListProps {
	pub criteria: Option<crate::database::Criteria>,
	pub entry: Option<SpellEntry>,
	pub header_addon: HeaderAddon,
	#[prop_or(SpellSource::Database)]
	pub source: SpellSource,
}
#[derive(Clone)]
pub struct HeaderAddon(std::sync::Arc<dyn Fn(&Spell) -> Html>);
impl PartialEq for HeaderAddon {
	fn eq(&self, other: &Self) -> bool {
		std::sync::Arc::ptr_eq(&self.0, &other.0)
	}
}
impl<F> From<F> for HeaderAddon
where
	F: Fn(&Spell) -> Html + 'static,
{
	fn from(value: F) -> Self {
		Self(std::sync::Arc::new(value))
	}
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SpellSource {
	Database,
	Items,
}
#[function_component]
pub fn AvailableSpellList(props: &AvailableSpellListProps) -> Html {
	use yew_hooks::{use_async_with_options, UseAsyncOptions};
	let state = use_context::<CharacterHandle>().unwrap();
	let database = use_context::<Database>().unwrap();
	let system_depot = use_context::<system::Registry>().unwrap();
	let object_cache = use_context::<ObjectCacheArc>().unwrap();

	let criteria_handle = use_state({
		let criteria = props.criteria.clone();
		move || criteria
	});
	use_effect_with(props.criteria.clone(), {
		let criteria_handle = criteria_handle.clone();
		move |criteria: &Option<Criteria>| {
			criteria_handle.set(criteria.clone());
		}
	});

	let source_kind = props.source;
	let mut contained_spells = Vec::new();
	let mut indirect_ids_to_fetch = Vec::new();

	// if we are looking for spells contained within items, lets gather all of the possible ids/objects
	if source_kind == SpellSource::Items {
		for equipable in state.inventory().entries() {
			// gather all spells from containers which are equipped and can be prepared from
			if !equipable.status.is_equipped() {
				continue;
			}
			let Some(spell_container) = &equipable.item.spells else {
				continue;
			};
			if !spell_container.can_prepare_from {
				continue;
			}
			contained_spells.extend(spell_container.spells.clone());
			indirect_ids_to_fetch.reserve(spell_container.spells.len());
			for contained_spell in &spell_container.spells {
				if let Indirect::Id(id) = &contained_spell.spell {
					indirect_ids_to_fetch.push(id.clone());
				}
			}
		}
	}

	let load_data = use_async_with_options(
		{
			let criteria_handle = criteria_handle.clone();
			async move {
				let mut sorted_info = Vec::<(String, u8)>::new();
				let mut spells = Vec::new();

				let mut insert_spell = |spell: Spell| {
					// Insertion sort by rank & name
					let idx = sorted_info
						.binary_search_by(|(name, rank)| rank.cmp(&spell.rank).then(name.cmp(&spell.name)))
						.unwrap_or_else(|err_idx| err_idx);
					let info = (spell.name.clone(), spell.rank);
					sorted_info.insert(idx, info);
					spells.insert(idx, spell);
				};

				match source_kind {
					SpellSource::Database => {
						use crate::system::System;
						let criteria = (*criteria_handle).clone();
						let index = EntryInSystemWithType::new::<Spell>(DnD5e::id());
						let query = Query::<Entry>::subset(&database, Some(index)).await;
						let Ok(query) = query else { return Ok(spells) };
						let query = query.apply_opt(criteria, Query::filter_by);
						let mut query = query.parse_as_cached::<Spell>(&system_depot, &object_cache);
						while let Some((_entry, spell)) = query.next().await {
							insert_spell(spell);
						}
					}
					SpellSource::Items => {
						for contained_spell in &contained_spells {
							let Indirect::Custom(spell) = &contained_spell.spell else {
								continue;
							};
							insert_spell(spell.clone());
						}

						for spell_id in &indirect_ids_to_fetch {
							let query = database.get_typed_entry::<Spell>(
								spell_id.clone(),
								system_depot.clone(),
								(*criteria_handle).clone(),
							);
							match query.await {
								Ok(Some(spell)) => insert_spell(spell),
								Ok(None) => {}
								Err(_err) => {}
							}
						}
					}
				}

				Ok(spells) as Result<Vec<Spell>, ()>
			}
		},
		UseAsyncOptions::default(),
	);

	// PERF: The relevant spell list could be searched when the character is opened, instead of doing it every time the modal is opened
	if yew_hooks::use_is_first_mount() {
		load_data.run();
	}
	use_effect_with((criteria_handle.clone(), props.source), {
		let load_data = load_data.clone();
		move |_: &(UseStateHandle<Option<Criteria>>, SpellSource)| {
			load_data.run();
		}
	});

	// TODO: Search bar for available spells section
	html! {<>
		{match (load_data.loading, &load_data.data) {
			(false, None) => html! {"Spells not loaded"},
			(true, _) => html!(<Spinner />),
			(false, Some(data)) => {
				let htmls = data.iter().map(|spell| {
					let addon = (props.header_addon.0)(spell);
					spell_list_item("relevant", &state, spell, props.entry.as_ref(), addon)
				}).collect::<Vec<_>>();
				html!(<>{htmls}</>)
			}
		}}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct UseSpellButtonProps {
	kind: UseSpell,
}
#[derive(Clone, PartialEq)]
enum UseSpell {
	AtWill,
	RitualOnly,
	Slot { spell_rank: u8, slot_rank: u8, slots: Option<(usize, usize)>, id: SourceId },
	LimitedUse { data_path: Option<PathBuf>, max_uses: u32, uses_consumed: u32 },
	ConsumeItemSpell { item_path: ItemPath, spell_id: SourceId, consume_item: bool },
}
#[function_component]
fn UseSpellButton(UseSpellButtonProps { kind }: &UseSpellButtonProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let disabled = !state.spellcasting().can_cast_any;
	match kind {
		UseSpell::AtWill if !disabled => html! {
			<div class="text-center" style="font-size: 9px; font-weight: 700; color: var(--bs-gray-600);">
				{"AT"}<br />{"WILL"}
			</div>
		},
		UseSpell::RitualOnly if !disabled => html! {
			<div class="text-center" style="font-size: 9px; font-weight: 700; color: var(--bs-gray-600);">
				{"RITUAL"}<br />{"ONLY"}
			</div>
		},
		UseSpell::AtWill | UseSpell::RitualOnly => html! {
			<div class="text-center" style="font-size: 9px; font-weight: 700; color: var(--bs-gray-600);">
				{"CASTING"}<br />{"DISABLED"}
			</div>
		},
		UseSpell::Slot { spell_rank, slot_rank, slots, id } => {
			let can_cast = slots.as_ref().map(|(consumed, max)| consumed < max).unwrap_or(false);
			let onclick = match slots.as_ref() {
				Some((consumed, max)) if consumed < max => Some(state.dispatch_change({
					let rank = *slot_rank;
					let available = *max - *consumed;
					let spell_id = id.clone();
					move |evt: MouseEvent| {
						use system::dnd5e::change::ConsumeSpellSlot;
						evt.stop_propagation();
						let slots_remaining = available.saturating_add_signed(-1);
						Some(ConsumeSpellSlot::cast_spell(rank, slots_remaining, spell_id.clone()))
					}
				})),
				_ => None,
			};
			// TODO: Revisit when spell panel rows are more fleshed out. This upcast rank span thing is not
			// very well formated/displayed, esp with such a small text size.
			let upcast_span = (slot_rank > spell_rank).then(|| html! {
				<span class="d-flex position-absolute" style="left: 1px; right: 0; top: -8px;">
					<span style="align-items: flex-start; background-color: #1c9aef; border: 1px solid hsla(0,0%,100%,.5); border-radius: 2px; color: #fff; display: flex; font-size: 8px; line-height: 1; padding: 1px 3px;">
						{*spell_rank}
						<span style="font-size: 6px;">{rank_suffix(*spell_rank)}</span>
					</span>
				</span>
			});
			let mut btn_classes = classes!("btn", "btn-xs", "px-1");
			btn_classes.push(match can_cast {
				true => classes!("btn-theme"),
				false => classes!("btn-outline-theme", "disabled"),
			});
			html! {
				<button class={btn_classes} {onclick} {disabled}>
					<div class="position-relative">
						{upcast_span.unwrap_or_default()}
						{"Cast"}
					</div>
				</button>
			}
		}
		UseSpell::LimitedUse { data_path, max_uses, uses_consumed } => {
			let onclick = match &data_path {
				None => None,
				Some(path) => Some(state.dispatch_change({
					let data_path = path.clone();
					let uses_consumed = uses_consumed + 1;
					move |evt: MouseEvent| {
						evt.stop_propagation();
						Some(system::dnd5e::change::ApplyLimitedUses(data_path.clone(), uses_consumed))
					}
				})),
			};
			let uses_remaining = max_uses.saturating_sub(*uses_consumed);
			let disabled = uses_consumed >= max_uses || !state.spellcasting().can_cast_any;
			html! {
				<button class="btn btn-theme btn-xs px-1" {onclick} {disabled}>
					{"Use"}
					<span class="ms-1 d-none" style="font-size: 9px; color: var(--bs-gray-600);">{format!("({uses_remaining}/{max_uses})")}</span>
				</button>
			}
		}
		UseSpell::ConsumeItemSpell { item_path, spell_id, consume_item } => {
			let onclick = state.dispatch_change({
				let item_path = item_path.clone();
				let spell_id = spell_id.clone();
				let consume_item = *consume_item;
				move |evt: MouseEvent| {
					evt.stop_propagation();
					Some(system::dnd5e::change::ConsumeItemSpell {
						item_path: item_path.clone(),
						spell_id: spell_id.clone(),
						consume_item,
					})
				}
			});
			let disabled = !state.spellcasting().can_cast_any;
			html! {
				<button class="btn btn-theme btn-xs px-1" {onclick} {disabled}>
					{"Use"}
				</button>
			}
		}
	}
}
