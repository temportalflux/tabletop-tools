use super::InventoryItemProps;
use crate::{
	components::context_menu,
	page::characters::sheet::{CharacterHandle, MutatorImpact},
	system::dnd5e::{
		change::{
			self,
			inventory::{EquipItem, ItemRef},
		},
		components::panel::{
			get_inventory_item, get_item_path_names, inventory::equip_toggle::ItemRowEquipBox, AddItemButton,
			AddItemOperation, ItemBodyProps, ItemInfo, ItemLocation,
		},
		data::item::{
			self,
			container::item::{EquipStatus, ItemPath},
			Item,
		},
	},
};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct ItemRowProps {
	pub id_path: ItemPath,
	pub item: Item,
	#[prop_or_default]
	pub is_equipped: Option<bool>,
}

#[function_component]
pub fn ItemRow(ItemRowProps { id_path, item, is_equipped }: &ItemRowProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let open_modal = context_menu::use_control_action({
		let id_path = id_path.clone();
		let name = AttrValue::from(item.name.clone());
		move |_, _context| context_menu::Action::open_root(name.clone(), html!(<ItemModal id_path={id_path.clone()} />))
	});

	html! {
		<tr class="align-middle" onclick={open_modal}>
			{is_equipped.as_ref().map(|is_equipped| html! {
				<td class="text-center">
					<ItemRowEquipBox
						id={id_path.last().unwrap()}
						name={item.name.clone()}
						is_equipable={item.is_equipable()}
						can_be_equipped={item.can_be_equipped(&*state)}
						is_equipped={*is_equipped}
					/>
				</td>
			}).unwrap_or_default()}
			<td>{item.name.clone()}</td>
			<td class="text-center">{item.weight * item.quantity() as f32}{" lb."}</td>
			<td class="text-center">{item.quantity()}</td>
			<td style="width: 250px;">{item.notes.clone()}</td>
		</tr>
	}
}

#[function_component]
pub fn ItemModal(InventoryItemProps { id_path }: &InventoryItemProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let close_modal = context_menu::use_close_fn();
	let item = get_inventory_item(&state, id_path);
	let Some(item) = item else {
		return Html::default();
	};

	let on_delete = state.dispatch_change({
		let item_ref = ItemRef { path: id_path.clone(), name: get_item_path_names(&state, &id_path) };
		let close_modal = close_modal.clone();
		move |_| {
			close_modal.emit(());
			Some(change::inventory::RemoveItem(item_ref.clone()))
		}
	});
	let mut item_props = ItemBodyProps {
		location: ItemLocation::Inventory { id_path: id_path.clone() },
		on_quantity_changed: None,
		equip_status: EquipStatus::default(),
		set_equipped: None,
	};
	match &item.kind {
		item::Kind::Simple { .. } => {
			item_props.on_quantity_changed = Some(state.new_dispatch({
				let id_path = id_path.clone();
				move |amt, persistent| {
					if let Some(item) = persistent.inventory.get_mut_at_path(&id_path) {
						if let item::Kind::Simple { count } = &mut item.kind {
							*count = amt;
						}
					}
					MutatorImpact::None
				}
			}));
		}
		item::Kind::Equipment(_equipment) => {
			if let Some(id) = id_path.as_single() {
				item_props.equip_status = state.inventory().get_equip_status(&id);
				let name = item.name.clone();
				item_props.set_equipped =
					Some(state.dispatch_change(move |status| Some(EquipItem { id, name: name.clone(), status })));
			}
		}
	}

	// TODO: In order to move only part of a stack, we should have a form field to allow the user to split the itemstack
	// (taking stack - newsize and inserting that as a new item), so the user can move this stack to a new container.
	let move_button = html! {
		<AddItemButton
			btn_classes={classes!("btn-outline-theme", "btn-sm", "mx-1")}
			operation={AddItemOperation::Move {
				item_id: id_path.clone(),
				source_container: id_path.container(),
			}}
			on_click={Callback::from({
				let mutate = state.dispatch_change({
					let state = state.clone();
					let item_ref = ItemRef{ path: id_path.clone(), name: get_item_path_names(&state, &id_path) };
					move |dest_path: Option<ItemPath>| {
						let destination_container = match dest_path {
							None => None,
							Some(path) => {
								let container_names = get_item_path_names(&state, &path);
								Some(ItemRef{ path, name: container_names })
							}
						};
						Some(change::inventory::MoveItem {
							item: item_ref.clone(),
							destination_container,
						})
					}
				});
				let close_modal = close_modal.clone();
				move |dst_id: Option<ItemPath>| {
					mutate.emit(dst_id);
					close_modal.emit(());
				}
			})}
		/>
	};

	html! {<div class="w-100 h-100 scroll-container-y">
		<div class="d-flex flex-column" style="min-height: 200px;">
			<ItemInfo ..item_props />
			<span class="hr my-2" />
			<div class="d-flex justify-content-center mt-auto">
				{move_button}
				<button type="button" class="btn btn-sm btn-outline-theme mx-1" onclick={on_delete}>
					<i class="bi bi-trash me-1" />
					{"Delete"}
				</button>
			</div>
		</div>
	</div>}
}
