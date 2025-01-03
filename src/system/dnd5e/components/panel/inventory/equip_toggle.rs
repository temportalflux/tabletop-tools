use crate::{
	bootstrap::components::Tooltip,
	components::stop_propagation,
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{change::EquipItem, data::item::container::item::EquipStatus},
	utility::InputExt,
};
use uuid::Uuid;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct EquipBoxProps {
	pub id: Uuid,
	pub is_equipable: bool,
	pub can_be_equipped: Result<(), String>,
	pub is_equipped: bool,
}

#[function_component]
pub fn ItemRowEquipBox(EquipBoxProps { id, is_equipable, can_be_equipped, is_equipped }: &EquipBoxProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	if !*is_equipable {
		return html! { {"--"} };
	}

	let on_change = state.dispatch_change({
		let id = id.clone();
		move |evt: web_sys::Event| {
			let Some(should_be_equipped) = evt.input_checked() else { return None };
			let desired_status = match should_be_equipped {
				false => EquipStatus::Unequipped,
				true => EquipStatus::Equipped,
			};
			Some(EquipItem { id, status: desired_status })
		}
	});

	html! {
		<Tooltip content={match *is_equipped {
			true => None,
			false => can_be_equipped.clone().err(),
		}}>
			<input
				class={"form-check-input equip"} type={"checkbox"}
				checked={*is_equipped}
				disabled={!*is_equipped && can_be_equipped.is_err()}
				onclick={stop_propagation()}
				onchange={on_change}
			/>
		</Tooltip>
	}
}
