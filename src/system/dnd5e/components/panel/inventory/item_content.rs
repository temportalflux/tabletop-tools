use crate::{
	components::{
		context_menu,
		database::{use_query_typed, QueryStatus, UseQueryAllHandle, UseQueryDiscreteTypedHandle},
		progress_bar, Tag,
	},
	database::Criteria,
	page::characters::sheet::{
		joined::editor::{description, mutator_list},
		CharacterHandle, MutatorImpact,
	},
	system::{
		dnd5e::{
			change::{self, inventory::ItemRef},
			components::{
				panel::{spell_name_and_icons, spell_overview_info, AvailableSpellList, HeaderAddon, NotesField},
				validate_uint_only, FormulaInline, GeneralProp, UseCounterDelta, WalletInline,
			},
			data::{
				character::{Persistent, MAX_SPELL_RANK},
				item::{
					self,
					container::{
						item::{EquipStatus, ItemPath},
						spell::ContainerSpell,
					},
					Item,
				},
				spell::CastingDuration,
				ArmorExtended, Indirect, Spell, WeaponProficiency,
			},
			evaluator::HasProficiency,
		},
		Evaluator, SourceId,
	},
	utility::InputExt,
};
use any_range::AnyRange;
use enumset::EnumSet;
use itertools::Itertools;
use std::{collections::HashSet, path::Path, str::FromStr, sync::Arc};
use yew::prelude::*;

pub fn get_inventory_item_hierarchy<'c>(state: &'c CharacterHandle, id_path: &ItemPath) -> Vec<&'c Item> {
	let mut iter = id_path.iter();
	let mut item = None;
	let mut items = Vec::new();
	while let Some(id) = iter.next() {
		item = match item {
			None => state.inventory().get_item(id),
			Some(prev_item) => match &prev_item.items {
				None => return Vec::new(),
				Some(container) => container.get_item(id),
			},
		};
		if let Some(item) = item {
			items.push(item);
		}
	}
	items
}
pub fn get_item_path_names<'c>(state: &'c CharacterHandle, id_path: &ItemPath) -> Vec<String> {
	let iter = get_inventory_item_hierarchy(&state, &id_path).into_iter();
	let iter = iter.map(|item| item.name.clone());
	iter.collect()
}
pub fn get_inventory_item<'c>(state: &'c CharacterHandle, id_path: &ItemPath) -> Option<&'c Item> {
	let mut iter = id_path.iter();
	let mut item = None;
	while let Some(id) = iter.next() {
		item = match item {
			None => state.inventory().get_item(id),
			Some(prev_item) => match &prev_item.items {
				None => {
					return None;
				}
				Some(container) => container.get_item(id),
			},
		};
	}
	item
}
pub fn get_inventory_item_mut<'c>(persistent: &'c mut Persistent, id_path: &ItemPath) -> Option<&'c mut Item> {
	let mut iter = id_path.iter();
	let Some(id) = iter.next() else {
		return None;
	};
	let mut item = persistent.inventory.get_mut(id);
	while let Some(id) = iter.next() {
		let Some(prev_item) = item.take() else {
			return None;
		};
		let Some(container) = &mut prev_item.items else {
			return None;
		};
		item = container.get_mut(id);
	}
	item
}

#[derive(Clone, PartialEq)]
pub enum ItemLocation {
	Explicit { item: std::rc::Rc<Item> },
	Database { query: UseQueryAllHandle<Item>, index: usize },
	Inventory { id_path: ItemPath },
}
impl ItemLocation {
	pub fn resolve<'c>(&'c self, state: &'c CharacterHandle) -> Option<&'c Item> {
		match self {
			Self::Explicit { item } => Some(&*item),
			Self::Database { query, index } => match query.status() {
				QueryStatus::Success(data) => data.get(*index),
				_ => None,
			},
			Self::Inventory { id_path } => get_inventory_item(state, id_path),
		}
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct ItemBodyProps {
	pub location: ItemLocation,
	#[prop_or_default]
	pub on_quantity_changed: Option<Callback<u32>>,
	#[prop_or_default]
	pub equip_status: EquipStatus,
	#[prop_or_default]
	pub set_equipped: Option<Callback<EquipStatus>>,
}
#[function_component]
pub fn ItemInfo(props: &ItemBodyProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();

	let Some(item) = props.location.resolve(&state) else { return Html::default() };
	let state_with_inventory = match &props.location {
		ItemLocation::Inventory { .. } => Some(state.clone()),
		_ => None,
	};

	let mut sections = Vec::new();
	if HasProficiency::Tool(item.name.clone()).evaluate(&state) {
		sections.push(html! {
			<div class="property">
				<strong>{"Proficient (with tool):"}</strong>
				<span><i class="bi bi-check-square" style="color: green;" />{"Yes"}</span>
			</div>
		});
	}
	if let Some(rarity) = item.rarity {
		sections.push(html! {
			<div class="property">
				<strong>{"Rarity:"}</strong>
				<span>{rarity.to_string()}</span>
			</div>
		});
	}
	if !item.worth.is_empty() {
		sections.push(html! {
			<div class="property">
				<strong>{"Worth:"}</strong>
				<span><WalletInline wallet={item.worth} /></span>
			</div>
		});
	}
	if item.weight > 0.0 {
		sections.push(html! {
			<div class="property">
				<strong>{"Weight:"}</strong>
				<span>{item.weight * item.quantity() as f32}{" lb."}</span>
			</div>
		});
	}
	match &item.kind {
		item::Kind::Simple { count } => {
			let inner = match (&props.on_quantity_changed, item.can_stack()) {
				(None, _) | (Some(_), false) => html! { <span>{count}</span> },
				(Some(on_changed), true) => {
					html!(<UIntField class={"num-field-inline"} value={*count} {on_changed} />)
				}
			};
			sections.push(html! {
				<div class="property">
					<strong>{"Quantity:"}</strong>
					{inner}
				</div>
			});
		}
		item::Kind::Equipment(equipment) => {
			let mut equip_sections = Vec::new();
			if let Some(on_equipped) = props.set_equipped.clone() {
				let onchange = Callback::from(move |evt: web_sys::Event| {
					let Some(value_str) = evt.select_value() else { return };
					let Ok(status) = EquipStatus::from_str(&value_str) else { return };
					on_equipped.emit(status);
				});
				let mut statuses = EnumSet::<EquipStatus>::all();
				let has_attunement_slots = state.attunement() < state.persistent().attunement_slots;
				if equipment.attunement.is_none() {
					statuses.remove(EquipStatus::Attuned);
				}
				equip_sections.push(html! {
					<select class="form-select form-select-sm w-auto" {onchange}>
						{statuses.into_iter().map(|status| {
							let selected = status == props.equip_status;
							// cannot select attuned if its not selected and we dont have additional slots
							let disabled = status == EquipStatus::Attuned && !selected && !has_attunement_slots;
							html!(<option {selected} {disabled}>{status.to_string()}</option>)
						}).collect::<Vec<_>>()}
					</select>
				});
			}
			if !equipment.mutators.is_empty() {
				let mut criteria_html = None;
				if let Some(criteria) = &equipment.criteria {
					criteria_html = Some(html! {
						<div>
							<div>{"Only if:"}</div>
							<span>{criteria.description().unwrap_or_else(|| format!("criteria missing description"))}</span>
						</div>
					});
				}
				equip_sections.push(html! {
					<div class="border-bottom-theme-muted">
						<div>{"You gain the following benefits while this item is equipped:"}</div>
						{mutator_list(&equipment.mutators, state_with_inventory.as_ref())}
						{criteria_html.unwrap_or_default()}
					</div>
				});
			}
			if let Some(shield_bonus) = &equipment.shield {
				equip_sections.push(html! {
					<div class="border-bottom-theme-muted">
						<strong>{"Shield"}</strong>
						<div class="ms-3">
							<div class="property">
								<strong>{"Proficient:"}</strong>
								{match equipment.always_proficient || HasProficiency::Armor(ArmorExtended::Shield).evaluate(&state) {
									true => html! { <span><i class="bi bi-check-square" style="color: green;" />{"Yes"}</span> },
									false => html! { <span><i class="bi bi-x-square" style="color: red;" />{"No"}</span> },
								}}
							</div>
							<div class="property">
								<strong>{"Armor Class Bonus:"}</strong>
								<span>{format!("{shield_bonus:+}")}</span>
							</div>
						</div>
					</div>
				});
			}
			if let Some(armor) = &equipment.armor {
				let mut armor_sections = Vec::new();
				armor_sections.push(html! {
					<div class="property">
						<strong>{"Type:"}</strong>
						<span>{armor.kind.to_string()}</span>
					</div>
				});
				armor_sections.push(html! {
					<div class="property">
						<strong>{"Proficient:"}</strong>
						{match equipment.always_proficient || HasProficiency::Armor(ArmorExtended::Kind(armor.kind)).evaluate(&state) {
							true => html! { <span><i class="bi bi-check-square" style="color: green;" />{"Yes"}</span> },
							false => html! { <span><i class="bi bi-x-square" style="color: red;" />{"No"}</span> },
						}}
					</div>
				});
				armor_sections.push(html! {
					<div class="property">
						<strong>{"Armor Class Formula:"}</strong>
						<span><FormulaInline formula={armor.formula.clone()} /></span>
					</div>
				});
				if let Some(min_score) = &armor.min_strength_score {
					armor_sections.push(html! {
						<div class="property">
							<strong>{"Minimum Strength Score:"}</strong>
							<span>{min_score}</span>
						</div>
					});
				}
				equip_sections.push(html! {
					<div class="border-bottom-theme-muted">
						<strong>{"Armor"}</strong>
						<div class="ms-3">
							{armor_sections}
						</div>
					</div>
				});
			}
			if let Some(weapon) = &equipment.weapon {
				let mut weapon_sections = Vec::new();
				weapon_sections.push(html! {
					<div class="property">
						<strong>{"Type:"}</strong>
						<span>{weapon.kind.to_string()}</span>
					</div>
				});
				weapon_sections.push(html! {
					<div class="property">
						<strong>{"Classification:"}</strong>
						<span>{weapon.classification.clone()}</span>
					</div>
				});
				let is_proficient = vec![
					HasProficiency::Weapon(WeaponProficiency::Kind(weapon.kind)),
					HasProficiency::Weapon(WeaponProficiency::Classification(weapon.classification.clone())),
				];
				let is_proficient = is_proficient.into_iter().any(|eval| eval.evaluate(&state));
				weapon_sections.push(html! {
					<div class="property">
						<strong>{"Proficient:"}</strong>
						{match equipment.always_proficient || is_proficient {
							true => html! { <span><i class="bi bi-check-square" style="color: green;" />{"Yes"}</span> },
							false => html! { <span><i class="bi bi-x-square" style="color: red;" />{"No"}</span> },
						}}
					</div>
				});
				if let Some(reach) = weapon.melee_reach() {
					weapon_sections.push(html! {
						<div class="property">
							<strong>{"Melee Attack Reach:"}</strong>
							<span>{reach}{" ft."}</span>
						</div>
					});
				}
				if let Some((short, long)) = weapon.range() {
					// TODO: find a way to communicate attack range better:
					// - normal if the target is at or closer than `short`
					// - made a disadvantage when the target is father than `short`, but closer than `long`
					// - impossible beyond the `long` range
					weapon_sections.push(html! {
						<div class="property">
							<strong>{"Range:"}</strong>
							<span>{format!("{short} ft. / {long} ft.")}</span>
						</div>
					});
				}
				if let Some(damage) = &weapon.damage {
					weapon_sections.push(html! {
						<div class="property">
							<strong>{"Damage:"}</strong>
							<span>
								{match (&damage.roll, damage.bonus) {
									(None, bonus) => bonus.to_string(),
									(Some(roll), 0) => roll.to_string(),
									(Some(roll), bonus) => format!("{}{bonus:+}", roll.to_string()),
								}}
								<span style="margin-left: 0.5rem;">{damage.damage_type.display_name()}</span>
							</span>
						</div>
					});
				}
				if !weapon.properties.is_empty() {
					weapon_sections.push(html! {
						<div class="property">
							<strong>{"Properties:"}</strong>
							<ul>
								{weapon.properties.iter().map(|property| html! {
									<li>
										<div class="property">
											<strong>{property.display_name()}{":"}</strong>
											<span>{property.description()}</span>
										</div>
									</li>
								}).collect::<Vec<_>>()}
							</ul>
						</div>
					});
				}
				equip_sections.push(html! {
					<div class="border-bottom-theme-muted">
						<strong>{"Weapon"}</strong>
						<div class="ms-3">
							{weapon_sections}
						</div>
					</div>
				});
			}
			if let Some(attunement) = &equipment.attunement {
				if !attunement.mutators.is_empty() {
					equip_sections.push(html! {
						<div class="border-bottom-theme-muted">
							<div>{"You gain the following benefits while this item is attuned:"}</div>
							{mutator_list(&attunement.mutators, state_with_inventory.as_ref())}
						</div>
					});
				}
			}
			if let Some(resource) = &equipment.charges {
				if matches!(&props.location, ItemLocation::Inventory { .. }) {
					let max_uses = resource.get_capacity(&*state) as u32;
					let consumed_uses = resource.get_uses_consumed(&*state);
					let on_apply = state.new_dispatch({
						let data_path = resource.get_uses_path();
						move |delta: i32, persistent| {
							let Some(data_path) = &data_path else {
								return MutatorImpact::None;
							};
							let prev_value = persistent.get_first_selection_at::<u32>(data_path);
							let consumed_uses = prev_value.map(Result::ok).flatten().unwrap_or_default();
							let new_value = consumed_uses.saturating_add_signed(-delta);
							let new_value = (new_value > 0).then(|| new_value.to_string());
							persistent.set_selected(data_path, new_value);
							MutatorImpact::None
						}
					});
					// <UIntField class={"num-field-inline"} value={uses_remaining} {on_changed} />
					equip_sections.push(html! {
						<div class="property">
							<strong>{"Charges:"}</strong>
							<UseCounterDelta {max_uses} {consumed_uses} {on_apply} />
						</div>
					});
				}
			}
			sections.push(html! {
				<div>
					<strong>{"Equipment"}</strong>
					<div class="ms-3">
						{equip_sections}
					</div>
				</div>
			});
		}
	}

	if let Some(spell_container) = &item.spells {
		let mut container_sections = Vec::new();

		container_sections.push(html! {
			<div class="property">
				<strong>{"Transcribe Spells From:"}</strong>
				<span>
					{match spell_container.can_transcribe_from {
						true => html!(<i class="bi bi-check-square" style="color: green;" />),
						false => html!(<i class="bi bi-x-square" style="color: red;" />),
					}}
				</span>
			</div>
		});
		container_sections.push(html! {
			<div class="property">
				<strong>{"Prepare Contained Spells:"}</strong>
				<span>
					{match spell_container.can_prepare_from {
						true => html!(<i class="bi bi-check-square" style="color: green;" />),
						false => html!(<i class="bi bi-x-square" style="color: red;" />),
					}}
				</span>
			</div>
		});
		container_sections.push(html! {
			<div class="property">
				<strong>{"Casting: "}</strong>
				{match &spell_container.casting {
					None => html!(<i class="bi bi-x-square" style="color: red;" />),
					Some(casting) => html! {
						<div class="ms-3">
							{match &casting.duration {
								None => html!(),
								Some(duration) => html! {
									<div class="property">
										<strong>{"Casting Time:"}</strong>
										<span>
											{match duration {
												CastingDuration::Action => html!("Action"),
												CastingDuration::Bonus => html!("Bonus Action"),
												CastingDuration::Reaction(_trigger) => html!("Reaction"),
												CastingDuration::Unit(amt, unit) => html!(format!("{amt} {unit}")),
											}}
										</span>
									</div>
								},
							}}
							<div class="property">
								<strong>{"Destroy Item on All Consumed:"}</strong>
								<span>
									{match casting.consume_item {
										true => html!("Destroyed"),
										false => html!("Not Destroyed"),
									}}
								</span>
							</div>
							<div class="property">
								<strong>{"Consume Spell on Use:"}</strong>
								<span>
									{match casting.consume_spell {
										true => html!("Consumed"),
										false => html!("Not Consumed"),
									}}
								</span>
							</div>
							{match casting.save_dc {
								None => html!(),
								Some(dc) => html! {
									<div class="property">
										<strong>{"Spell Save DC:"}</strong>
										<span>{dc}</span>
									</div>
								},
							}}
							{match casting.attack_bonus {
								None => html!(),
								Some(atk_bonus) => html! {
									<div class="property">
										<strong>{"Spell Attack:"}</strong>
										<span>{format!("{atk_bonus:+}")}</span>
									</div>
								},
							}}
						</div>
					},
				}}
			</div>
		});

		let mut capacity_sections = Vec::new();
		if let Some(max_count) = &spell_container.capacity.max_count {
			capacity_sections.push(html! {
				<div class="property">
					<strong>{"Total Stored Spells"}</strong>
					<progress_bar::Ticked
						classes={"mt-4"}
						ticked_bar_range={progress_bar::TickDisplay::BoundsOnly}
						bar_range={0..=(*max_count as i64)}
						value_range={AnyRange::from(..=spell_container.spells.len() as i64)}
						show_labels={true}
					/>
				</div>
			});
		}
		if let Some(rank_total) = &spell_container.capacity.rank_total {
			// TODO: Query the contained spell ids so we have them on hand
			let current_value: i64 = spell_container
				.spells
				.iter()
				.map(|contained| {
					match &contained.rank {
						Some(rank) => *rank as i64,
						None => 0 as i64, // TODO: get rank for that spell id
					}
				})
				.sum();
			capacity_sections.push(html! {
				<div class="property">
					<strong>{"Total Stored Ranks"}</strong>
					<progress_bar::Ticked
						classes={"mt-4"}
						ticked_bar_range={progress_bar::TickDisplay::BoundsOnly}
						bar_range={0..=(*rank_total as i64)}
						value_range={AnyRange::from(..=current_value)}
						show_labels={true}
					/>
				</div>
			});
		}
		let rank_min = spell_container.capacity.rank_min.unwrap_or(0);
		let rank_max = spell_container.capacity.rank_max.unwrap_or(MAX_SPELL_RANK);
		if rank_min != 0 || rank_max != MAX_SPELL_RANK {
			capacity_sections.push({
				html! {
					<div class="property">
						<strong>{"Allowed Spell Ranks"}</strong>
						<progress_bar::Ticked
							classes={"mt-4"}
							ticked_bar_range={progress_bar::TickDisplay::AllTicks}
							bar_range={0..=(MAX_SPELL_RANK as i64)}
							value_range={AnyRange::from((rank_min as i64)..=(rank_max as i64))}
							show_labels={true}
						/>
					</div>
				}
			});
		}

		if !capacity_sections.is_empty() {
			container_sections.push(html! {
				<div>
					<strong>{"Storage Capacity"}</strong>
					<div class="ms-3 capacity">
						{capacity_sections}
					</div>
				</div>
			});
		}

		let browse = match &props.location {
			ItemLocation::Inventory { id_path } => {
				let onclick = Callback::from({
					let context_menu = context_menu.clone();
					let id_path = id_path.clone();
					move |_| {
						context_menu.dispatch(context_menu::Action::open_child(
							"Spell Container",
							html!(<ModalSpellContainerBrowser value={id_path.clone()} />),
						));
					}
				});
				html!(
					<button class="btn btn-outline-theme btn-xs" type="button" {onclick}>
						{"Browse Spells"}
					</button>
				)
			}
			_ => html!(),
		};
		let contents = html! {
			<div>
				{"TODO Show spell contents"}
			</div>
		};

		sections.push(html! {
			<div>
				<strong>{"Spell Container"}</strong>
				<div class="ms-3 spell-container">
					{container_sections}
					{browse}
					{contents}
				</div>
			</div>
		});
	}

	if !item.tags.is_empty() {
		sections.push(html! {
			<div class="property">
				<strong>{"Tags:"}</strong>
				<span>{item.tags.join(", ")}</span>
			</div>
		});
	}

	let available_tags = {
		use crate::system::Block;
		// NOTE: Could optimize by having the metadata for the item accessible to the function
		let item_metadata = item.clone().to_metadata();
		let iter = state.user_tags().tags().iter();
		// filter to only include tags relevant to this specific item
		let iter = iter.filter(|user_tag| match &user_tag.filter {
			None => true,
			Some(filter) => filter.as_criteria().is_relevant(&item_metadata),
		});
		// Map to collect metadata about each tag; if it can be applied to more items and if its applied to the item
		let iter = iter.map(|user_tag| {
			let is_applied = item.user_tags.contains(&user_tag.tag);

			let has_available_usages = match &user_tag.max_count {
				None => true,
				Some(max) => {
					let usages = state.user_tags().usages_of(&user_tag.tag);
					let usages = usages.map(Vec::len).unwrap_or(0);
					usages < *max
				}
			};

			(&user_tag.tag, is_applied, has_available_usages)
		});
		iter.collect::<Vec<_>>()
	};
	let add_tag_button = match (available_tags.is_empty(), &props.location) {
		// Cannot toggle the tag if there are no tags, or the item is in the database
		(true, _) | (false, ItemLocation::Database { .. } | ItemLocation::Explicit { .. }) => None,
		// Can toggle the tag if the item is in the character's inventory
		(_, ItemLocation::Inventory { id_path }) => Some({
			let onchange = state.dispatch_change({
				let state = state.clone();
				let item_ref = ItemRef { path: id_path.clone(), name: get_item_path_names(&state, &id_path) };
				move |evt: web_sys::Event| {
					let tag = evt.select_value()?;
					let item = get_inventory_item(&state, &item_ref.path)?;
					let is_applied = item.user_tags.contains(&tag);
					Some(change::inventory::ApplyItemUserTag {
						item: item_ref.clone(),
						tag,
						should_be_applied: !is_applied,
					})
				}
			});
			html!(<select class="form-select form-select-sm w-auto" {onchange}>
				<option selected=true>{"Select Tag(s)"}</option>
				{available_tags.into_iter().map(|(tag, is_applied, has_available_usages)| {
					let can_select = is_applied || has_available_usages;
					html!(<option value={tag.clone()} disabled={!can_select}>
						{is_applied.then_some(html!("✅ "))}
						{tag}
					</option>)
				}).collect::<Vec<_>>()}
			</select>)
		}),
	};
	if !item.user_tags.is_empty() || add_tag_button.is_some() {
		sections.push(html! {
			<div class="property user-tags">
				<strong>{"User Tags:"}</strong>
				<span class="d-flex justify-content-start align-items-center">
					{add_tag_button}
					{item.user_tags.iter().map(|tag_id| html!(<Tag>{tag_id.as_str()}</Tag>)).collect::<Vec<_>>()}
				</span>
			</div>
		});
	}

	if !item.description.is_empty() {
		let desc = item.description.clone().evaluate(&state);
		sections.push(description(&desc, false, false));
	}
	if let Some(notes) = &item.notes {
		sections.push(html! {
			<div class="property">
				<strong>{"Notes."}</strong>
				<span class="text-block">{notes.clone()}</span>
			</div>
		});
	}

	if let ItemLocation::Inventory { id_path } = &props.location {
		// TODO: update the notes for an item when it is moved between inventory containers
		//       (wont be done at this location, this is just where the thought came to mind)
		// TODO: update inventory panel to show notes in each row
		let path = Arc::new(Path::new(&id_path.iter().join("/")).to_owned());
		sections.push(html!(<NotesField {path} />));
	}

	html! {<>
		{sections}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct UIntFieldProps {
	#[prop_or_default]
	class: Classes,
	value: u32,
	on_changed: Callback<u32>,
}
#[function_component]
fn UIntField(UIntFieldProps { class, value, on_changed }: &UIntFieldProps) -> Html {
	let count = *value;
	let increment = Callback::from({
		let on_changed = on_changed.clone();
		move |_| {
			on_changed.emit(count.saturating_add(1));
		}
	});
	let decrement = Callback::from({
		let on_changed = on_changed.clone();
		move |_| {
			on_changed.emit(count.saturating_sub(1));
		}
	});
	let onchange = Callback::from({
		let on_changed = on_changed.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.input_value_t::<u32>() else {
				return;
			};
			on_changed.emit(value);
		}
	});
	html! {
		<div class={classes!(class.clone(), "input-group")}>
			<button type="button" class="btn btn-theme" onclick={decrement}><i class="bi bi-dash" /></button>
			<input
				class="form-control text-center"
				type="number"
				min="0" value={count.to_string()}
				onkeydown={validate_uint_only()}
				onchange={onchange}
			/>
			<button type="button" class="btn btn-theme" onclick={increment}><i class="bi bi-plus" /></button>
		</div>
	}
}

#[function_component]
fn ModalSpellContainerBrowser(GeneralProp { value }: &GeneralProp<ItemPath>) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();

	let fetch_indirect_spells = use_query_typed::<Spell>();
	let indirect_spell_ids = use_state_eq(|| Vec::new());
	use_effect_with(indirect_spell_ids.clone(), {
		let fetch_indirect_spells = fetch_indirect_spells.clone();
		move |ids: &UseStateHandle<Vec<SourceId>>| {
			fetch_indirect_spells.run((**ids).clone());
		}
	});

	let Some(item) = get_inventory_item(&state, value) else {
		return Html::default();
	};
	let Some(spell_container) = &item.spells else {
		return Html::default();
	};

	indirect_spell_ids.set(
		spell_container
			.spells
			.iter()
			.filter_map(|contained| match &contained.spell {
				Indirect::Id(id) => Some(id.unversioned()),
				Indirect::Custom(_spell) => None,
			})
			.collect::<Vec<_>>(),
	);

	let consumed_rank_sum: usize = spell_container
		.spells
		.iter()
		.map(|contained| match contained.rank {
			Some(fixed_rank) => fixed_rank as usize,
			None => {
				let spell = match &contained.spell {
					Indirect::Id(id) => match fetch_indirect_spells.status() {
						QueryStatus::Success((_ids, spells_by_id)) => spells_by_id.get(id),
						_ => None,
					},
					Indirect::Custom(spell) => Some(spell),
				};
				spell.map(|spell| spell.rank as usize).unwrap_or(0)
			}
		})
		.sum();
	let remaining_total_rank = spell_container.capacity.rank_total.map(|total| total.saturating_sub(consumed_rank_sum));

	let open_browser = {
		let onclick = Callback::from({
			let context_menu = context_menu.clone();
			let id_path = value.clone();
			let fetch_indirect_spells = fetch_indirect_spells.clone();
			move |_| {
				context_menu.dispatch(context_menu::Action::open_child(
					"Add Spells",
					html!(<ModalSpellContainerAvailableList
						id_path={id_path.clone()}
						fetch_indirect_spells={fetch_indirect_spells.clone()}
					/>),
				));
			}
		});
		html!(
			<button class="btn btn-outline-theme btn-xs" type="button" {onclick}>
				{"Browse Spells"}
			</button>
		)
	};

	html! {<>
		<div class="details browse item-spell-container">
			{open_browser}
			<ContainedSpellsSection
				id_path={value.clone()}
				fetch_indirect_spells={fetch_indirect_spells.clone()}
				{remaining_total_rank}
			/>
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ContainedSpellsSectionProps {
	id_path: ItemPath,
	fetch_indirect_spells: UseQueryDiscreteTypedHandle<Spell>,
	remaining_total_rank: Option<usize>,
}
#[function_component]
fn ContainedSpellsSection(props: &ContainedSpellsSectionProps) -> Html {
	let ContainedSpellsSectionProps { id_path, fetch_indirect_spells, remaining_total_rank } = props;
	let state = use_context::<CharacterHandle>().unwrap();

	let Some(item) = get_inventory_item(&state, id_path) else {
		return Html::default();
	};
	let Some(spell_container) = &item.spells else {
		return Html::default();
	};

	let has_casting = spell_container.casting.is_some();
	let (casting_atk_bonus, casting_dc) = match &spell_container.casting {
		None => (None, None),
		Some(casting) => (casting.attack_bonus, casting.save_dc),
	};
	let rank_min = spell_container.capacity.rank_min.unwrap_or(0);
	let rank_max = spell_container.capacity.rank_max.unwrap_or(MAX_SPELL_RANK);

	let remove_from_container = state.new_dispatch({
		let id_path = id_path.clone();
		move |spell_idx: usize, persistent: &mut Persistent| {
			let Some(item) = get_inventory_item_mut(persistent, &id_path) else {
				return MutatorImpact::None;
			};
			let Some(spell_container) = &mut item.spells else {
				return MutatorImpact::None;
			};
			spell_container.spells.remove(spell_idx);
			// TODO: only recompile character when the modal is dismissed
			return MutatorImpact::Recompile;
		}
	});

	fn get_container_spell<'c>(
		persistent: &'c mut Persistent, id_path: &ItemPath, spell_idx: usize,
	) -> Option<&'c mut ContainerSpell> {
		let Some(item) = get_inventory_item_mut(persistent, &id_path) else {
			return None;
		};
		let Some(spell_container) = &mut item.spells else {
			return None;
		};
		spell_container.spells.get_mut(spell_idx)
	}

	let select_rank = state.new_dispatch({
		let id_path = id_path.clone();
		move |(spell_idx, desired_rank): (usize, Option<u8>), persistent: &mut Persistent| {
			let Some(contained) = get_container_spell(persistent, &id_path, spell_idx) else {
				return MutatorImpact::None;
			};
			contained.rank = desired_rank;
			// TODO: only recompile character when the modal is dismissed
			return MutatorImpact::Recompile;
		}
	});
	let select_atk_bonus = state.new_dispatch({
		let id_path = id_path.clone();
		move |(spell_idx, desired_bonus): (usize, Option<i32>), persistent: &mut Persistent| {
			let Some(contained) = get_container_spell(persistent, &id_path, spell_idx) else {
				return MutatorImpact::None;
			};
			contained.attack_bonus = desired_bonus;
			// TODO: only recompile character when the modal is dismissed
			return MutatorImpact::Recompile;
		}
	});
	let select_save_dc = state.new_dispatch({
		let id_path = id_path.clone();
		move |(spell_idx, desired_dc): (usize, Option<u8>), persistent: &mut Persistent| {
			let Some(contained) = get_container_spell(persistent, &id_path, spell_idx) else {
				return MutatorImpact::None;
			};
			contained.save_dc = desired_dc;
			// TODO: only recompile character when the modal is dismissed
			return MutatorImpact::Recompile;
		}
	});

	match fetch_indirect_spells.status() {
		QueryStatus::Pending => html!(<crate::components::Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No contained spells"),
		QueryStatus::Success((_ids, spells_by_id)) => {
			let mut ordered_indices = Vec::with_capacity(spell_container.spells.len());
			for (container_idx, contained) in spell_container.spells.iter().enumerate() {
				let spell = match &contained.spell {
					Indirect::Id(id) => match spells_by_id.get(&*id.minimal()) {
						Some(spell) => spell,
						None => continue,
					},
					Indirect::Custom(spell) => spell,
				};
				// Insertion sort by rank & name
				let order_idx = ordered_indices
					.binary_search_by(|(_, name, rank): &(usize, String, u8)| {
						rank.cmp(&spell.rank).then(name.cmp(&spell.name))
					})
					.unwrap_or_else(|err_idx| err_idx);
				ordered_indices.insert(order_idx, (container_idx, spell.name.clone(), spell.rank));
			}
			let mut contained_spells = Vec::with_capacity(ordered_indices.len());
			for (contained_idx, _, _) in ordered_indices {
				let Some(contained) = spell_container.spells.get(contained_idx) else {
					continue;
				};
				let ContainerSpell { spell, rank, save_dc, attack_bonus } = contained;
				let spell = match spell {
					Indirect::Id(id) => match spells_by_id.get(&*id.minimal()) {
						Some(spell) => spell,
						None => return Html::default(),
					},
					Indirect::Custom(spell) => spell,
				};

				let casting_stats = match has_casting {
					false => html!(),
					true => {
						let field_rank = {
							let select_rank = Callback::from({
								let min_rank = spell.rank;
								let select_rank = select_rank.clone();
								move |evt: web_sys::Event| {
									let Some(selected_rank) = evt.select_value_t::<u8>() else {
										return;
									};
									select_rank
										.emit((contained_idx, (selected_rank != min_rank).then_some(selected_rank)));
								}
							});
							let selected_rank = rank.unwrap_or(spell.rank);
							let rank_min = spell.rank.max(rank_min);
							let rank_max = spell.rank.max(match remaining_total_rank {
								None => rank_max,
								Some(remaining_total_rank) => {
									rank_max.min(selected_rank.saturating_add(*remaining_total_rank as u8))
								}
							});
							html! {
								<select class="form-select px-2 py-1" onchange={select_rank}>
									{(rank_min..=rank_max).into_iter().map(|option_rank| {
										html! {
											<option selected={option_rank == selected_rank} value={option_rank.to_string()}>
												{"Rank "}{option_rank}
											</option>
										}
									}).collect::<Vec<_>>()}
								</select>
							}
						};
						let field_atk_bonus = {
							let select_atk_bonus = Callback::from({
								let select_atk_bonus = select_atk_bonus.clone();
								move |evt: web_sys::Event| {
									let Some(value) = evt.select_value() else {
										return;
									};
									if value.is_empty() {
										return;
									}
									let Ok(value) = value.parse::<i32>() else {
										return;
									};
									select_atk_bonus.emit((contained_idx, Some(value)));
								}
							});
							let is_fixed = casting_atk_bonus.is_some();
							let attack_bonus = casting_atk_bonus.or(*attack_bonus);
							let mut class = classes!("form-select", "px-2", "py-1");
							let selected = {
								let mut selected = 0;
								if let Some(value) = attack_bonus {
									selected = value;
								} else {
									class.push("missing-value");
								}
								selected
							};
							html! {
								<select {class} onchange={select_atk_bonus} disabled={is_fixed}>
									<option selected={attack_bonus.is_none()} value="" />
									{(-20..=20).into_iter().map(|bonus| {
										html! {
											<option selected={bonus == selected} value={bonus.to_string()}>
												{format!("{bonus:+}")}
											</option>
										}
									}).collect::<Vec<_>>()}
								</select>
							}
						};
						let field_save_dc = {
							let select_save_dc = Callback::from({
								let select_save_dc = select_save_dc.clone();
								move |evt: web_sys::Event| {
									let Some(value) = evt.select_value() else {
										return;
									};
									if value.is_empty() {
										return;
									}
									let Ok(value) = value.parse::<u8>() else {
										return;
									};
									select_save_dc.emit((contained_idx, Some(value)));
								}
							});
							let is_fixed = casting_dc.is_some();
							let save_dc = casting_dc.or(*save_dc);
							let mut class = classes!("form-select", "px-2", "py-1");
							let selected = {
								let mut selected = 0;
								if let Some(value) = save_dc {
									selected = value;
								} else {
									class.push("missing-value");
								}
								selected
							};
							html! {
								<select {class} onchange={select_save_dc} disabled={is_fixed}>
									<option selected={save_dc.is_none()} value="" />
									{(0..=35).into_iter().map(|dc| {
										html! {
											<option selected={dc == selected} value={dc.to_string()}>
												{dc}
											</option>
										}
									}).collect::<Vec<_>>()}
								</select>
							}
						};
						html! {
							<div class="row flex-fill mt-1" style="font-size: 1rem;">
								<div class="col">
									<div class="text-center">{"Rank"}</div>
									{field_rank}
								</div>
								<div class="col">
									<div class="text-center">{"Attack Bonus"}</div>
									{field_atk_bonus}
								</div>
								<div class="col">
									<div class="text-center">{"Save DC"}</div>
									{field_save_dc}
								</div>
							</div>
						}
					}
				};

				let entry = spell_container.get_spell_entry(contained, Some((0, 0)));

				contained_spells.push(html! {
					<div class="feature-row border-bottom">
						<div class="flex-fill d-flex">
							<div class="column name-and-source">
								{spell_name_and_icons(&state, spell, entry.as_ref(), false)}
							</div>
							<button
								type="button" class="btn btn-danger btn-xs ms-2"
								onclick={remove_from_container.reform(move |_| contained_idx)}
							>
								<i class="bi bi-dash me-1" />
								{"Remove"}
							</button>
						</div>
						{spell_overview_info(&state, spell, entry.as_ref(), None)}
						{casting_stats}
					</div>
				});
			}
			html! {
				<div class="spell-section">
					{contained_spells}
				</div>
			}
		}
	}
}

#[derive(Clone, PartialEq, Properties)]
struct ModalSpellContainerAvailableListProps {
	id_path: ItemPath,
	fetch_indirect_spells: UseQueryDiscreteTypedHandle<Spell>,
}
#[function_component]
fn ModalSpellContainerAvailableList(props: &ModalSpellContainerAvailableListProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(item) = get_inventory_item(&state, &props.id_path) else {
		return Html::default();
	};
	let Some(spell_container) = &item.spells else {
		return Html::default();
	};

	let rank_min = spell_container.capacity.rank_min.unwrap_or(0);
	let rank_max = spell_container.capacity.rank_max.unwrap_or(MAX_SPELL_RANK);
	let container_capacity_criteria = Criteria::contains_prop(
		"rank",
		Criteria::any((rank_min..=rank_max).into_iter().map(|rank| Criteria::exact(rank))),
	);
	let criteria = use_state({
		let criteria = container_capacity_criteria.clone();
		move || criteria
	});

	let consumed_rank_sum: usize = spell_container
		.spells
		.iter()
		.map(|contained| match contained.rank {
			Some(fixed_rank) => fixed_rank as usize,
			None => {
				let spell = match &contained.spell {
					Indirect::Id(id) => match props.fetch_indirect_spells.status() {
						QueryStatus::Success((_ids, spells_by_id)) => spells_by_id.get(id),
						_ => None,
					},
					Indirect::Custom(spell) => Some(spell),
				};
				spell.map(|spell| spell.rank as usize).unwrap_or(0)
			}
		})
		.sum();
	let remaining_total_rank = spell_container.capacity.rank_total.map(|total| total.saturating_sub(consumed_rank_sum));
	let num_spells = spell_container.spells.len();
	let remaining_total_spells = spell_container.capacity.max_count.map(|total| total.saturating_sub(num_spells));

	let mut criteria_filter_btns = Vec::new();
	if spell_container.can_prepare_from {
		// casters which prepare from items
		let valid_casters =
			state.spellcasting().iter_casters().filter(|caster| caster.prepare_from_item).collect::<Vec<_>>();
		if !valid_casters.is_empty() {
			let set_filter_default = Callback::from({
				let default_criteria = container_capacity_criteria.clone();
				let criteria = criteria.clone();
				move |_| {
					criteria.set(default_criteria.clone());
				}
			});
			criteria_filter_btns.push(html! {
				<button type="button" class="btn btn-theme btn-xs mx-1" onclick={set_filter_default}>
					{"All Spells"}
				</button>
			});
			// NOTE: These filters do not abide by the rank bounds of the item.
			// This is fine for now because the only item which is prepare-from-able
			// is a wizard's spellbook (which has no rank bounds).
			// In the future, the filter system should abide by both item and caster criterias.
			for caster in valid_casters {
				let current_level = state.persistent().level(Some(caster.name()));
				let filter = state.spellcasting().get_filter(caster.name(), state.persistent()).unwrap_or_default();
				let set_filter = Callback::from({
					let criteria = criteria.clone();
					move |_| {
						criteria.set(filter.as_criteria());
					}
				});
				criteria_filter_btns.push(html! {
					<button type="button" class="btn btn-theme btn-xs mx-1" onclick={set_filter}>
						{format!("{} Lvl {current_level} spells", caster.name())}
					</button>
				});
			}
		}
	}

	html! {<>
		<div class="d-flex">
			{criteria_filter_btns}
		</div>
		<AvailableSpellList
			header_addon={HeaderAddon::from({
				let id_path = props.id_path.clone();
				move |spell: &Spell| -> Html {
					let has_rank_capacity = remaining_total_rank.map(|capacity| spell.rank as usize <= capacity).unwrap_or(true);
					let has_count_capacity = remaining_total_spells.map(|capacity| capacity > 0).unwrap_or(true);
					html! {
						<SpellListContainerAction
							container_id={id_path.clone()}
							spell_id={spell.id.unversioned()}
							{has_rank_capacity}
							{has_count_capacity}
						/>
					}
				}
			})}
			criteria={(*criteria).clone()}
			entry={None}
		/>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct SpellListContainerActionProps {
	container_id: ItemPath,
	spell_id: SourceId,
	has_rank_capacity: bool,
	has_count_capacity: bool,
}
#[function_component]
fn SpellListContainerAction(
	SpellListContainerActionProps {
		container_id,
		spell_id,
		has_rank_capacity,
		has_count_capacity,
	}: &SpellListContainerActionProps,
) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let Some(item) = get_inventory_item(&state, container_id) else {
		return Html::default();
	};
	let Some(spell_container) = &item.spells else {
		return Html::default();
	};
	let contained_ids = spell_container
		.spells
		.iter()
		.map(|contained| {
			match &contained.spell {
				Indirect::Id(id) => id,
				Indirect::Custom(spell) => &spell.id,
			}
			.unversioned()
		})
		.collect::<HashSet<_>>();

	let mut classes = classes!("btn", "btn-xs", "ms-auto");
	let (action_name, disabled) = match contained_ids.contains(&spell_id) {
		true => ("Added", true),
		false => match (has_rank_capacity, has_count_capacity) {
			(true, true) => ("Add", false),
			(false, true) => ("Not Enough Ranks", true),
			(true, false) => ("Not Enough Space", true),
			(false, false) => ("Full", true),
		},
	};
	if disabled {
		classes.push("btn-outline-secondary");
	} else {
		classes.push("btn-outline-theme");
	}

	let onclick = Callback::from({
		let state = state.clone();
		let container_id = container_id.clone();
		let spell_id = spell_id.clone();
		move |evt: MouseEvent| {
			evt.stop_propagation();
			state.dispatch({
				let container_id = container_id.clone();
				let spell_id = spell_id.clone();
				move |persistent: &mut Persistent| {
					let Some(item) = get_inventory_item_mut(persistent, &container_id) else {
						return MutatorImpact::None;
					};
					let Some(spell_container) = &mut item.spells else {
						return MutatorImpact::None;
					};
					spell_container.spells.push(ContainerSpell {
						spell: Indirect::Id(spell_id.clone()),
						rank: None,
						save_dc: None,
						attack_bonus: None,
					});
					// TODO: only recompile character when the modal is dismissed
					return MutatorImpact::Recompile;
				}
			});
		}
	});
	let onclick = (!disabled).then_some(onclick);

	html! {
		<button type="button" class={classes} {disabled} {onclick}>{action_name}</button>
	}
}
