use crate::{
	components::modal,
	system::{
		core::SourceId,
		dnd5e::{
			components::{panel::SpellFilter, GeneralProp, SharedCharacter},
			data::{
				bundle::{Background, Lineage, Race, RaceVariant, Upbringing},
				character::{ActionEffect, Persistent},
				description, Feature, Spell,
			},
			DnD5e,
		},
	},
	utility::{
		web_ext::{self, CallbackExt, CallbackOptExt},
		GenericMutator, InputExt, SelectorMeta, SelectorOptions,
	},
};
use multimap::MultiMap;
use std::{
	collections::{HashMap, HashSet},
	str::FromStr,
};
use yew::prelude::*;

static HELP_TEXT: &'static str = "Lineages and Upbingings are a replacement for races. \
They offer an expanded set of options around traits and features granted from \
the parents and community your character comes from.";

#[function_component]
pub fn OriginTab() -> Html {
	let use_lineages = use_state_eq(|| true);

	let toggle_lineages = web_ext::callback()
		.map(|evt: web_sys::Event| evt.input_checked())
		.on_some({
			let use_lineages = use_lineages.clone();
			move |checked| use_lineages.set(checked)
		});
	let lineages_switch = html! {
		<div class="form-check form-switch m-2">
			<label for="useLineages" class="form-check-label">{"Use Lineages & Upbringings"}</label>
			<input  id="useLineages" class="form-check-input"
				type="checkbox" role="switch"
				checked={*use_lineages}
				onchange={toggle_lineages}
			/>
			<div id="useLineagesHelpBlock" class="form-text">{HELP_TEXT}</div>
		</div>
	};

	html! {<>
		{lineages_switch}
		<CharacterContent />
		<CategoryBrowser use_lineages={*use_lineages} />
	</>}
}

#[function_component]
fn CharacterContent() -> Html {
	let state = use_context::<SharedCharacter>().unwrap();

	// TODO: temporarily hard-coded until all bundles are the same type
	let requirements = {
		let mut map = MultiMap::new();
		for variant in &state.persistent().named_groups.race_variant {
			for req in &variant.requirements {
				map.insert(
					req.clone(),
					("Race Variant".to_owned(), variant.name.clone()),
				);
			}
		}
		map
	};

	let mut entries = Vec::new();
	for idx in 0..state.persistent().named_groups.race.len() {
		entries.push(bundle_state::<Race>(
			&state,
			idx,
			"Race",
			&requirements,
			|persistent, idx| persistent.named_groups.race.get(idx),
			|value| &value.name,
			|_| None,
			race,
			|persistent, idx| {
				persistent.named_groups.race.remove(idx);
			},
		));
	}
	for idx in 0..state.persistent().named_groups.race_variant.len() {
		entries.push(bundle_state::<RaceVariant>(
			&state,
			idx,
			"Race Variant",
			&requirements,
			|persistent, idx| persistent.named_groups.race_variant.get(idx),
			|value| &value.name,
			|value| Some(&value.requirements),
			race_variant,
			|persistent, idx| {
				persistent.named_groups.race_variant.remove(idx);
			},
		));
	}
	for idx in 0..state.persistent().named_groups.lineage.len() {
		entries.push(bundle_state::<Lineage>(
			&state,
			idx,
			"Lineage",
			&requirements,
			|persistent, idx| persistent.named_groups.lineage.get(idx),
			|value| &value.name,
			|_| None,
			lineage,
			|persistent, idx| {
				persistent.named_groups.lineage.remove(idx);
			},
		));
	}
	for idx in 0..state.persistent().named_groups.upbringing.len() {
		entries.push(bundle_state::<Upbringing>(
			&state,
			idx,
			"Upbringing",
			&requirements,
			|persistent, idx| persistent.named_groups.upbringing.get(idx),
			|value| &value.name,
			|_| None,
			upbringing,
			|persistent, idx| {
				persistent.named_groups.upbringing.remove(idx);
			},
		));
	}
	for idx in 0..state.persistent().named_groups.background.len() {
		entries.push(bundle_state::<Background>(
			&state,
			idx,
			"Background",
			&requirements,
			|persistent, idx| persistent.named_groups.background.get(idx),
			|value| &value.name,
			|_| None,
			background,
			|persistent, idx| {
				persistent.named_groups.background.remove(idx);
			},
		));
	}

	if entries.is_empty() {
		return html! {};
	}

	html! {<>
		<div class="accordion mt-2 mb-4" id="selected-content">
			{entries}
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct CategoryBrowserProps {
	use_lineages: bool,
}

#[function_component]
fn CategoryBrowser(CategoryBrowserProps { use_lineages }: &CategoryBrowserProps) -> Html {
	let system = use_context::<UseStateHandle<DnD5e>>().unwrap();
	let state = use_context::<SharedCharacter>().unwrap();
	let selected_category = use_state(|| None::<AttrValue>);
	let content_list = match &*selected_category {
		None => html! {},
		Some(category) => html! {
			<div class="accordion my-2" id="all-entries">
				{match category.as_str() {
					"Race" => {
						content_list::<Race>(
							&system,
							&state,
							|system| &system.races,
							|value| &value.source_id,
							|value| &value.name,
							|_| 1,
							|_| None,
							race,
							|persistent| &persistent.named_groups.race,
							|persistent, item| persistent.named_groups.race.push(item),
						)
					}
					"Race Variant" => {
						content_list::<RaceVariant>(
							&system,
							&state,
							|system| &system.race_variants,
							|value| &value.source_id,
							|value| &value.name,
							|_| 1,
							|value| Some(&value.requirements),
							race_variant,
							|persistent| &persistent.named_groups.race_variant,
							|persistent, item| persistent.named_groups.race_variant.push(item),
						)
					}
					"Lineage" => {
						content_list::<Lineage>(
							&system,
							&state,
							|system| &system.lineages,
							|value| &value.source_id,
							|value| &value.name,
							|value| value.limit as usize,
							|_| None,
							lineage,
							|persistent| &persistent.named_groups.lineage,
							|persistent, item| persistent.named_groups.lineage.push(item),
						)
					}
					"Upbringing" => {
						content_list::<Upbringing>(
							&system,
							&state,
							|system| &system.upbringings,
							|value| &value.source_id,
							|value| &value.name,
							|_| 1,
							|_| None,
							upbringing,
							|persistent| &persistent.named_groups.upbringing,
							|persistent, item| persistent.named_groups.upbringing.push(item),
						)
					}
					"Background" => {
						content_list::<Background>(
							&system,
							&state,
							|system| &system.backgrounds,
							|value| &value.source_id,
							|value| &value.name,
							|_| 1,
							|_| None,
							background,
							|persistent| &persistent.named_groups.background,
							|persistent, item| persistent.named_groups.background.push(item),
						)
					}
					_ => html! {},
				}}
			</div>
		},
	};

	let mut options = vec!["Background"];
	match *use_lineages {
		true => {
			options.push("Lineage");
			options.push("Upbringing");
		}
		false => {
			options.push("Race");
			options.push("Race Variant");
		}
	}
	options.sort();

	html! {<>
		<CategoryPicker
			value={(*selected_category).clone()}
			options={options}
			on_change={Callback::from({
				let selected_category = selected_category.clone();
				move |value| selected_category.set(value)
			})}
		/>
		{content_list}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct CategoryPickerProps {
	options: Vec<&'static str>,
	value: Option<AttrValue>,
	on_change: Callback<Option<AttrValue>>,
}
#[function_component]
fn CategoryPicker(
	CategoryPickerProps {
		options,
		value,
		on_change,
	}: &CategoryPickerProps,
) -> Html {
	let on_selection_changed = Callback::from({
		let on_change = on_change.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.select_value() else { return; };
			on_change.emit((!value.is_empty()).then_some(value.into()));
		}
	});
	let close_btn = match value.is_some() {
		true => html! {
			<button type="button"
				class="btn btn-outline-warning"
				onclick={on_change.reform(|_| None)}
			>
				{"Close"}
			</button>
		},
		false => html! {},
	};
	html! {
		<div class="input-group">
			<span class="input-group-text">{"Browse Categories"}</span>
			<select class="form-select" onchange={on_selection_changed} disabled={value.is_some()}>
				<option
					value=""
					selected={value.is_none()}
				>{"Select a category..."}</option>
				{options.iter().map(|item| html! {
					<option
						value={item.clone()}
						selected={value.as_ref().map(AttrValue::as_str) == Some(*item)}
					>{item.clone()}</option>
				}).collect::<Vec<_>>()}
			</select>
			{close_btn}
		</div>
	}
}

fn bundle_state<T>(
	state: &SharedCharacter,
	idx: usize,
	category: &'static str,
	dependencies: &MultiMap<(String, String), (String, String)>,
	get_item: impl Fn(&Persistent, usize) -> Option<&T> + 'static,
	item_name: impl Fn(&T) -> &String + 'static,
	item_reqs: impl Fn(&T) -> Option<&Vec<(String, String)>> + 'static,
	item_body: impl Fn(&T, Option<&SharedCharacter>) -> Html + 'static,
	remove_item: impl Fn(&mut Persistent, usize) + 'static,
) -> Html {
	let Some(value) = get_item(state.persistent(), idx) else { return html! {}; };
	let value_name = item_name(value);
	let dependents = match dependencies.get_vec(&(category.to_owned(), value_name.clone())) {
		None => None,
		Some(reqs) => Some(
			reqs.iter()
				.map(|(category, name)| format!("{category}: {name}"))
				.collect::<Vec<_>>()
				.join(", "),
		),
	};

	let mut title = value_name.clone();
	if let Some(reqs) = item_reqs(value) {
		let reqs_as_str = reqs
			.iter()
			.map(|(category, name)| format!("{category}: {name}"))
			.collect::<Vec<_>>()
			.join(", ");
		title = format!("{title} (requires: [{}])", reqs_as_str);
	}
	html! {
		<ContentItem
			id_prefix={format!("item{}", idx)}
			name={format!("{}: {}", category, title)}
			kind={ContentItemKind::Remove {
				disable_selection: dependents.map(|desc| format!("Cannot remove, depended on by: {desc}").into()),
			}}
			on_click={Callback::from({
				let state = state.clone();
				let remove_item = std::sync::Arc::new(remove_item);
				move |_| {
					let remove_item = remove_item.clone();
					state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
						(*remove_item)(persistent, idx);
						Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
					}));
				}
			})}
		>
			{item_body(value, Some(state))}
		</ContentItem>
	}
}

fn content_list<T>(
	system: &UseStateHandle<DnD5e>,
	state: &SharedCharacter,
	get_items: impl Fn(&DnD5e) -> &HashMap<SourceId, T> + 'static,
	get_item_id: impl Fn(&T) -> &SourceId + 'static,
	get_item_name: impl Fn(&T) -> &String + 'static,
	get_item_limit: impl Fn(&T) -> usize + 'static,
	item_reqs: impl Fn(&T) -> Option<&Vec<(String, String)>> + 'static,
	item_body: impl Fn(&T, Option<&SharedCharacter>) -> Html + 'static,
	get_state_items: impl Fn(&Persistent) -> &Vec<T> + 'static,
	add_item: impl Fn(&mut Persistent, T) + 'static,
) -> Html
where
	T: 'static + Clone,
{
	let get_items = std::sync::Arc::new(get_items);
	let on_select = Callback::from({
		let system = system.clone();
		let state = state.clone();
		let get_items = get_items.clone();
		let add_item = std::sync::Arc::new(add_item);
		move |source_id| {
			let Some(source) = (*get_items)(&system).get(&source_id) else { return; };
			let new_value = source.clone();
			let add_item = add_item.clone();
			state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
				(*add_item)(persistent, new_value);
				Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
			}));
		}
	});

	let ordered_items = {
		let mut vec = get_items(&system).iter().collect::<Vec<_>>();
		vec.sort_by(|(_, a), (_, b)| get_item_name(a).partial_cmp(get_item_name(b)).unwrap());
		vec
	};

	html! {<>
		{ordered_items.into_iter().map(move |(source_id, value)| {
			let amount_selected = get_state_items(state.persistent()).iter().filter(|selected| {
				get_item_id(selected) == source_id
			}).count();
			let mut title = get_item_name(value).clone();
			let mut disable_selection = None;
			if let Some(reqs) = item_reqs(value) {
				let reqs_as_str = reqs.iter().map(|(category, name)| format!("{category}: {name}")).collect::<Vec<_>>().join(", ");
				title = format!("{title} (requires: [{}])", reqs_as_str);

				for (category, name) in reqs {
					let bundles = &state.persistent().named_groups;
					let names = match category.as_str() {
						"Race" => bundles.race.iter().map(|value| &value.name).collect::<Vec<_>>(),
						"RaceVariant" => bundles.race_variant.iter().map(|value| &value.name).collect::<Vec<_>>(),
						"Lineage" => bundles.lineage.iter().map(|value| &value.name).collect::<Vec<_>>(),
						"Upbringing" => bundles.upbringing.iter().map(|value| &value.name).collect::<Vec<_>>(),
						"Background" => bundles.background.iter().map(|value| &value.name).collect::<Vec<_>>(),
						_ => Vec::new(),
					};
					if names.into_iter().filter(|entry| *entry == name).count() == 0 {
						disable_selection = Some(format!("Requires {category}: {name}").into());
						break;
					}
				}
			}
			html! {
				<ContentItem
					parent_collapse={"#all-entries"}
					name={title}
					kind={ContentItemKind::Add {
						amount_selected,
						selection_limit: get_item_limit(value),
						disable_selection,
					}}
					on_click={on_select.reform({
						let source_id = source_id.clone();
						move |_| source_id.clone()
					})}
				>
					{item_body(value, None)}
				</ContentItem>
			}
		}).collect::<Vec<_>>()}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ContentItemProps {
	#[prop_or_default]
	parent_collapse: Option<AttrValue>,
	#[prop_or_default]
	id_prefix: Option<AttrValue>,
	name: AttrValue,
	kind: ContentItemKind,
	children: Children,
	on_click: Callback<()>,
}
#[derive(Clone, PartialEq)]
enum ContentItemKind {
	Add {
		amount_selected: usize,
		selection_limit: usize,
		disable_selection: Option<AttrValue>,
	},
	Remove {
		disable_selection: Option<AttrValue>,
	},
}
#[function_component]
fn ContentItem(
	ContentItemProps {
		parent_collapse,
		id_prefix,
		name,
		kind,
		children,
		on_click,
	}: &ContentItemProps,
) -> Html {
	use convert_case::{Case, Casing};

	let disabled_btn = |text: Html| {
		html! {
			<button type="button" class="btn btn-outline-secondary my-1 w-100" disabled={true}>
				{text}
			</button>
		}
	};
	let slot_buttons = match kind {
		ContentItemKind::Add {
			amount_selected,
			selection_limit,
			disable_selection,
		} => {
			let select_btn = |text: Html| {
				html! {
					<button type="button" class="btn btn-outline-success my-1 w-100" onclick={on_click.reform(|_| ())}>
						{text}
					</button>
				}
			};

			match (disable_selection, *amount_selected, *selection_limit) {
				(Some(reason), _, _) => disabled_btn(html! {{reason.clone()}}),
				// Slot is empty, and this option is not-yet used
				(_, 0, _) => select_btn(html! {{"Add"}}),
				// Slot is empty, this option is in another slot, but it can be used again
				(_, count, limit) if count < limit => {
					select_btn(html! {{format!("Add Again ({} / {})", count, limit)}})
				}
				// option already selected for another slot, and cannot be selected again
				(_, count, limit) if count >= limit => {
					disabled_btn(html! {{format!("Cannot Add Again ({} / {})", count, limit)}})
				}
				_ => html! {},
			}
		}
		ContentItemKind::Remove { disable_selection } => match disable_selection {
			Some(reason) => disabled_btn(html! {{reason.clone()}}),
			None => html! {
				<button type="button" class="btn btn-outline-danger my-1 w-100" onclick={on_click.reform(|_| ())}>
					{"Remove"}
				</button>
			},
		},
	};

	let id = format!(
		"{}{}",
		id_prefix
			.as_ref()
			.map(AttrValue::as_str)
			.unwrap_or_default(),
		name.as_str().to_case(Case::Kebab),
	);
	html! {
		<div class="accordion-item">
			<h2 class="accordion-header">
				<button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target={format!("#{id}")}>
					{name.clone()}
				</button>
			</h2>
			<div {id} class="accordion-collapse collapse" data-bs-parent={parent_collapse.clone()}>
				<div class="accordion-body">
					<div>{slot_buttons}</div>
					{children.clone()}
				</div>
			</div>
		</div>
	}
}

fn race(value: &Race, state: Option<&SharedCharacter>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
		{value.features.iter().map(|f| feature(f, state)).collect::<Vec<_>>()}
	</>}
}

fn race_variant(value: &RaceVariant, state: Option<&SharedCharacter>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
		{value.features.iter().map(|f| feature(f, state)).collect::<Vec<_>>()}
	</>}
}

fn lineage(value: &Lineage, state: Option<&SharedCharacter>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
		{value.features.iter().map(|f| feature(f, state)).collect::<Vec<_>>()}
	</>}
}

fn upbringing(value: &Upbringing, state: Option<&SharedCharacter>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
		{value.features.iter().map(|f| feature(f, state)).collect::<Vec<_>>()}
	</>}
}

fn background(value: &Background, state: Option<&SharedCharacter>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
		{value.features.iter().map(|f| feature(f, state)).collect::<Vec<_>>()}
	</>}
}

pub fn feature(value: &Feature, state: Option<&SharedCharacter>) -> Html {
	let desc = match (state, value.description.clone()) {
		(Some(state), desc) => desc.evaluate(state),
		(None, desc) => desc,
	};

	let criteria = {
		let criteria = value.criteria.as_ref();
		let desc = criteria.map(|eval| eval.description()).flatten();
		desc.map(|text| {
			html! {
				<div class="property">
					<strong>{"Criteria"}</strong>
					{text}
				</div>
			}
		})
	};

	html! {
		<div class="my-2">
			<h5>{value.name.clone()}</h5>
			{description(&desc, false)}
			{mutator_list(&value.mutators, state)}
			{criteria.unwrap_or_default()}
		</div>
	}
}

// TODO: Unify with DescriptionSection
pub fn description(info: &description::Info, prefer_short: bool) -> Html {
	if prefer_short {
		if let Some(desc) = &info.short {
			return html! { <div class="text-block">{desc}</div> };
		}
	}
	let sections = info
		.sections
		.iter()
		.map(|section| {
			html! { <DescriptionSection section={section.clone()} show_selectors={false} /> }
		})
		.collect::<Vec<_>>();
	html! {
		<div>
			{sections}
		</div>
	}
}

pub fn mutator_list<T: 'static>(
	list: &Vec<GenericMutator<T>>,
	state: Option<&impl AsRef<T>>,
) -> Html {
	let mutators = list
		.iter()
		.filter_map(|value| mutator(value, state))
		.collect::<Vec<_>>();
	html! {<>{mutators}</>}
}

fn mutator<T: 'static>(value: &GenericMutator<T>, state: Option<&impl AsRef<T>>) -> Option<Html> {
	let target = state.map(|t| t.as_ref());
	let section = value.description(target);
	Some(html! { <DescriptionSection {section} show_selectors={state.is_some()} /> })
}

#[derive(Clone, PartialEq, Properties)]
pub struct SectionProps {
	pub section: description::Section,
	pub show_selectors: bool,
}
#[function_component]
pub fn DescriptionSection(
	SectionProps {
		section,
		show_selectors,
	}: &SectionProps,
) -> Html {
	let name = match &section.title {
		None => None,
		Some(title) => Some(html! {<strong>{title.clone()}{". "}</strong>}),
	};

	let content = match &section.content {
		description::SectionContent::Body(text) => {
			html! {
				<div class="text-block">
					{text}
				</div>
			}
		}
		description::SectionContent::Selectors(selectors) => show_selectors
			.then(|| {
				if !selectors.errors().is_empty() {
					log::warn!(target: "utility", "Section has empty data path: {section:?}");
				}
				let iter_selectors = selectors.as_vec().iter();
				let fields = iter_selectors
					.map(|meta| html! { <SelectorField meta={meta.clone()} /> })
					.collect::<Vec<_>>();
				html! {
					<div>
						{fields}
					</div>
				}
			})
			.unwrap_or_default(),
		description::SectionContent::Table {
			column_count: _,
			headers,
			rows,
		} => {
			html! {
				<table class="table table-compact table-striped m-0">
					<thead>
						<tr class="text-center" style="color: var(--bs-heading-color);">
							{match headers.as_ref() {
								None => html!(),
								Some(headers) => html! {<>
									{headers.iter().map(|s| html! {
										<th scope="col">{s}</th>
									}).collect::<Vec<_>>()}
								</>},
							}}
						</tr>
					</thead>
					<tbody>
						{rows.iter().map(|cols| {
							html! { <tr>
								{cols.iter().map(|s| html! {
									<td>{s}</td>
								}).collect::<Vec<_>>()}
							</tr> }
						}).collect::<Vec<_>>()}
					</tbody>
				</table>
			}
		}
	};

	let children = (!section.children.is_empty()).then(|| {
		html! {
			<div class="ms-2">
				{section.children.iter().map(|section| html! {
					<DescriptionSection section={section.clone()} show_selectors={*show_selectors} />
				}).collect::<Vec<_>>()}
			</div>
		}
	});

	html! {
		<div>
			{name.unwrap_or_default()}
			{content}
			{children}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct SelectorFieldProps {
	meta: SelectorMeta,
}
#[function_component]
fn SelectorField(
	SelectorFieldProps {
		meta: SelectorMeta {
			name,
			data_path,
			options,
		},
	}: &SelectorFieldProps,
) -> Html {
	let state = use_context::<SharedCharacter>().unwrap();
	let modal_dispatcher = use_context::<modal::Context>().unwrap();
	let value = state.get_first_selection(data_path);

	let save_value = Callback::from({
		let data_path = data_path.clone();
		let state = state.clone();
		move |value| {
			let data_path = data_path.clone();
			state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
				match value {
					None => {
						persistent.selected_values.remove(&data_path);
					}
					Some(value) => {
						persistent.selected_values.set(&data_path, value);
					}
				}
				Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
			}));
		}
	});

	let classes = classes!("my-2", "selector");
	let missing_value = value
		.is_none()
		.then(|| classes!("missing-value"))
		.unwrap_or_default();
	let inner = match options {
		SelectorOptions::Any => {
			let onchange = Callback::from({
				let save_value = save_value.clone();
				move |evt: web_sys::Event| {
					let Some(value) = evt.input_value() else { return; };
					save_value.emit((!value.is_empty()).then_some(value.into()));
				}
			});
			html! {<input
				class="form-control" type="text"
				value={value.cloned().unwrap_or_default()}
				{onchange}
			/>}
		}
		SelectorOptions::AnyOf {
			options: valid_values,
			cannot_match,
			amount: _, // TODO: Display a different UI if amount > 1
		} => {
			let onchange = Callback::from({
				let save_value = save_value.clone();
				move |evt: web_sys::Event| {
					let Some(value) = evt.select_value() else { return; };
					save_value.emit((!value.is_empty()).then_some(value.into()));
				}
			});
			let invalid_values = match cannot_match {
				None => HashSet::new(),
				Some(selection_paths) => {
					let mut values = HashSet::new();
					for path in selection_paths {
						if let Some(value) = state.get_first_selection(path) {
							values.insert(value);
						}
					}
					values
				}
			};
			html! {
				<select class="form-select" {onchange}>
					<option
						value=""
						selected={value.is_none()}
					>{"Select a value..."}</option>
					{valid_values.iter().map(|item| {
						html! {
							<option
								value={item.clone()}
								selected={value == Some(item)}
								disabled={invalid_values.contains(item)}
							>
								{item.clone()}
							</option>
						}
					}).collect::<Vec<_>>()}
				</select>
			}
		}
		SelectorOptions::Object {
			category,
			count: capacity,
			spell_filter,
		} => {
			let browse = modal_dispatcher.callback({
				let data_path = data_path.clone();
				let category: AttrValue = category.clone().into();
				let capacity = *capacity;
				let spell_filter = spell_filter.clone();
				move |_| {
					let data_path = data_path.clone();
					let category = category.clone();
					let filter = spell_filter.clone();
					modal::Action::Open(modal::Props {
						centered: true,
						scrollable: true,
						root_classes: classes!("browse", "objects"),
						content: html! {<ModalObjectBrowser {data_path} {category} {capacity} {filter} />},
						..Default::default()
					})
				}
			});
			let btn_classes = classes!("btn", "btn-outline-theme", "btn-xs", missing_value);
			let selection_count = state
				.get_selections_at(data_path)
				.map(|list| list.len())
				.unwrap_or_default();
			return html! {
				<div class={classes}>
					<h6>{name.clone()}</h6>
					<button type="button" class={btn_classes} onclick={browse}>
						{format!("Browse ({}/{capacity} selected)", selection_count)}
					</button>
					<ObjectSelectorList value={data_path.clone()} />
				</div>
			};
		}
	};
	html! {
		<div class={classes!("input-group", classes, missing_value)} style="max-width: 300px;">
			<span class="input-group-text">{name.clone()}</span>
			{inner}
		</div>
	}
}

#[function_component]
fn ObjectSelectorList(props: &GeneralProp<std::path::PathBuf>) -> Html {
	let state = use_context::<SharedCharacter>().unwrap();
	let system = use_context::<UseStateHandle<DnD5e>>().unwrap();
	let mut entries = Vec::new();
	if let Some(selected_values) = state.get_selections_at(&props.value) {
		for id_str in selected_values {
			let Ok(id) = SourceId::from_str(id_str) else { continue; };
			// TODO: Get this from the database, not DnD5e in-memory
			let Some(spell) = system.spells.get(&id) else { continue; };
			entries.push(html! {
				<li>
					{&spell.name}
				</li>
			})
		}
	}
	html! {
		<ul class="mb-0">
			{entries}
		</ul>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct ModalObjectBrowserProps {
	data_path: std::path::PathBuf,
	category: AttrValue,
	capacity: usize,
	filter: Option<SpellFilter>,
}
#[function_component]
fn ModalObjectBrowser(props: &ModalObjectBrowserProps) -> Html {
	use crate::system::dnd5e::components::panel::{AvailableSpellList, HeaderAddon};

	// TODO: This modal should query the database and check for objects with provided category that meet the provided criteria,
	// checking against database metadata instead of the actual parsed objects.
	if props.category.as_str() != "spell" {
		return html! {<>
			<div class="modal-header">
				<h1 class="modal-title fs-4">{"Unsupported object category"}</h1>
				<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
			</div>
		</>};
	}

	let header_addon = HeaderAddon::from({
		let data_path = props.data_path.clone();
		let capacity = props.capacity;
		move |spell: &Spell| -> Html {
			html! {
				<ObjectSelectorEntryButton
					data_path={data_path.clone()}
					id={spell.id.clone()}
					{capacity}
				/>
			}
		}
	});
	html! {<>
		<div class="modal-header">
			<h1 class="modal-title fs-4">{"Browse Spells"}</h1>
			<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
		</div>
		<div class="modal-body spell-list">
			<AvailableSpellList
				{header_addon}
				filter={props.filter.clone().unwrap_or_default()}
			/>
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ObjectSelectorEntryButtonProps {
	data_path: std::path::PathBuf,
	id: SourceId,
	capacity: usize,
}
#[function_component]
fn ObjectSelectorEntryButton(props: &ObjectSelectorEntryButtonProps) -> Html {
	let state = use_context::<SharedCharacter>().unwrap();

	let props_id = props.id.to_string();
	let mut selected_idx = None;
	let mut can_select_more = props.capacity > 0;

	if let Some(entries) = state.get_selections_at(&props.data_path) {
		can_select_more = entries.len() < props.capacity;
		for (idx, id_str) in entries.iter().enumerate() {
			if id_str.as_str() == props_id.as_str() {
				selected_idx = Some(idx);
				break;
			}
		}
	}

	let is_selected = selected_idx.is_some();
	let onclick = state.new_dispatch({
		let data_path = props.data_path.clone();
		move |evt: MouseEvent, persistent, _| {
			evt.stop_propagation();
			match selected_idx {
				None => {
					persistent.insert_selection(&data_path, props_id.clone());
				}
				Some(idx) => {
					persistent.remove_selection(&data_path, idx);
				}
			}
			// recompile required because mutators which have object selections
			// probably need to use those selections to affect derived data
			// (e.g. spellcasting add_prepared)
			Some(ActionEffect::Recompile)
		}
	});

	let mut classes = classes!("btn", "btn-xs");
	let disabled = !is_selected && !can_select_more;
	classes.push(match is_selected {
		true => "btn-outline-theme",
		false => match can_select_more {
			true => "btn-theme",
			false => "btn-outline-secondary",
		},
	});

	html! {
		<button
			type="button" class={classes}
			style={"margin-left: auto; width: 85px;"}
			{disabled} {onclick}
		>
			{match is_selected {
				true => "Selected",
				false => "Select",
			}}
		</button>
	}
}
