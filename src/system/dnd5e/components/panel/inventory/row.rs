use super::InventoryItemProps;
use crate::{
	components::context_menu,
	page::characters::sheet::{CharacterHandle, MutatorImpact},
	system::dnd5e::{
		components::panel::{
			get_inventory_item, inventory::equip_toggle::ItemRowEquipBox, AddItemButton, AddItemOperation,
			ItemBodyProps, ItemInfo, ItemLocation,
		},
		data::item::{self, container::item::EquipStatus, Item},
	},
};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct ItemRowProps {
	pub id_path: Vec<Uuid>,
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
						id={id_path.last().unwrap().clone()}
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

	let on_delete = state.new_dispatch({
		let id_path = id_path.clone();
		let close_modal = close_modal.clone();
		move |_: MouseEvent, persistent| {
			let equip_status = {
				let item_id = id_path.get(0);
				let equip_status = item_id.map(|id| persistent.inventory.get_equip_status(id));
				equip_status.unwrap_or_default()
			};
			let _item = persistent.inventory.remove_at_path(&id_path);
			close_modal.emit(());
			match equip_status {
				EquipStatus::Unequipped => MutatorImpact::None,
				EquipStatus::Equipped | EquipStatus::Attuned => MutatorImpact::Recompile,
			}
		}
	});
	let mut item_props =
		ItemBodyProps { location: Some(ItemLocation::Inventory { id_path: id_path.clone() }), ..Default::default() };
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
			if id_path.len() == 1 {
				item_props.equip_status = state.inventory().get_equip_status(&id_path[0]);
				item_props.set_equipped = Some(state.new_dispatch({
					let id: Uuid = id_path[0].clone();
					move |should_be_equipped, persistent| {
						persistent.inventory.set_equipped(&id, should_be_equipped);
						MutatorImpact::Recompile
					}
				}));
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
				source_container: match id_path.len() {
					1 => None,
					// TODO: This should take the first n-1 elements as the parent path,
					// right now it assumes that the current item is always in a top-level container.
					n => Some(id_path[0..(n-1)].to_vec()),
				},
			}}
			on_click={state.new_dispatch({
				let close_modal = close_modal.clone();
				let id_path = id_path.clone();
				move |dst_id: Option<Vec<Uuid>>, persistent| {
					let Some(item) = persistent.inventory.remove_at_path(&id_path) else { return MutatorImpact::None; };
					persistent.inventory.insert_to(item, &dst_id);
					close_modal.emit(());
					MutatorImpact::None
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
