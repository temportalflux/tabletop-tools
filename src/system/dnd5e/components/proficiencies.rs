use crate::{
	bootstrap::components::Tooltip,
	components::modal,
	system::dnd5e::{components::SharedCharacter, data::AttributedValueMap},
};
use yew::prelude::*;

#[function_component]
pub fn Proficiencies() -> Html {
	let state = use_context::<SharedCharacter>().unwrap();
	let modal_dispatcher = use_context::<modal::Context>().unwrap();
	let proficiencies = state.other_proficiencies();
	let onclick = modal_dispatcher.callback({
		let state = state.clone();
		move |_| {
			let proficiencies = state.other_proficiencies();
			modal::Action::Open(modal::Props {
				centered: true,
				scrollable: true,
				content: html! {<>
					<div class="modal-header">
						<h1 class="modal-title fs-4">{"General Proficiencies"}</h1>
						<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
					</div>
					<div class="modal-body">
						{make_proficiencies_section_long("Languages", &proficiencies.languages)}
						{make_proficiencies_section_long("Armor", &proficiencies.armor)}
						{make_proficiencies_section_long("Weapons", &proficiencies.weapons)}
						{make_proficiencies_section_long("Tools", &proficiencies.tools)}
					</div>
				</>},
				..Default::default()
			})
		}
	});
	html! {
		<div id="proficiencies-container" class="card my-1 mx-auto" style="max-width: 200px; border-color: var(--theme-frame-color);" {onclick}>
			<div class="card-body" style="padding: 5px;">
				<h5 class="card-title text-center" style="font-size: 0.8rem;">{"Proficiencies"}</h5>
				{make_proficiencies_section("Languages", &proficiencies.languages)}
				{make_proficiencies_section("Armor", &proficiencies.armor)}
				{make_proficiencies_section("Weapons", &proficiencies.weapons)}
				{make_proficiencies_section("Tools", &proficiencies.tools)}
			</div>
		</div>
	}
}

fn make_proficiencies_section<T>(title: &str, values: &AttributedValueMap<T>) -> Html
where
	T: ToString,
{
	let count = values.len();
	let mut items = Vec::with_capacity(count);
	for (idx, (value, sources)) in values.iter().enumerate() {
		let is_last = idx == count - 1;
		let tooltip = crate::data::as_feature_paths_html(sources.iter());
		items.push(html! {
			<span>
				<Tooltip tag="span" content={tooltip} use_html={true}>
					{value.to_string()}
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
			<span>{match !items.is_empty() {
				false => html! { {"None"} },
				true => html! {<> {items} </>},
			}}</span>
		</div>
	}
}

fn make_proficiencies_section_long<T>(title: &str, values: &AttributedValueMap<T>) -> Html
where
	T: ToString,
{
	let count = values.len();
	let mut items = Vec::with_capacity(count);
	for (value, sources) in values.iter() {
		items.push(html! {
			<tr>
				<td class="text-center">{value.to_string()}</td>
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
	let title_header = (!has_content)
		.then(|| html! { <h5 style="font-size: 1.1rem;">{title}</h5> })
		.unwrap_or_default();
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
