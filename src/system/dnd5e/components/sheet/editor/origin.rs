use crate::system::dnd5e::{
	components::SharedCharacter,
	data::{
		character::{ActionEffect, Persistent},
		Lineage, Upbringing,
	},
	DnD5e,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

static HELP_TEXT: &'static str = "Lineages and Upbingings are a replacement for races. \
They offer an expanded set of options around traits and features granted from \
the parents and community your character comes from.";

#[function_component]
pub fn OriginTab() -> Html {
	let use_lineages = use_state_eq(|| true);

	let toggle_lineages = Callback::from({
		let use_lineages = use_lineages.clone();
		move |evt: web_sys::Event| {
			let Some(target) = evt.target() else { return; };
			let Some(input) = target.dyn_ref::<HtmlInputElement>() else { return; };
			use_lineages.set(input.checked());
		}
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
		<CategoryBrowser />
	</>}
}

#[function_component]
fn CharacterContent() -> Html {
	let state = use_context::<SharedCharacter>().unwrap();

	let mut entries = Vec::new();
	for idx in 0..state.persistent().named_groups.lineage.len() {
		entries.push(html! { <LineageState {idx} /> });
	}
	for idx in 0..state.persistent().named_groups.upbringing.len() {
		entries.push(html! { <UpbringingState {idx} /> });
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
struct StateProps {
	idx: usize,
}
#[function_component]
fn LineageState(StateProps { idx }: &StateProps) -> Html {
	let state = use_context::<SharedCharacter>().unwrap();
	let Some(value) = state.persistent().named_groups.lineage.get(*idx) else { return html! {}; };
	html! {
		<ContentItem
			//parent_collapse={"#selected-content"}
			id_prefix={format!("item{}", *idx)}
			name={format!("Lineage: {}", value.name)}
			kind={ContentItemKind::Remove}
			on_click={Callback::from({
				let state = state.clone();
				let idx = *idx;
				move |_| {
					state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
						persistent.named_groups.lineage.remove(idx);
						Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
					}));
				}
			})}
		>
			<LineageBody value={value.clone()} />
		</ContentItem>
	}
}
#[function_component]
fn UpbringingState(StateProps { idx }: &StateProps) -> Html {
	let state = use_context::<SharedCharacter>().unwrap();
	let Some(value) = state.persistent().named_groups.upbringing.get(*idx) else { return html! {}; };
	html! {
		<ContentItem
			//parent_collapse={"#selected-content"}
			id_prefix={format!("item{}", *idx)}
			name={format!("Upbringing: {}", value.name)}
			kind={ContentItemKind::Remove}
			on_click={Callback::from({
				let state = state.clone();
				let idx = *idx;
				move |_| {
					state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
						persistent.named_groups.upbringing.remove(idx);
						Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
					}));
				}
			})}
		>
			<UpbringingBody value={value.clone()} />
		</ContentItem>
	}
}

#[function_component]
fn CategoryBrowser() -> Html {
	let selected_category = use_state(|| None::<AttrValue>);
	let content_list = match &*selected_category {
		None => html! {},
		Some(category) => html! {
			<div class="accordion my-2" id="all-entries">
				{match category.as_str() {
					"Lineage" => html! {<ContentListLineage />},
					"Upbringing" => html! {<ContentListUpbringing />},
					"Background" => html! {},
					_ => html! {},
				}}
			</div>
		},
	};
	html! {<>
		<CategoryPicker
			value={(*selected_category).clone()}
			options={vec!["Lineage".into(), "Upbringing".into(), "Background".into()]}
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
	options: Vec<AttrValue>,
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
			let Some(target) = evt.target() else { return; };
			let Some(element) = target.dyn_ref::<HtmlSelectElement>() else { return; };
			let value = element.value();
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
						selected={value.as_ref() == Some(item)}
					>{item.clone()}</option>
				}).collect::<Vec<_>>()}
			</select>
			{close_btn}
		</div>
	}
}

#[function_component]
fn ContentListLineage() -> Html {
	let system = use_context::<UseStateHandle<DnD5e>>().unwrap();
	let state = use_context::<SharedCharacter>().unwrap();

	let on_select = Callback::from({
		let system = system.clone();
		let state = state.clone();
		move |source_id| {
			let Some(source) = system.lineages.get(&source_id) else { return; };
			let new_value = source.clone();
			state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
				persistent.named_groups.lineage.push(new_value);
				Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
			}));
		}
	});

	let ordered_items = {
		let mut vec = system.lineages.iter().collect::<Vec<_>>();
		vec.sort_by(|(_, a), (_, b)| a.name.partial_cmp(&b.name).unwrap());
		vec
	};

	html! {<>
		{ordered_items.into_iter().map(move |(source_id, value)| {
			let amount_selected = state.persistent().named_groups.lineage.iter().filter(|selected| {
				&selected.source_id == source_id
			}).count();
			html! {
				<ContentItem
					parent_collapse={"#all-entries"}
					name={value.name.clone()}
					kind={ContentItemKind::Add {
						amount_selected,
						selection_limit: value.limit as usize,
					}}
					on_click={on_select.reform({
						let source_id = source_id.clone();
						move |_| source_id.clone()
					})}
				>
					<LineageBody value={value.clone()} />
				</ContentItem>
			}
		}).collect::<Vec<_>>()}
	</>}
}

#[function_component]
fn ContentListUpbringing() -> Html {
	let system = use_context::<UseStateHandle<DnD5e>>().unwrap();
	let state = use_context::<SharedCharacter>().unwrap();

	let on_select = Callback::from({
		let system = system.clone();
		let state = state.clone();
		move |source_id| {
			let Some(source) = system.upbringings.get(&source_id) else { return; };
			let new_value = source.clone();
			state.dispatch(Box::new(move |persistent: &mut Persistent, _| {
				persistent.named_groups.upbringing.push(new_value);
				Some(ActionEffect::Recompile) // TODO: Only do this when returning to sheet view
			}));
		}
	});

	let ordered_items = {
		let mut vec = system.upbringings.iter().collect::<Vec<_>>();
		vec.sort_by(|(_, a), (_, b)| a.name.partial_cmp(&b.name).unwrap());
		vec
	};

	html! {<>
		{ordered_items.into_iter().map(move |(source_id, value)| {
			let amount_selected = state.persistent().named_groups.upbringing.iter().filter(|selected| {
				&selected.source_id == source_id
			}).count();
			html! {
				<ContentItem
					parent_collapse={"#all-entries"}
					name={value.name.clone()}
					kind={ContentItemKind::Add {
						amount_selected,
						selection_limit: 1,
					}}
					on_click={on_select.reform({
						let source_id = source_id.clone();
						move |_| source_id.clone()
					})}
				>
					<UpbringingBody value={value.clone()} />
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
	},
	Remove,
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

	let slot_buttons = match kind {
		ContentItemKind::Add {
			amount_selected,
			selection_limit,
		} => {
			let disabled_btn = |text: Html| {
				html! {
					<button type="button" class="btn btn-outline-secondary my-1 w-100" disabled={true}>
						{text}
					</button>
				}
			};
			let select_btn = |text: Html| {
				html! {
					<button type="button" class="btn btn-outline-success my-1 w-100" onclick={on_click.reform(|_| ())}>
						{text}
					</button>
				}
			};

			match (*amount_selected, *selection_limit) {
				// Slot is empty, and this option is not-yet used
				(0, _) => select_btn(html! {{"Add"}}),
				// Slot is empty, this option is in another slot, but it can be used again
				(count, limit) if count < limit => {
					select_btn(html! {{format!("Add Again ({} / {})", count, limit)}})
				}
				// option already selected for another slot, and cannot be selected again
				(count, limit) if count >= limit => {
					disabled_btn(html! {{format!("Cannot Add Again ({} / {})", count, limit)}})
				}
				_ => html! {},
			}
		}
		ContentItemKind::Remove => {
			html! {
				<button type="button" class="btn btn-outline-danger my-1 w-100" onclick={on_click.reform(|_| ())}>
					{"Remove"}
				</button>
			}
		}
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

#[derive(Clone, PartialEq, Properties)]
struct LineageBodyProps {
	value: Lineage,
}
#[function_component]
fn LineageBody(LineageBodyProps { value }: &LineageBodyProps) -> Html {
	let mutator_descs = value
		.mutators
		.iter()
		.filter_map(|v| v.description())
		.map(|desc| {
			html! {
				<li>{desc}</li>
			}
		})
		.collect::<Vec<_>>();
	for mutator in &value.mutators {
		if let Some(_meta) = mutator.selector_meta() {
			// TODO: We need to know what the key of the selector is,
			// which requires not only the selector's id (which we have),
			// but the full path to the mutator from the character root.
			// for lineages this is just `<lineage name>/<mutator name>/<selector id>`,
			// but this is going to be an ongoing issue for all mutator groups.
			// Unfortunately, we only have that path when we iterate over the entire tree during a `Character::apply`.
		}
	}
	html! {<>
		<div style="white-space: pre-line;">
			{value.description.clone()}
		</div>
		{mutator_descs}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct UpbringingBodyProps {
	value: Upbringing,
}
#[function_component]
fn UpbringingBody(UpbringingBodyProps { value }: &UpbringingBodyProps) -> Html {
	html! {<>
		<div style="white-space: pre-line;">
			{value.description.clone()}
		</div>
	</>}
}
