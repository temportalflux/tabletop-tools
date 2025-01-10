use crate::{
	page::characters::sheet::CharacterHandle,
	system::dnd5e::data::item::container::item::{AsItem, ItemPath},
};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct AddItemButtonProps {
	#[prop_or_default]
	pub root_classes: Classes,
	#[prop_or_default]
	pub btn_classes: Classes,
	#[prop_or_else(|| 1)]
	pub amount: u32,
	#[prop_or_default]
	pub disabled: bool,
	pub operation: AddItemOperation,
	pub on_click: Callback<Option<ItemPath>>,
}

#[derive(Clone, PartialEq)]
pub enum AddItemOperation {
	Add,
	Buy,
	Move { item_id: ItemPath, source_container: Option<ItemPath> },
}

#[function_component]
pub fn AddItemButton(props: &AddItemButtonProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let item_containers = {
		let iter = state.inventory().iter_by_name();
		let iter = iter.filter(|(_, entry)| entry.as_item().items.is_some());
		let iter = iter.map(|(id, entry)| (id, entry.as_item()));
		iter.collect::<Vec<_>>()
	};

	if let AddItemOperation::Move { .. } = &props.operation {
		if item_containers.is_empty() {
			return Html::default();
		}
	}

	let mut btn_classes = classes!("btn", props.btn_classes.clone());

	if !item_containers.is_empty() {
		btn_classes.push("dropdown-toggle");
	}

	let op_text = match props.operation {
		AddItemOperation::Add => "ADD",
		AddItemOperation::Buy => "BUY",
		AddItemOperation::Move { .. } => "MOVE",
	};
	let amt_text = match props.amount {
		n if n > 1 => format!(" {n}"),
		_ => String::default(),
	};
	let dst_text = match item_containers.is_empty() {
		true => "",
		false => " TO",
	};
	let btn_text = format!("{op_text}{amt_text}{dst_text}");

	if item_containers.is_empty() {
		return html! {
			<button
				type="button" class={classes!(btn_classes, props.root_classes.clone())}
				onclick={props.on_click.reform(|_| None)}
				disabled={props.disabled}
			>
				{btn_text}
			</button>
		};
	}

	let is_valid_dst = |dst_id: &Option<ItemPath>| match &props.operation {
		AddItemOperation::Move { item_id, source_container } => {
			dst_id != source_container && dst_id.as_ref() != Some(item_id)
		}
		_ => true,
	};
	let make_container_button = |id: Option<ItemPath>, name: String| -> Html {
		let mut classes = classes!("dropdown-item");
		let is_a_valid_dst = is_valid_dst(&id);
		let mut onclick = props.on_click.reform(move |_| id.clone());
		if !is_a_valid_dst {
			classes.push("disabled");
			onclick = Callback::default();
		}
		html! {
			<li>
				<a class={classes} onclick={onclick}>
					{name}
				</a>
			</li>
		}
	};
	let mut container_entries = Vec::with_capacity(item_containers.len() + 1);
	container_entries.push(make_container_button(None, "Equipment".into()));
	// TODO: Display containers that are inside other containers (not just top level)
	container_entries.extend(
		item_containers
			.into_iter()
			.map(|(id, item)| make_container_button(Some(ItemPath::from(id)), item.name.clone())),
	);

	html! {
		<div class={classes!("btn-group", props.root_classes.clone())} role="group">
			<button
				type="button" class={btn_classes}
				disabled={props.disabled}
				data-bs-toggle="dropdown"
			>
				{btn_text}
			</button>
			<ul class="dropdown-menu">
				{container_entries}
			</ul>
		</div>
	}
}
