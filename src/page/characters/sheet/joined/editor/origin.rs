use crate::{
	components::{
		context_menu,
		database::{use_query_all_typed, use_typed_fetch_callback, QueryAllArgs, QueryStatus},
		object_browser::{self, ObjectSelectorList},
		Spinner,
	},
	database::Criteria,
	page::characters::sheet::{CharacterHandle, MutatorImpact},
	system::{
		dnd5e::{
			components::GeneralProp,
			data::{character::Persistent, description, Bundle, Feature},
			evaluator::HasBundle,
			DnD5e,
		},
		mutator, System,
	},
	utility::{
		selector,
		web_ext::{self, CallbackExt, CallbackOptExt},
		InputExt,
	},
};
use convert_case::{Case, Casing};
use multimap::MultiMap;

use std::sync::Arc;
use yew::prelude::*;

static HELP_TEXT: &'static str = "Lineages and Upbingings are a replacement for races. \
They offer an expanded set of options around traits and features granted from \
the parents and community your character comes from.";

#[function_component]
pub fn OriginTab() -> Html {
	let use_lineages = use_state_eq(|| true);

	let toggle_lineages = web_ext::callback().map(|evt: web_sys::Event| evt.input_checked()).on_some({
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
	let state = use_context::<CharacterHandle>().unwrap();

	// Map of things required (Bundle Category, Name) to that which requires them (Bundle Category, Name)
	let requirements = {
		let mut map = MultiMap::new();
		for bundle in &state.persistent().bundles {
			for generic_requirement in &bundle.requirements {
				let Some(has_bundle) = generic_requirement.as_any().downcast_ref::<Arc<HasBundle>>() else { continue };
				map.insert((&has_bundle.category, &has_bundle.name), (&bundle.category, &bundle.name));
			}
		}
		map
	};

	if state.persistent().bundles.is_empty() {
		return html! {};
	}
	html! {<>
		<div class="accordion mt-2 mb-4" id="selected-content">
			{state.persistent().bundles.iter().enumerate().map(|(idx, bundle)| {
				selected_bundle(&state, idx, bundle, &requirements)
			}).collect::<Vec<_>>()}
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct CategoryBrowserProps {
	use_lineages: bool,
}

#[function_component]
fn CategoryBrowser(CategoryBrowserProps { use_lineages: _ }: &CategoryBrowserProps) -> Html {
	let selected_category = use_state(|| None::<AttrValue>);
	let query_bundles = use_query_all_typed::<Bundle>(false, None);
	// Query for bundles when the category changes
	use_effect_with(selected_category.clone(), {
		let query_bundles = query_bundles.clone();
		move |selected: &UseStateHandle<Option<AttrValue>>| {
			if selected.is_some() {
				let criteria = {
					let category = selected.as_ref().map(AttrValue::as_str).unwrap().to_owned();
					let matches_category = Criteria::Exact(category.into());
					Some(Criteria::ContainsProperty("category".into(), matches_category.into()).into())
				};
				let query_args = QueryAllArgs::<Bundle> {
					system: DnD5e::id().into(),
					criteria,
					adjust_listings: Some(Arc::new(|mut bundles| {
						bundles.sort_by(|a, b| a.name.cmp(&b.name));
						bundles
					})),
					..Default::default()
				};
				query_bundles.run(query_args);
			} else {
				query_bundles.update(vec![]);
			}
		}
	});

	let options = vec!["Race", "Race Variant", "Lineage", "Upbringing", "Background", "Feat"];
	html! {<>
		<CategoryPicker
			value={(*selected_category).clone()}
			options={options}
			on_change={Callback::from({
				let selected_category = selected_category.clone();
				move |value| selected_category.set(value)
			})}
		/>
		{match query_bundles.status() {
			QueryStatus::Pending => html!(<Spinner />),
			QueryStatus::Empty | QueryStatus::Failed(_) => html!("No available bundles"),
			QueryStatus::Success(bundles) => html! {
				<div class="accordion my-2" id="all-entries">
					{bundles.iter().map(|bundle| {
						html!(<AvailableBundle value={bundle.clone()} />)
					}).collect::<Vec<_>>()}
				</div>
			},
		}}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct CategoryPickerProps {
	options: Vec<&'static str>,
	value: Option<AttrValue>,
	on_change: Callback<Option<AttrValue>>,
}
#[function_component]
fn CategoryPicker(CategoryPickerProps { options, value, on_change }: &CategoryPickerProps) -> Html {
	let on_selection_changed = Callback::from({
		let on_change = on_change.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.select_value() else {
				return;
			};
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
						value={item.to_owned()}
						selected={value.as_ref().map(AttrValue::as_str) == Some(*item)}
					>{item}</option>
				}).collect::<Vec<_>>()}
			</select>
			{close_btn}
		</div>
	}
}

fn selected_bundle(
	state: &CharacterHandle, idx: usize, bundle: &Bundle,
	all_dependents: &MultiMap<(&String, &String), (&String, &String)>,
) -> Html {
	let dependents = match all_dependents.get_vec(&(&bundle.category, &bundle.name)) {
		None => None,
		Some(reqs) => {
			Some(reqs.iter().map(|(category, name)| format!("{category}: {name}")).collect::<Vec<_>>().join(", "))
		}
	};

	let bundle_id = bundle.name.to_case(Case::Kebab).replace("(", "").replace(")", "").replace(":", "");
	html! {
		<ContentItem
			id={format!("{}-{idx}-{bundle_id}", bundle.category)}
			name={format!("{}: {}", bundle.category, bundle.name)}
			kind={ContentItemKind::Remove {
				disable_selection: dependents.map(|desc| format!("Cannot remove, depended on by: {desc}").into()),
			}}
			on_click={state.new_dispatch(move |_, persistent| {
				log::debug!("remove bundle {idx}");
				persistent.bundles.remove(idx);
				MutatorImpact::Recompile // TODO: Only do this when returning to sheet view
			})}
		>
			{bundle_content(bundle, Some(state), true)}
		</ContentItem>
	}
}

#[function_component]
fn AvailableBundle(GeneralProp { value: bundle }: &GeneralProp<Bundle>) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let on_select = use_typed_fetch_callback(
		"Add Bundle".into(),
		state.new_dispatch(|bundle: Bundle, persistent| {
			persistent.bundles.push(bundle);
			MutatorImpact::Recompile // TODO: Only do this when returning to sheet view
		}),
	);

	let amount_selected = state.persistent().bundles.iter().filter(|selected| selected.id == bundle.id).count();
	let mut title = bundle.name.clone();
	let mut unmet_requirements = Vec::with_capacity(bundle.requirements.len());
	if !bundle.requirements.is_empty() {
		for req in &bundle.requirements {
			if let Err(_) = req.evaluate(&*state) {
				if let Some(desc) = req.description() {
					unmet_requirements.push(desc);
				}
			}
		}
		if !unmet_requirements.is_empty() {
			title = format!("{title} (missing: [{}])", unmet_requirements.join(", "));
		}
	}
	let bundle_id = bundle.name.to_case(Case::Kebab).replace("(", "").replace(")", "").replace(":", "");
	html! {
		<ContentItem
			parent_collapse={"#all-entries"}
			id={bundle_id}
			name={title}
			kind={ContentItemKind::Add {
				amount_selected,
				selection_limit: bundle.limit,
				disable_selection: (!unmet_requirements.is_empty()).then(|| "Requirements not met".into()),
			}}
			on_click={on_select.reform({
				let source_id = bundle.id.unversioned();
				move |_| source_id.clone()
			})}
		>
			{bundle_content(bundle, Some(&state), false)}
		</ContentItem>
	}
}
pub fn bundle_content(bundle: &Bundle, state: Option<&CharacterHandle>, selectors: bool) -> Html {
	let mut requirements = Vec::with_capacity(bundle.requirements.len());
	for requirement in &bundle.requirements {
		let Some(content) = requirement.description() else { continue };
		let icon_status = state.map(|state| requirement.evaluate(state));
		requirements.push(html!(<div>
			{icon_status.map(|result| match result {
				Ok(()) => html!(<i class="bi bi-check-square" style="color: green;" />),
				Err(_err) => html!(<i class="bi bi-x-square" style="color: red;" />),
			})}
			{content}
		</div>));
	}
	html! {<>
		<div class="text-block mb-2">
			{description(&bundle.description, false, selectors)}
		</div>
		{(!requirements.is_empty()).then(|| html!(<div class="text-block mb-2">
			<strong>{"Requirements:"}</strong>
			{requirements}
		</div>))}
		{mutator_list(&bundle.mutators, state)}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ContentItemProps {
	#[prop_or_default]
	parent_collapse: Option<AttrValue>,
	id: AttrValue,
	name: AttrValue,
	kind: ContentItemKind,
	children: Children,
	on_click: Callback<()>,
}
#[derive(Clone, PartialEq)]
enum ContentItemKind {
	Add { amount_selected: usize, selection_limit: usize, disable_selection: Option<AttrValue> },
	Remove { disable_selection: Option<AttrValue> },
}
#[function_component]
fn ContentItem(ContentItemProps { parent_collapse, id, name, kind, children, on_click }: &ContentItemProps) -> Html {
	let disabled_btn = |text: Html| {
		html! {
			<button type="button" class="btn btn-outline-secondary my-1 w-100" disabled={true}>
				{text}
			</button>
		}
	};
	let slot_buttons = match kind {
		ContentItemKind::Add { amount_selected, selection_limit, disable_selection } => {
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

	html! {
		<div class="accordion-item">
			<h2 class="accordion-header">
				<button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target={format!("#{id}")}>
					{name.clone()}
				</button>
			</h2>
			<div id={id.clone()} class="accordion-collapse collapse" data-bs-parent={parent_collapse.clone()}>
				<div class="accordion-body">
					<div>{slot_buttons}</div>
					{children.clone()}
				</div>
			</div>
		</div>
	}
}

pub fn feature(value: &Feature, state: Option<&CharacterHandle>) -> Html {
	let desc = match (state, value.description.clone()) {
		(Some(state), desc) => desc.evaluate(state),
		(None, desc) => desc,
	};

	html! {
		<div class="my-2">
			<h5>{value.name.clone()}</h5>
			{description(&desc, false, false)}
			{mutator_list(&value.mutators, state)}
		</div>
	}
}

// TODO: Unify with DescriptionSection
pub fn description(info: &description::Info, prefer_short: bool, show_selectors: bool) -> Html {
	if prefer_short {
		if let Some(desc) = &info.short {
			return html! { <div class="text-block">{desc}</div> };
		}
	}
	let sections = info
		.sections
		.iter()
		.map(|section| {
			html! { <DescriptionSection section={section.clone()} {show_selectors} /> }
		})
		.collect::<Vec<_>>();
	html! {
		<div>
			{sections}
		</div>
	}
}

pub fn mutator_list<T: 'static>(list: &Vec<mutator::Generic<T>>, state: Option<&impl AsRef<T>>) -> Html {
	let mutators = list.iter().filter_map(|value| mutator(value, state)).collect::<Vec<_>>();
	html! {<>{mutators}</>}
}

fn mutator<T: 'static>(value: &mutator::Generic<T>, state: Option<&impl AsRef<T>>) -> Option<Html> {
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
pub fn DescriptionSection(SectionProps { section, show_selectors }: &SectionProps) -> Html {
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
					.map(|data_option| html! { <SelectorField data={data_option.clone()} /> })
					.collect::<Vec<_>>();
				html! {
					<div>
						{fields}
					</div>
				}
			})
			.unwrap_or_default(),
		description::SectionContent::Table { column_count: _, headers, rows } => {
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
	data: selector::DataOption,
}
#[function_component]
fn SelectorField(
	SelectorFieldProps { data: selector::DataOption { name, data_path, kind } }: &SelectorFieldProps,
) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();
	let active_details = use_context::<context_menu::ActiveContext>();
	let value = state.get_first_selection(data_path);

	let save_value = Callback::from({
		let data_path = data_path.clone();
		let state = state.clone();
		move |value| {
			let data_path = data_path.clone();
			state.dispatch(Box::new(move |persistent: &mut Persistent| {
				match value {
					None => {
						persistent.selected_values.remove(&data_path);
					}
					Some(value) => {
						persistent.selected_values.set(&data_path, value);
					}
				}
				MutatorImpact::Recompile // TODO: Only do this when returning to sheet view
			}));
		}
	});

	let classes = classes!("my-2", "selector");
	let missing_value = value.is_none().then(|| classes!("missing-value")).unwrap_or_default();
	let inner = match kind {
		// TODO: Display a different UI if amount > 1
		selector::Kind::StringEntry { amount: _, options, blocked_options, cannot_match } => {
			// Gather the list of all blocked options (both provided directly by the kind and via those that the picker is not allowed to match).
			let mut blocked_options = blocked_options.clone();
			blocked_options
				.extend(cannot_match.iter().filter_map(|data_path| state.get_first_selection(data_path).cloned()));

			// No options means text field
			if options.is_empty() {
				let onchange = Callback::from({
					let save_value = save_value.clone();
					move |evt: web_sys::Event| {
						let Some(value) = evt.input_value() else {
							return;
						};
						save_value.emit((!value.is_empty()).then_some(value.into()));
					}
				});
				html! {<input
					class="form-control" type="text"
					value={value.cloned().unwrap_or_default()}
					{onchange}
				/>}
			}
			// If we have explicit options, then this is a select / dropdown picker
			else {
				let onchange = Callback::from({
					let save_value = save_value.clone();
					move |evt: web_sys::Event| {
						let Some(value) = evt.select_value() else {
							return;
						};
						save_value.emit((!value.is_empty()).then_some(value.into()));
					}
				});
				html! {
					<select class="form-select" {onchange}>
						<option
							value=""
							selected={value.is_none()}
						>{"Select a value..."}</option>
						{options.iter().map(|item| {
							html! {
								<option
									value={item.clone()}
									selected={value == Some(item)}
									disabled={blocked_options.contains(item)}
								>
									{item.clone()}
								</option>
							}
						}).collect::<Vec<_>>()}
					</select>
				}
			}
		}
		selector::Kind::Object { amount, object_category, criteria } => {
			let browse = Callback::from({
				let context_menu = context_menu.clone();
				let props = object_browser::ModalProps {
					data_path: data_path.clone(),
					category: object_category.clone().into(),
					capacity: *amount,
					criteria: criteria.clone(),
				};
				let title = format!("Browse {object_category}");
				move |_| {
					let props = props.clone();
					let item = context_menu::Item::new(title.clone(), html!(<object_browser::Modal ..props />));
					context_menu.dispatch(match active_details {
						None => context_menu::Action::OpenRoot(item),
						Some(_) => context_menu::Action::OpenSubpage(item),
					});
				}
			});
			let btn_classes = classes!("btn", "btn-outline-theme", "btn-xs", missing_value);
			let selection_count = state.get_selections_at(data_path).map(|list| list.len()).unwrap_or_default();
			return html! {
				<div class={classes}>
					<h6>{name.clone()}</h6>
					<button type="button" class={btn_classes} onclick={browse}>
						{format!("Browse ({}/{amount} selected)", selection_count)}
					</button>
					<div>
						<ObjectSelectorList value={data_path.clone()} />
					</div>
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
