use super::ItemLocation;
use crate::{
	components::{
		database::{use_query_all_typed, use_typed_fetch_callback_tuple, QueryAllArgs, QueryStatus},
		Spinner,
	},
	database::Criteria,
	page::characters::sheet::{joined::editor::CollapsableCard, CharacterHandle},
	system::{
		dnd5e::{
			change,
			components::{
				panel::{get_inventory_item_hierarchy, AddItemButton, AddItemOperation, ItemInfo},
				validate_uint_only, GeneralProp, WalletInline,
			},
			data::{
				currency::Wallet,
				item::{self, container::item::ItemPath, Item},
			},
			DnD5e,
		},
		SourceId, System,
	},
	utility::InputExt,
};
use yew::prelude::*;

#[function_component]
pub fn BrowseModal() -> Html {
	static DEFAULT_RESULT_LIMIT: usize = 10;

	let search_params = use_state(|| SearchParams::default());
	let result_limit = use_state_eq(|| Some(DEFAULT_RESULT_LIMIT));
	let query_handle = use_query_all_typed::<Item>(false, None);

	use_effect_with((search_params.clone(), result_limit.clone()), {
		let query_handle = query_handle.clone();
		move |(params, limit): &(UseStateHandle<SearchParams>, UseStateHandle<Option<usize>>)| {
			if params.is_empty() {
				return;
			}
			let args = QueryAllArgs::<Item> {
				system: DnD5e::id().into(),
				criteria: Some(params.as_criteria()),
				max_limit: **limit,
				..Default::default()
			};
			query_handle.run(args);
		}
	});

	let on_search_changed = Callback::from({
		let result_limit = result_limit.clone();
		let search_params = search_params.clone();
		move |value: SearchParams| {
			result_limit.set(Some(DEFAULT_RESULT_LIMIT));
			search_params.set(value);
		}
	});
	let on_load_all_results = Callback::from({
		let result_limit = result_limit.clone();
		move |_| {
			result_limit.set(None);
		}
	});

	let found_item_listings = match query_handle.status() {
		QueryStatus::Pending => html! {
			<div class="d-flex justify-content-center">
				<Spinner />
			</div>
		},
		QueryStatus::Empty | QueryStatus::Failed(_) => html! {
			<div class="text-center">
				{"No items found"}
			</div>
		},
		QueryStatus::Success(items) => html! {<>
			{items.iter().enumerate().map(|(idx, _item)| {
				let location = ItemLocation::Database {
					query: query_handle.clone(),
					index: idx,
				};
				html!(<BrowsedItemCard value={location} />)
			}).collect::<Vec<_>>()}
			{result_limit.as_ref().map(|_limit| html! {
				<button
					type="button" class="btn btn-primary"
					onclick={on_load_all_results}
				>
					{"Load All Results"}
				</button>
			}).unwrap_or_default()}
		</>},
	};

	html! {<div class="w-100 h-100 scroll-container-y">
		<SearchInput on_change={on_search_changed} />
		<div style="height: 600px">
			{found_item_listings}
		</div>
	</div>}
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct SearchParams {
	text: String,
}
impl SearchParams {
	fn is_empty(&self) -> bool {
		self.text.is_empty()
	}

	fn as_criteria(&self) -> Box<Criteria> {
		let contains_text = Criteria::ContainsSubstring(self.text.clone());
		let name_contains = Criteria::ContainsProperty("name".into(), contains_text.into());
		name_contains.into()
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct SearchInputProps {
	pub on_change: Callback<SearchParams>,
}
#[function_component]
pub fn SearchInput(SearchInputProps { on_change }: &SearchInputProps) -> Html {
	let params = use_state(|| SearchParams::default());

	let set_params_text = Callback::from({
		let params = params.clone();
		let on_change = on_change.clone();
		move |text: String| {
			if params.text == text {
				return;
			}
			let mut new_params = (*params).clone();
			new_params.text = text;
			params.set(new_params.clone());
			on_change.emit(new_params);
		}
	});

	let oninput = Callback::from({
		let set_params_text = set_params_text.clone();
		move |evt: InputEvent| {
			let Some(value) = evt.input_value() else {
				return;
			};
			set_params_text.emit(value);
		}
	});

	html! {
		<div class="input-group mb-2">
			<span class="input-group-text"><i class="bi bi-search"/></span>
			<input
				type="text" class="form-control"
				placeholder="Search item names, types, rarities, or tags"
				{oninput}
			/>
		</div>
	}
}

#[function_component]
fn BrowsedItemCard(props: &GeneralProp<ItemLocation>) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(item) = props.value.resolve(&state) else {
		return Html::default();
	};

	let add_item = state.dispatch_change({
		let state = state.clone();
		let item = item.clone();
		move |container: Option<ItemPath>| {
			let container = match container {
				None => None,
				Some(path) => {
					let names = {
						let iter = get_inventory_item_hierarchy(&state, &path).into_iter();
						let iter = iter.map(|item| item.name.clone());
						iter.collect()
					};
					Some((path, names))
				}
			};
			Some(change::inventory::AddItem { item: item.clone(), container })
		}
	});

	let card_id = item.id.unversioned().ref_id();
	let batch_size = match &item.kind {
		item::Kind::Simple { count } => Some(*count),
		_ => None,
	};
	html! {
		<CollapsableCard
			id={card_id}
			header_content={{
				html! {<>
					<span>{item.name.clone()}</span>
					<AddItemButton
						root_classes={"ms-auto"} btn_classes={classes!("btn-theme", "btn-xs")}
						operation={AddItemOperation::Add}
						on_click={add_item}
					/>
				</>}
			}}
		>
			<ItemInfo location={props.value.clone()} />
			<AddItemActions
				id={item.id.unversioned()}
				{batch_size}
				worth={item.worth.clone()}
			/>
		</CollapsableCard>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct AddItemActionsProps {
	id: SourceId,
	batch_size: Option<u32>,
	worth: Wallet,
}
type AddItemArgs = (usize, Wallet, Option<ItemPath>);
#[function_component]
fn AddItemActions(AddItemActionsProps { id, batch_size, worth }: &AddItemActionsProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let auto_exchange = state.persistent().settings.currency_auto_exchange;
	let amt_to_add = use_state_eq(|| 1u32);
	let amt_to_buy = use_state_eq(|| 1u32);

	let add_items = use_typed_fetch_callback_tuple::<Item, AddItemArgs>(
		"Add Items".into(),
		state.dispatch_change({
			let state = state.clone();
			move |args: (Item, AddItemArgs)| {
				let (item, (amount, cost, container)) = args;
				let container = match container {
					None => None,
					Some(path) => {
						let names = {
							let iter = get_inventory_item_hierarchy(&state, &path).into_iter();
							let iter = iter.map(|item| item.name.clone());
							iter.collect()
						};
						Some((path, names))
					}
				};
				Some(change::inventory::PurchaseItem { item, amount, cost, container })
			}
		}),
	);
	let add_items = add_items.reform({
		let id = id.clone();
		move |args| (id.clone(), args)
	});

	let section_add_amt = match batch_size {
		Some(batch_size) => Some({
			let amt_state = amt_to_add.clone();
			let set_amt = Callback::from({
				let amt_state = amt_state.clone();
				move |value: u32| {
					amt_state.set(value.clamp(1, 999));
				}
			});
			let onchange = Callback::from({
				let set_amt = set_amt.clone();
				move |evt: web_sys::Event| {
					let Some(value) = evt.input_value_t::<u32>() else {
						return;
					};
					set_amt.emit(value);
				}
			});
			let increment = Callback::from({
				let set_amt = set_amt.clone();
				let amt_state = amt_state.clone();
				move |_| {
					set_amt.emit(amt_state.saturating_add(1));
				}
			});
			let decrement = Callback::from({
				let set_amt = set_amt.clone();
				let amt_state = amt_state.clone();
				move |_| {
					set_amt.emit(amt_state.saturating_sub(1));
				}
			});
			let on_confirm = Callback::from({
				let add_items = add_items.clone();
				let amt_state = amt_state.clone();
				move |container_id| {
					add_items.emit((*amt_state as usize, Wallet::default(), container_id));
					amt_state.set(1);
				}
			});
			html! {<>
				<div class="input-group item-add-amount mt-3">
					<span class="input-group-text title">{"Amount to Add"}</span>
					<button type="button" class="btn btn-outline-theme dec" onclick={decrement}><i class="bi bi-dash" /></button>
					<input
						class="form-control text-center"
						type="number"
						min="1" value={(*amt_state).to_string()}
						onkeydown={validate_uint_only()}
						onchange={onchange}
					/>
					<button type="button" class="btn btn-outline-theme inc" onclick={increment}><i class="bi bi-plus" /></button>
					<span class="input-group-text spacer" />
					<AddItemButton
						root_classes={"submit"} btn_classes={classes!("btn-theme", "btn-xs")}
						operation={AddItemOperation::Add}
						amount={*amt_state} on_click={on_confirm}
					/>
				</div>
				<div class="form-text">
					{match (*amt_state, *batch_size) {
						(1, 1) => format!("Add 1 item to your equipment."),
						(n, 1) | (1, n) => format!("Add {n} items to your equipment."),
						(n, b) => format!("Add {n} batches of {b} items to your equipment."),
					}}
				</div>
			</>}
		}),
		None => None,
	};
	let section_purchase = match worth.is_empty() {
		false => Some({
			let batch_size = batch_size.clone().unwrap_or(1);
			let amt_state = amt_to_buy.clone();
			let set_amt = Callback::from({
				let amt_state = amt_state.clone();
				move |value: u32| {
					amt_state.set(value.clamp(1, 999));
				}
			});
			let onchange = Callback::from({
				let set_amt = set_amt.clone();
				move |evt: web_sys::Event| {
					let Some(value) = evt.input_value_t::<u32>() else {
						return;
					};
					set_amt.emit(value);
				}
			});
			let increment = Callback::from({
				let set_amt = set_amt.clone();
				let amt_state = amt_state.clone();
				move |_| {
					set_amt.emit(amt_state.saturating_add(1));
				}
			});
			let decrement = Callback::from({
				let set_amt = set_amt.clone();
				let amt_state = amt_state.clone();
				move |_| {
					set_amt.emit(amt_state.saturating_sub(1));
				}
			});
			let purchase_cost = {
				let mut cost = *worth * (*amt_state as u64);
				if auto_exchange {
					cost.normalize();
				}
				cost
			};
			let on_confirm = Callback::from({
				let add_items = add_items.clone();
				let amt_state = amt_state.clone();
				move |container_id| {
					add_items.emit((*amt_state as usize, purchase_cost, container_id));
					amt_state.set(1);
				}
			});
			let not_enough_in_wallet = !state.inventory().wallet().contains(&purchase_cost, auto_exchange);
			html! {<>
				<div class="input-group item-add-amount mt-3">
					<span class="input-group-text title">{"Purchase"}</span>
					<button type="button" class="btn btn-outline-theme dec" onclick={decrement}><i class="bi bi-dash" /></button>
					<input
						class="form-control text-center"
						type="number"
						min="1" value={(*amt_state).to_string()}
						onkeydown={validate_uint_only()}
						onchange={onchange}
					/>
					<button type="button" class="btn btn-outline-theme inc" onclick={increment}><i class="bi bi-plus" /></button>
					<span class="input-group-text spacer" />
					<AddItemButton
						root_classes={"submit"} btn_classes={classes!("btn-theme", "btn-xs")}
						amount={*amt_state} on_click={on_confirm}
						operation={AddItemOperation::Buy}
						disabled={not_enough_in_wallet}
					/>
				</div>
				<div class="form-text">
					{match (*amt_state, batch_size) {
						(1, 1) => format!("Buy 1 item for "),
						(n, 1) | (1, n) => format!("Buy {n} items for "),
						(n, b) => format!("Buy {n} batches of {b} items for "),
					}}
					<span><WalletInline wallet={purchase_cost} /></span>
				</div>
			</>}
		}),
		true => None,
	};
	html! {<>
		{section_add_amt.unwrap_or_default()}
		{section_purchase.unwrap_or_default()}
	</>}
}
