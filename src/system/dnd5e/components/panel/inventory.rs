use crate::{
	components::{
		context_menu,
		database::{use_query_all_typed, QueryAllArgs, QueryStatus},
		stop_propagation, IndirectFetch, ObjectLink,
	},
	database::Database,
	page::characters::sheet::{CharacterHandle, MutatorImpact},
	system::{
		self,
		dnd5e::{
			change::inventory::AppendItems, components::{WalletInline, WalletInlineButton}, data::{
				character::{IndirectItem, Persistent, StartingEquipment},
				currency::Wallet,
				item::{
					self,
					container::item::{AsItem, EquipStatus, ItemPath},
					Item,
				},
				Indirect,
			}
		},
		SourceId, System,
	},
	utility::InputExt,
};
use itertools::{EitherOrBoth, Itertools};
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	rc::Rc,
	str::FromStr,
};
use uuid::Uuid;
use yew::prelude::*;

mod add_btn;
pub use add_btn::*;
mod browse_content;
pub use browse_content::*;
mod equip_toggle;
pub use equip_toggle::*;
mod item_content;
pub use item_content::*;
mod row;
pub use row::*;

#[derive(Clone, PartialEq, Properties)]
pub struct SystemItemProps {
	pub id: SourceId,
}

#[derive(Clone, PartialEq, Properties)]
pub struct InventoryItemProps {
	pub id_path: ItemPath,
}

#[derive(Default, PartialEq, Clone)]
struct SearchFilter {
	text: Option<String>,
}

#[function_component]
pub fn Inventory() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let open_browser = context_menu::use_control_action({
		|_, _context| context_menu::Action::open_root("Item Browser", html!(<BrowseModal />))
	});
	let open_starting_equipment = context_menu::use_control_action({
		|_, _context| context_menu::Action::open_root("Starting Equipment", html!(<BrowseStartingEquipment />))
	});

	let search_filter = use_state_eq(|| SearchFilter::default());
	let on_search_input = Callback::from({
		let search_filter = search_filter.clone();
		move |evt: web_sys::InputEvent| {
			let Some(value) = evt.input_value() else {
				return;
			};
			let mut filter = (*search_filter).clone();
			filter.text = (!value.is_empty()).then_some(value.to_lowercase());
			search_filter.set(filter);
		}
	});

	let search_header = html! {
		<div class="input-group mt-2">
			<span class="input-group-text"><i class="bi bi-search"/></span>
			<input
				type="text" class="form-control"
				placeholder="Search item names, types, rarities, or tags"
				value={(*search_filter).text.clone()}
				oninput={on_search_input}
			/>
			<button type="button" class="btn btn-outline-theme" onclick={open_browser}>{"Browse Items"}</button>
		</div>
	};

	if state.inventory().is_empty() {
		return html! {
			<div class="panel inventory">
				{search_header}
				<div class="empty-prompt">
					<div class="text">{"Your inventory is empty. Do you want to select starting equipment?"}</div>
					<button type="button" class="btn btn-sm btn-theme mt-2" onclick={open_starting_equipment}>{"Add Starting Equipment"}</button>
				</div>
			</div>
		};
	}

	// TODO: Implement search-inventory functionality
	// TODO: tag buttons to browse item containers
	let containers = state
		.inventory()
		.iter_by_name()
		.filter(|(_, entry)| entry.as_item().items.is_some())
		.map(|(id, _)| html! { <ContainerSection container_id={id.clone()} /> })
		.collect::<Vec<_>>();

	// Gather the list of attuned items, placed into each slot alphabetically.
	let attuned_entries = {
		let slot_range = 0..state.persistent().attunement_slots;
		let iter = state.inventory().iter_by_name();
		let iter = iter.filter_map(|(id, entry)| match entry.status {
			EquipStatus::Attuned => Some((ItemPath::from(id), Rc::new(entry.item.clone()))),
			_ => None,
		});
		let iter = slot_range.zip_longest(iter);
		let iter = iter.filter_map(|zip| match zip {
			EitherOrBoth::Both(_slot, (item_id, rc_item)) => Some(Some((item_id, rc_item))),
			EitherOrBoth::Left(_slot) => Some(None),
			EitherOrBoth::Right(_) => None,
		});
		iter.collect::<Vec<_>>()
	};

	html! {
		<div class="panel inventory">
			<div class="row m-0">
				{attuned_entries.into_iter().map(|entry| match entry {
					None => html!(<AttunementSlot />),
					Some((id_path, item)) => html!(<AttunementSlot
						entry={AttunementSlotEntry {
							id_path,
							item,
						}}
					/>),
				}).collect::<Vec<_>>()}
			</div>
			{search_header}
			<div class="sections">
				<ContextProvider<UseStateHandle<SearchFilter>> context={search_filter}>
					<ContainerSection container_id={None} />
					{containers}
				</ContextProvider<UseStateHandle<SearchFilter>>>
			</div>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct AttunementSlotProps {
	#[prop_or_default]
	entry: Option<AttunementSlotEntry>,
}

#[derive(Clone, PartialEq)]
struct AttunementSlotEntry {
	id_path: ItemPath,
	item: Rc<Item>,
}

#[function_component]
fn AttunementSlot(AttunementSlotProps { entry }: &AttunementSlotProps) -> Html {
	let ctxmenu_control = use_context::<context_menu::Control>().unwrap();

	let Some(AttunementSlotEntry { id_path, item }) = entry else {
		return html!(<div class="card col mx-1">
			<div class="card-body p-1">
				<span class="d-block w-100 text-center text-body-secondary">
					{"Empty Slot"}
				</span>
			</div>
		</div>);
	};

	let open_modal = Callback::from({
		let id_path = id_path.clone();
		let name = AttrValue::from(item.name.clone());
		move |_| {
			let action = context_menu::Action::open_root(name.clone(), html!(<ItemModal id_path={id_path.clone()} />));
			ctxmenu_control.dispatch(action);
		}
	});

	html!(<div class="card col mx-1" onclick={open_modal}>
		<div class="card-body p-1">
			<span class="d-block w-100 text-center">
				{item.name.clone()}
			</span>
		</div>
	</div>)
}

#[derive(Clone, PartialEq, Properties)]
struct ContainerSectionProps {
	container_id: Option<Uuid>,
}
#[function_component]
fn ContainerSection(ContainerSectionProps { container_id }: &ContainerSectionProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let search_filter = use_context::<UseStateHandle<SearchFilter>>().unwrap();
	let context_menu = use_context::<context_menu::Control>().unwrap();

	let title: AttrValue;
	let wallet: Option<Html>;
	let rows: Vec<Html>;
	let can_equip_from = container_id.is_none();
	let open_modal: Option<Callback<MouseEvent>>;
	match container_id {
		None => {
			let container = state.inventory();
			title = "Equipment".into();
			wallet = Some(html! { <WalletInlineButton /> });
			rows = {
				let all_items = container.iter_by_name();
				let no_containers = all_items.filter(|(_, entry)| entry.as_item().items.is_none());
				let unattuned = no_containers.filter(|(_, entry)| entry.status != EquipStatus::Attuned);
				let items = unattuned.filter(|(_id, entry)| {
					if let Some(search_text) = &search_filter.text {
						return entry.item.name.to_lowercase().contains(search_text);
					}
					true
				});
				let as_html = items.map(|(id, entry)| {
					html! {
						<ItemRow
							id_path={ItemPath::from(id)}
							item={entry.item.clone()}
							is_equipped={entry.status == EquipStatus::Equipped}
						/>
					}
				});
				as_html.collect::<Vec<_>>()
			};
			open_modal = None;
		}
		Some(container_id) => {
			let Some(item) = state.inventory().get_item(container_id) else { return Html::default() };
			let Some(container) = &item.items else { return Html::default() };
			let item_path = ItemPath::from(container_id);
			title = item.name.clone().into();
			wallet = (!container.wallet().is_empty()).then(|| html! { <WalletInlineButton {item_path} /> });
			rows = {
				let container_path = ItemPath::from(container_id);
				let items = container.iter_by_name();
				let items = items.filter(|(_id, item)| {
					if let Some(search_text) = &search_filter.text {
						return item.name.to_lowercase().contains(search_text);
					}
					true
				});
				let items = items.map(|(item_id, item)| {
					html! {
						<ItemRow id_path={container_path.join(item_id)} item={item.clone()} />
					}
				});
				items.collect::<Vec<_>>()
			};
			open_modal = Some(Callback::from({
				let context_menu = context_menu.clone();
				let id_path = ItemPath::from(container_id);
				let name = AttrValue::from(item.name.clone());
				move |_| {
					context_menu.dispatch(context_menu::Action::open_root(
						name.clone(),
						html!(<ItemModal id_path={id_path.clone()} />),
					))
				}
			}));
		}
	}
	html! {
		<div class="item-container mb-3">
			<div class="d-flex" onclick={open_modal}>
				<h5 class="ms-2 my-auto">{title}</h5>
				{wallet.unwrap_or_default()}
			</div>
			<span class="hr" />
			<table class="table table-compact m-0">
				<thead>
					<tr class="text-center" style="font-size: 0.7rem;">
						{can_equip_from.then(|| html! {
							<th scope="col">{"Equip"}</th>
						}).unwrap_or_default()}
						<th scope="col">{"Name"}</th>
						<th scope="col">{"Weight"}</th>
						<th scope="col">{"Qty"}</th>
						<th scope="col">{"Notes"}</th>
					</tr>
				</thead>
				<tbody>
					{rows}
				</tbody>
			</table>
		</div>
	}
}

#[derive(Clone, Default, Debug)]
struct SelectedEquipment {
	wallet: Wallet,
	item_ids: Vec<SourceId>,
	items: Vec<Item>,
	missing_selections: Vec<PathBuf>,
}
impl SelectedEquipment {
	fn find_selections(element: &StartingEquipment, path: &Path, persistent: &Persistent) -> Self {
		match element {
			StartingEquipment::Currency(wallet) => Self { wallet: *wallet, ..Default::default() },
			StartingEquipment::IndirectItem(IndirectItem::Specific(item_id, quantity)) => {
				Self { item_ids: vec![item_id.clone(); *quantity], ..Default::default() }
			}
			StartingEquipment::IndirectItem(IndirectItem::Custom(item)) => {
				Self { items: vec![item.clone()], ..Default::default() }
			}
			StartingEquipment::SelectItem(_filter) => {
				let Some(id_str) = persistent.get_first_selection(&path) else {
					return Self { missing_selections: vec![path.to_owned()], ..Default::default() };
				};
				let Ok(id) = SourceId::from_str(id_str) else {
					return Self::default();
				};
				Self { item_ids: vec![id], ..Default::default() }
			}
			StartingEquipment::Group { entries, .. } => {
				let idx_strs = match persistent.get_selections_at(&path) {
					Some(idx_strs) if !idx_strs.is_empty() => idx_strs,
					_ => {
						return Self { missing_selections: vec![path.to_owned()], ..Default::default() };
					}
				};
				let mut selected = Self::default();
				for idx_str in idx_strs {
					let Ok(idx) = idx_str.parse::<usize>() else {
						continue;
					};
					let Some(entry) = entries.get(idx) else {
						continue;
					};
					let selection_path = path.join(idx.to_string());
					selected.extend(Self::find_selections(entry, &selection_path, persistent));
				}
				selected
			}
		}
	}

	fn extend(&mut self, other: Self) {
		self.wallet += other.wallet;
		self.item_ids.extend(other.item_ids);
		self.items.extend(other.items);
		self.missing_selections.extend(other.missing_selections);
	}

	fn is_missing_selections(&self) -> bool {
		!self.missing_selections.is_empty()
	}
}

#[function_component]
fn BrowseStartingEquipment() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let database = use_context::<Database>().unwrap();
	let system_depot = use_context::<system::Registry>().unwrap();
	let task_dispatch = use_context::<crate::task::Dispatch>().unwrap();
	let close_details = context_menu::use_close_fn::<()>();

	let mut selected_equipment = SelectedEquipment::default();
	let mut sections = Vec::with_capacity(state.starting_equipment().len());
	let selection_path = Path::new("starting_equipment").to_owned();
	for (idx, (options, source)) in state.starting_equipment().iter().enumerate() {
		let selection_path = selection_path.join(idx.to_string());
		let mut subsections = Vec::with_capacity(options.len());
		for (idx, element) in options.iter().enumerate() {
			let selection_path = selection_path.join(idx.to_string());
			selected_equipment.extend(SelectedEquipment::find_selections(element, &selection_path, state.persistent()));
			subsections.push(html!(<Section kind={element.clone()} {selection_path} />));
		}
		sections.push(html! {
			<div class="section-group">
				<h4 class="title">
					{crate::data::as_feature_path_text(source)}
				</h4>
				{subsections}
			</div>
		});
	}
	let is_missing_selections = selected_equipment.is_missing_selections();

	// Consumes `selected_equipment` to add all the resulting items to
	// the character's inventory and close the details panel.
	let submit = Callback::from({
		let state = state.clone();
		let database = database.clone();
		let system_depot = system_depot.clone();
		let task_dispatch = task_dispatch.clone();
		let close_details = close_details.clone();
		move |_| {
			if selected_equipment.is_missing_selections() {
				return;
			};
			let database = database.clone();
			let system_depot = system_depot.clone();
			let state = state.clone();
			let close_details = close_details.clone();
			let mut selected_equipment = selected_equipment.clone();
			// task required so we can get items from the database
			task_dispatch.spawn("Add Starting Equipment", None, async move {
				let item_ids = selected_equipment.item_ids.drain(..).collect::<Vec<_>>();
				let mut fetched_items = HashMap::<SourceId, (Item, usize)>::new();
				// Fetch each item from the database, counting the number of duplicates.
				for item_id in item_ids {
					match fetched_items.get_mut(&item_id) {
						Some((item, count)) => {
							// simple items (which can stack) applies the count directly to the item
							// equipment (cannot stack) keeps track of the quantity in `fetched_items`
							match &mut item.kind {
								item::Kind::Simple { count } => {
									*count += 1;
								}
								_ => {
									*count += 1;
								}
							}
						}
						// fetch yet-unfetched items from database
						None => {
							let Some(item) =
								database.get_typed_entry::<Item>(item_id.clone(), system_depot.clone(), None).await?
							else {
								return Ok(());
							};
							fetched_items.insert(item_id, (item, 1));
						}
					}
				}
				// Move fetched instances into `selected_equipment`
				for (_item_id, (item, count)) in fetched_items {
					selected_equipment.items.extend(item.create_stack(count));
				}
				// finally, actually add the wallet and items to the inventory,
				// and then close the details
				state.add_change(AppendItems {
					container: None,
					items: selected_equipment.items,
					currency: selected_equipment.wallet,
				});
				close_details.emit(());
				Ok(()) as Result<(), crate::database::FetchError>
			});
		}
	});

	html!(<div class="w-100 h-100 scroll-container-y">
		<div class="starting-equipment">
			<div class="actions">
				<button type="button" onclick={submit} class={classes!(
					"btn", "btn-sm", "btn-success",
					is_missing_selections.then(|| "disabled")
				)}>
					{"Add Equipment & Close"}
				</button>
			</div>
			<div class="body">
				{sections}
			</div>
		</div>
	</div>)
}

#[derive(Clone, PartialEq, Properties)]
struct SectionProps {
	selection_path: PathBuf,
	kind: StartingEquipment,
	#[prop_or_default]
	prefix: Html,
	#[prop_or_default]
	disabled: bool,
}

#[function_component]
fn Section(SectionProps { selection_path, kind, prefix, disabled }: &SectionProps) -> Html {
	let content = match kind {
		StartingEquipment::Currency(wallet) => html! {<>
			<div class="label">
				{prefix.clone()}
				<span>{"Get Currency"}</span>
			</div>
			<div class="wallet">
				<WalletInline wallet={*wallet} />
			</div>
		</>},
		StartingEquipment::IndirectItem(IndirectItem::Specific(id, quantity)) => {
			html!(<ItemById
				id={id.minimal().into_owned()} quantity={*quantity}
				prefix={prefix.clone()} disabled={*disabled}
			/>)
		}
		StartingEquipment::IndirectItem(IndirectItem::Custom(item)) => {
			html!(<SpecificItem
				item={std::rc::Rc::new(item.clone())} quantity={1}
				prefix={prefix.clone()} disabled={*disabled}
			/>)
		}
		// TODO: Pick should always be a number, never none
		StartingEquipment::Group { entries, pick } => html!(
			<Group
				entries={entries.clone()} pick_max={pick.unwrap_or(1)}
				selection_path={selection_path.clone()}
				prefix={prefix.clone()} disabled={*disabled}
			/>
		),
		StartingEquipment::SelectItem(filter) => html!(
			<SelectItem
				filter={filter.clone()}
				selection_path={selection_path.clone()}
				prefix={prefix.clone()} disabled={*disabled}
			/>
		),
	};
	html!(<div class="section">{content}</div>)
}

#[derive(Clone, PartialEq, Properties)]
struct GroupProps {
	entries: Vec<StartingEquipment>,
	pick_max: usize,
	selection_path: PathBuf,
	#[prop_or_default]
	prefix: Html,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn Group(props: &GroupProps) -> Html {
	let GroupProps { entries, pick_max, selection_path, prefix, disabled } = props;
	let state = use_context::<CharacterHandle>().unwrap();

	fn get_selected_indices(values: Option<&Vec<String>>) -> Vec<usize> {
		values
			.map(|values| values.iter().filter_map(|s| s.parse::<usize>().ok()).collect::<Vec<_>>())
			.unwrap_or_default()
	}
	let values = get_selected_indices(state.get_selections_at(&selection_path));
	let selected_options = use_state_eq(move || values);

	let set_option_selected = Callback::from({
		let state = state.clone();
		let selection_path = selection_path.clone();
		let pick_max = *pick_max;
		let selected_options = selected_options.clone();
		move |(idx, should_be_selected): (usize, bool)| {
			let key = selection_path.clone();
			let value = idx.to_string();
			let selected_options = selected_options.clone();
			state.dispatch(move |persistent| {
				match (should_be_selected, pick_max) {
					(false, _) => {
						persistent.remove_selected_value(&key, value);
					}
					(true, 1) => {
						persistent.set_selected_value(&key, value);
					}
					(true, _) => {
						persistent.insert_selection(&key, value);
					}
				}

				let values = get_selected_indices(persistent.get_selections_at(&key));
				selected_options.set(values);

				MutatorImpact::None
			});
		}
	});

	let can_select_new = *pick_max == 1 || selected_options.len() < *pick_max;

	html! {<>
		<div class="label">
			{prefix.clone()}
			<span>{format!("Pick {pick_max} of")}</span>
		</div>
		<div class="group">
			{entries.iter().enumerate().map(move |(idx, group_option)| {
				html!(<GroupOption
					group_option={group_option.clone()}

					{can_select_new}
					has_any_selected={!selected_options.is_empty()}
					is_selected={selected_options.contains(&idx)}
					set_option_selected={set_option_selected.reform(move |toggle| (idx, toggle))}

					selection_path={selection_path.join(idx.to_string())}
					disabled={*disabled}
				/>)
			}).collect::<Vec<_>>()}
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct GroupOptionProps {
	group_option: StartingEquipment,

	can_select_new: bool,
	has_any_selected: bool,
	is_selected: bool,
	set_option_selected: Callback<bool>,

	selection_path: PathBuf,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn GroupOption(props: &GroupOptionProps) -> Html {
	let GroupOptionProps {
		group_option,
		can_select_new,
		has_any_selected,
		is_selected,
		set_option_selected,
		selection_path,
		disabled,
	} = props;

	let onchange = Callback::from({
		let set_option_selected = set_option_selected.clone();
		move |evt: web_sys::Event| {
			let Some(should_be_picked) = evt.input_checked() else {
				return;
			};
			set_option_selected.emit(should_be_picked);
		}
	});

	let checkbox = html!(<input
		type="checkbox"
		class={classes!(
			"form-check-input", "slot",
			(!disabled && !has_any_selected).then(|| "missing-value"),
		)}
		checked={*is_selected}
		disabled={*disabled || (!can_select_new && !is_selected)}
		onclick={stop_propagation()}
		{onchange}
	/>);
	html!(<Section
		kind={group_option.clone()}
		selection_path={selection_path.clone()}
		prefix={checkbox.clone()} disabled={*disabled || !is_selected}
	/>)
}

#[derive(Clone, PartialEq, Properties)]
struct ItemByIdProps {
	id: SourceId,
	quantity: usize,
	#[prop_or_default]
	prefix: Html,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn ItemById(ItemByIdProps { id, quantity, prefix, disabled }: &ItemByIdProps) -> Html {
	html!(<IndirectFetch<Item>
		indirect={Indirect::Id(id.clone())}
		to_inner={Callback::from({
			let quantity = *quantity;
			let disabled = *disabled;
			let prefix = prefix.clone();
			move |item: Rc<Item>| html! {
				<SpecificItem item={item.clone()} {quantity} prefix={prefix.clone()} {disabled} />
			}
		})}
	/>)
}

#[derive(Clone, PartialEq, Properties)]
struct SpecificItemProps {
	item: std::rc::Rc<Item>,
	quantity: usize,
	#[prop_or_default]
	prefix: Html,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn SpecificItem(SpecificItemProps { item, quantity, prefix, disabled }: &SpecificItemProps) -> Html {
	let onclick = context_menu::use_control_action({
		let item = item.clone();
		move |_, context| {
			let location = ItemLocation::Explicit { item: item.clone() };
			context_menu::Action::open(&context, item.name.clone(), html!(<ItemInfo {location} />))
		}
	});
	html! {<>
		<div class="label">
			{prefix.clone()}
			<span>
				{"Get Item"}
				{(*quantity > 1).then(|| html!(format!(" (x{quantity})"))).unwrap_or_default()}
			</span>
		</div>
		<div class="specific-item">
			<ObjectLink
				title={item.name.clone()}
				{onclick}
				disabled={*disabled}
			/>
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct SelectItemProps {
	filter: item::Restriction,
	selection_path: PathBuf,
	#[prop_or_default]
	prefix: Html,
	#[prop_or_default]
	disabled: bool,
}
#[function_component]
fn SelectItem(SelectItemProps { filter, selection_path, prefix, disabled }: &SelectItemProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let selected = state.get_first_selection(&selection_path).cloned();

	let query_handle = use_query_all_typed::<Item>(
		true,
		Some(QueryAllArgs {
			system: crate::system::dnd5e::DnD5e::id().into(),
			criteria: Some(filter.as_criteria().into()),
			..Default::default()
		}),
	);
	let onchange = Callback::from({
		let state = state.clone();
		let key = selection_path.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.select_value() else {
				return;
			};
			let next_value = (!value.is_empty()).then(move || value);
			let key = key.clone();
			state.dispatch(move |persistent| {
				persistent.set_selected(key, next_value);
				MutatorImpact::None
			});
		}
	});

	let mut selected_item = None;
	match (&selected, query_handle.status()) {
		(Some(id_str), QueryStatus::Success(items)) => {
			for item in items {
				if item.id.to_string() == *id_str {
					selected_item = Some(Rc::new(item.clone()));
					break;
				}
			}
		}
		_ => {}
	};

	let empty_option = |text: &'static str| html!(<option selected={selected.is_none()}>{text}</option>);
	let options = match query_handle.status() {
		QueryStatus::Pending => empty_option("Pending..."),
		QueryStatus::Empty => empty_option("No Options"),
		QueryStatus::Failed(err) => {
			log::error!(target: "starting-equipment", "Failed to find items for {filter:?}: {err:?}");
			empty_option("Unavailable")
		}
		QueryStatus::Success(items) => {
			let mut options = vec![empty_option("Select Item...")];
			for item in items {
				let id_str = item.id.minimal().to_string();
				let is_selected = selected.as_ref() == Some(&id_str);
				options.push(html! {
					<option selected={is_selected} value={id_str}>
						{&item.name}
					</option>
				});
			}
			html!(<>{options}</>)
		}
	};
	let mut select_class = classes!("form-select");
	if selected.is_none() && !*disabled {
		select_class.push("missing-value");
	}
	let onclick_selected = context_menu::use_control_action({
		move |rc_item: Rc<Item>, context| {
			let location = ItemLocation::Explicit { item: rc_item.clone() };
			context_menu::Action::open(&context, rc_item.name.clone(), html!(<ItemInfo {location} />))
		}
	});
	html! {<>
		<div class="label">
			{prefix.clone()}
			<span>{"Select an item"}</span>
		</div>
		<div class="select-item">
			<div class="content">
				<select
					class={select_class}
					disabled={*disabled}
					{onchange}
				>
					{options}
				</select>
				{selected_item.map(|item| html! {
					<ObjectLink
						title={item.name.clone()}
						onclick={onclick_selected.reform({
							let item = item.clone();
							move |_| item.clone()
						})}
						disabled={*disabled}
					/>
				})}
			</div>
		</div>
	</>}
}
