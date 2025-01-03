use crate::{
	bootstrap::components::Tooltip,
	components::context_menu,
	page::characters::sheet::CharacterHandle,
	system::dnd5e::data::{AttributedValueMap, WeaponProficiency},
};
use yew::prelude::*;

#[function_component]
pub fn Proficiencies() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let proficiencies = state.other_proficiencies();
	let onclick = context_menu::use_control_action({
		let state = state.clone();
		move |_, _context| {
			let proficiencies = state.other_proficiencies();
			context_menu::Action::open_root(
				"General Proficiencies",
				html! {<div class="w-100 h-100 scroll-container-y">
					{make_proficiencies_section_long("Languages", &proficiencies.languages, String::to_string)}
					{make_proficiencies_section_long("Armor", &proficiencies.armor, |(value, context)| {
						format!("{}{}", value.to_string(), context.as_ref().map(|s| format!(" ({s})")).unwrap_or_default())
					})}
					{make_proficiencies_section_long("Weapons", &proficiencies.weapons, WeaponProficiency::display_name)}
					{make_proficiencies_section_long("Tools", &proficiencies.tools, String::to_string)}
				</div>},
			)
		}
	});
	html! {
		<div id="proficiencies-container" class="card my-1" style="border-color: var(--theme-frame-color);" {onclick}>
			<div class="card-body" style="padding: 5px;">
				<h5 class="card-title text-center" style="font-size: 0.8rem;">{"Proficiencies"}</h5>
				{make_proficiencies_section("Languages", &proficiencies.languages, String::to_string)}
				{make_proficiencies_section("Armor", &proficiencies.armor, |(value, context)| {
					format!("{}{}", value.to_string(), context.as_ref().map(|s| format!(" ({s})")).unwrap_or_default())
				})}
				{make_proficiencies_section("Weapons", &proficiencies.weapons, WeaponProficiency::display_name)}
				{make_proficiencies_section("Tools", &proficiencies.tools, String::to_string)}
			</div>
		</div>
	}
}

fn make_proficiencies_section<T, F>(title: &str, values: &AttributedValueMap<T>, to_string: F) -> Html
where
	F: Fn(&T) -> String,
{
	let count = values.len();
	let mut items = Vec::with_capacity(count);
	for (idx, (value, sources)) in values.iter().enumerate() {
		let is_last = idx == count - 1;
		let tooltip = crate::data::as_feature_paths_html(sources.iter());
		items.push(html! {
			<span>
				<Tooltip tag="span" content={tooltip} use_html={true}>
					{to_string(value)}
				</Tooltip>
				{match is_last {
					false => ", ",
					true => "",
				}}
			</span>
		});
	}
	html! {
		<div class="proficiency-section">
			<h6>{title}</h6>
			<span class="proficiencies">{match !items.is_empty() {
				false => html! { {"None"} },
				true => html! {<> {items} </>},
			}}</span>
		</div>
	}
}

fn make_proficiencies_section_long<T, F>(title: &str, values: &AttributedValueMap<T>, to_string: F) -> Html
where
	F: Fn(&T) -> String,
{
	let count = values.len();
	let mut items = Vec::with_capacity(count);
	for (value, sources) in values.iter() {
		items.push(html! {
			<tr>
				<td class="text-center">{to_string(value)}</td>
				<td>
					{sources.iter().map(|path| html! {
						<div>
							{crate::data::as_feature_path_text(path)}
						</div>
					}).collect::<Vec<_>>()}
				</td>
			</tr>
		});
	}

	let has_content = !items.is_empty();
	let mut section_classes = classes!("proficiency-section");
	if !has_content {
		section_classes.push("text-center");
	}
	let title_header =
		(!has_content).then(|| html! { <h5 style="font-size: 1.1rem;">{title}</h5> }).unwrap_or_default();
	let content = html! {
		<table class="table table-compact table-striped m-0">
			<thead>
				<tr class="text-center" style="font-size: 1.1rem; color: var(--bs-heading-color);">
					<th scope="col" style="width: 200px;">{title}</th>
					<th scope="col">{"Sources"}</th>
				</tr>
			</thead>
			<tbody>{items}</tbody>
		</table>
	};

	html! {
		<div class={section_classes} style={"border-style: none;"}>
			{title_header}
			<div>{match has_content {
				false => html! { {"None"} },
				true => content,
			}}</div>
		</div>
	}
}
