use crate::{
	components::database::{use_query, QueryStatus},
	database::{entry::EntryInSystemWithType, Criteria, Entry, Query},
	page::characters::sheet::{joined::editor::bundle_content, CharacterHandle},
	system::{
		self,
		dnd5e::{
			components::GeneralProp,
			data::{
				character::{spellcasting::AbilityOrStat, ObjectCacheArc},
				Bundle,
			},
			DnD5e,
		},
		System,
	},
};
use futures_util::FutureExt;
use kdlize::NodeId;
use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
	rc::Rc,
	str::FromStr,
	sync::Arc,
};
use yew::prelude::*;

#[function_component]
pub fn Provider(props: &html::ChildrenProps) -> Html {
	let registry = use_state(|| Registry::new());
	html! {
		<ContextProvider<Registry> context={(*registry).clone()}>
			{props.children.clone()}
		</ContextProvider<Registry>>
	}
}

#[derive(Clone)]
pub struct Registry(Rc<HashMap<String, Box<dyn ObjectBrowser>>>);
impl Registry {
	fn new() -> Self {
		let mut registry = HashMap::<_, Box<dyn ObjectBrowser>>::new();
		registry.insert(SpellBrowser::id().to_owned(), Box::new(SpellBrowser));
		registry.insert(BundleBrowser::id().to_owned(), Box::new(BundleBrowser));
		registry.insert(SubclassBrowser::id().to_owned(), Box::new(SubclassBrowser));
		Self(Rc::new(registry))
	}

	fn get(&self, key: impl AsRef<str>) -> Option<&Box<dyn ObjectBrowser>> {
		self.0.get(key.as_ref())
	}
}
impl PartialEq for Registry {
	fn eq(&self, other: &Self) -> bool {
		Rc::ptr_eq(&self.0, &other.0)
	}
}

trait ObjectBrowser {
	fn id() -> &'static str
	where
		Self: Sized;

	fn modal(&self, props: &ModalProps) -> Html;
}

#[derive(Clone, PartialEq, Properties)]
pub struct ModalProps {
	pub data_path: PathBuf,
	pub category: AttrValue,
	pub capacity: usize,
	pub criteria: Option<Criteria>,
}
#[function_component]
pub fn Modal(props: &ModalProps) -> Html {
	let registry = use_context::<Registry>().unwrap();
	if let Some(browser) = registry.get(props.category.as_str()) {
		return browser.modal(props);
	}
	html! {<>
		<h1>{"Unsupported object category"}</h1>
		{format!(
			"There is no browser registry item for the object type \"{}\". \
			You may need to add a new browser object entry to the registry. \
			The currently supported types are: {:?}",
			props.category.as_str(), registry.0.as_ref().keys().collect::<Vec<_>>()
		)}
	</>}
}

struct SpellBrowser;
impl ObjectBrowser for SpellBrowser {
	fn id() -> &'static str {
		crate::system::dnd5e::data::Spell::id()
	}

	fn modal(&self, props: &ModalProps) -> Html {
		use crate::system::dnd5e::{
			components::panel::{AvailableSpellList, HeaderAddon},
			data::{character::spellcasting, Spell},
		};

		let header_addon = HeaderAddon::from({
			let data_path = props.data_path.clone();
			let capacity = props.capacity;
			move |spell: &Spell| -> Html {
				html! {
					<ObjectSelectorEntryButton
						data_path={data_path.clone()}
						id={spell.id.unversioned()}
						{capacity}
					/>
				}
			}
		});

		// TODO: Somehow generate the spell entry for the feature's selector data
		let spell_entry = spellcasting::SpellEntry {
			source: std::path::PathBuf::new(),
			classified_as: None,
			method: spellcasting::CastingMethod::Cast { can_use_slots: true, can_use_ritual: true },
			attack_bonus: AbilityOrStat::Ability(crate::system::dnd5e::data::Ability::Charisma),
			save_dc: AbilityOrStat::Ability(crate::system::dnd5e::data::Ability::Charisma),
			ability: Some(crate::system::dnd5e::data::Ability::Charisma),
			casting_duration: None,
			rank: None,
			range: None,
		};

		html! {<>
			<div class="browse objects spell-list">
				<AvailableSpellList
					{header_addon}
					criteria={props.criteria.clone()}
					entry={spell_entry}
				/>
			</div>
		</>}
	}
}

struct BundleBrowser;
impl ObjectBrowser for BundleBrowser {
	fn id() -> &'static str {
		crate::system::dnd5e::data::Bundle::id()
	}

	fn modal(&self, props: &ModalProps) -> Html {
		html! {<>
			<div class="browse objects list">
				<BundleList
					data_path={props.data_path.clone()}
					capacity={props.capacity}
					criteria={props.criteria.clone()}
				/>
			</div>
		</>}
	}
}

#[derive(Clone, PartialEq, Properties)]
struct BundleListProps {
	data_path: PathBuf,
	capacity: usize,
	criteria: Option<Criteria>,
}
#[function_component]
fn BundleList(props: &BundleListProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let system_depot = use_context::<system::Registry>().unwrap();
	let object_cache = use_context::<ObjectCacheArc>().unwrap();
	let fetch_bundles = use_query(Some(props.criteria.clone()), move |database, criteria| {
		let system_depot = system_depot.clone();
		let object_cache = object_cache.clone();
		async move {
			let index = EntryInSystemWithType::new::<Bundle>(DnD5e::id());
			let query = Query::subset(&database, Some(index)).await?;
			let query = query.apply_opt(criteria, Query::filter_by);
			let query = query.parse_as_cached::<Bundle>(&system_depot, &object_cache);
			let query = query.map(|(_entry, bundle)| bundle);
			let mut bundles = query.collect::<Vec<_>>().await;
			bundles.sort_by(|a, b| a.name.cmp(&b.name));
			Ok(bundles) as Result<Vec<Bundle>, database::Error>
		}
		.boxed_local()
	});
	match fetch_bundles.status() {
		QueryStatus::Pending => html!(<crate::components::Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No bundles available"),
		QueryStatus::Success(bundles) => {
			let mut htmls = Vec::new();
			for bundle in bundles {
				let collapse_id = format!("{}", bundle.id.ref_id());
				let meets_requirements = {
					let mut iter = bundle.requirements.iter();
					iter.all(|requirement| requirement.evaluate(&*state).is_ok())
				};
				htmls.push(html! {
					<div class="section mb-1">
						<div class="header mb-1">
							<button
								role="button" class={"collapse_trigger arrow_left collapsed"}
								data-bs-toggle="collapse"
								data-bs-target={format!("#{collapse_id}")}
							>
								{bundle.name.clone()}
							</button>
							<ObjectSelectorEntryButton
								data_path={props.data_path.clone()}
								id={bundle.id.unversioned()}
								capacity={props.capacity}
								disabled={!meets_requirements}
							/>
						</div>
						<div class="collapse mb-2" id={collapse_id}>
							<div class="card">
								<div class="card-body px-2 py-1">
									{bundle_content(bundle, None, false)}
								</div>
							</div>
						</div>
					</div>
				});
			}
			html!(<>{htmls}</>)
		}
	}
}

struct SubclassBrowser;
impl ObjectBrowser for SubclassBrowser {
	fn id() -> &'static str {
		crate::system::dnd5e::data::Subclass::id()
	}

	fn modal(&self, props: &ModalProps) -> Html {
		html! {<>
			<div class="modal-header">
				<h1 class="modal-title fs-4">{"Browse Subclasses"}</h1>
				<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
			</div>
			<div class="modal-body list">
				<SubclassList
					data_path={props.data_path.clone()}
					capacity={props.capacity}
					criteria={props.criteria.clone()}
				/>
			</div>
		</>}
	}
}

#[derive(Clone, PartialEq, Properties)]
struct SubclassListProps {
	data_path: PathBuf,
	capacity: usize,
	criteria: Option<Criteria>,
}
#[function_component]
fn SubclassList(props: &SubclassListProps) -> Html {
	use crate::{
		components::database::{use_query_all_typed, QueryAllArgs, QueryStatus},
		system::{
			dnd5e::{data::Subclass, DnD5e},
			System,
		},
	};

	let fetch_objects = use_query_all_typed::<Subclass>(
		true,
		Some(QueryAllArgs::<Subclass> {
			system: DnD5e::id().into(),
			criteria: props.criteria.clone().map(Box::new),
			adjust_listings: Some(Arc::new(|mut objects| {
				objects.sort_by(|a, b| a.name.cmp(&b.name));
				objects
			})),
			max_limit: None,
		}),
	);
	match fetch_objects.status() {
		QueryStatus::Pending => html!(<crate::components::Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No subclasses available"),
		QueryStatus::Success(objects) => {
			let mut htmls = Vec::new();
			for item in objects {
				let collapse_id = format!("{}", item.id.ref_id());
				htmls.push(html! {
					<div class="section mb-1">
						<div class="header mb-1">
							<button
								role="button" class={"collapse_trigger arrow_left collapsed"}
								data-bs-toggle="collapse"
								data-bs-target={format!("#{collapse_id}")}
							>
								{item.name.clone()}
							</button>
							<ObjectSelectorEntryButton
								data_path={props.data_path.clone()}
								id={item.id.unversioned()}
								capacity={props.capacity}
							/>
						</div>
						<div class="collapse mb-2" id={collapse_id}>
							<div class="card">
								<div class="card-body px-2 py-1">
									{"TODO: Subclass contents"}
								</div>
							</div>
						</div>
					</div>
				});
			}
			html!(<>{htmls}</>)
		}
	}
}

#[function_component]
pub fn ObjectSelectorList(props: &GeneralProp<std::path::PathBuf>) -> Html {
	use crate::{
		components::database::{use_query, QueryStatus},
		page::characters::sheet::CharacterHandle,
		system::SourceId,
	};

	let state = use_context::<CharacterHandle>().unwrap();
	let fetched_entries = use_query(None, |database, args: Vec<SourceId>| {
		async move {
			if args.is_empty() {
				return Ok((args, BTreeMap::<SourceId, Entry>::new()));
			}
			let query = Query::<Entry>::multiple(&database, &args).await?;
			let entries = query.collect::<Vec<_>>().await;
			let iter = entries.into_iter().map(|entry| (entry.source_id(false), entry));
			let entries = iter.collect::<BTreeMap<_, _>>();
			Ok((args, entries)) as Result<_, database::Error>
		}
		.boxed_local()
	});
	use_effect_with(state.clone(), {
		let data_path = props.value.clone();
		let fetched_entries = fetched_entries.clone();
		move |state: &CharacterHandle| {
			let Some(values) = state.get_selections_at(&data_path) else {
				fetched_entries.update((Vec::new(), BTreeMap::new()));
				return;
			};
			let mut ids = Vec::with_capacity(values.len());
			for value in values {
				let Ok(id) = SourceId::from_str(value.as_str()) else {
					continue;
				};
				ids.push(id.into_unversioned());
			}
			fetched_entries.run(ids);
		}
	});

	match fetched_entries.status() {
		QueryStatus::Pending => html!(<crate::components::Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No selections"),
		QueryStatus::Success((ids, items)) => {
			html! {
				<ul class="mb-0">
					{ids.iter().filter_map(|id| items.get(id)).map(|entry| html! {
						<li>{entry.name().unwrap_or("Unknown")}</li>
					}).collect::<Vec<_>>()}
				</ul>
			}
		}
	}
}

#[derive(Clone, PartialEq, Properties)]
struct ObjectSelectorEntryButtonProps {
	data_path: std::path::PathBuf,
	id: crate::system::SourceId,
	capacity: usize,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn ObjectSelectorEntryButton(props: &ObjectSelectorEntryButtonProps) -> Html {
	let state = use_context::<crate::page::characters::sheet::CharacterHandle>().unwrap();

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
		move |evt: MouseEvent, persistent| {
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
			crate::page::characters::sheet::MutatorImpact::Recompile
		}
	});

	let mut classes = classes!("btn", "btn-xs", "select");
	let disabled = !is_selected && !can_select_more || props.disabled;
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
			{disabled} {onclick}
		>
			{match is_selected {
				true => "Selected",
				false => "Select",
			}}
		</button>
	}
}
