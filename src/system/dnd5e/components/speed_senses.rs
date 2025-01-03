use crate::{
	components::context_menu, page::characters::sheet::CharacterHandle, system::dnd5e::data::character::StatOperation,
};
use itertools::Itertools;
use std::{path::PathBuf, sync::Arc};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
struct SingleValueProps {
	title: AttrValue,
	amount: u32,
}

#[function_component]
fn SingleValue(SingleValueProps { title, amount }: &SingleValueProps) -> Html {
	html! {<div>
		<h6 class="text-center" style="font-size: 0.8rem; color: var(--bs-card-title-color);">{title.clone()}</h6>
		<div class="text-center" style="width: 100%;">
			<span style="position: relative; font-size: 26px; font-weight: 500;">
				<span>{*amount}</span>
				<span style="position: absolute; bottom: 2px; font-size: 16px; margin-left: 3px;">{"ft."}</span>
			</span>
		</div>
	</div>}
}

#[function_component]
pub fn SpeedAndSenses() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let divider = (state.speeds().len() > 0 && state.senses().len() > 0)
		.then(|| {
			html! {
				<div class="col-auto p-0"><div class="vr" style="min-height: 100%;" /></div>
			}
		})
		.unwrap_or_else(|| html! {});

	let speed = match state.speeds().len() {
		0 => html! {},
		1 => {
			let (title, value) = state.speeds().iter_compiled().next().unwrap();
			html! {<div class="col">
				<SingleValue title={format!("{title} Speed")} amount={value} />
			</div>}
		}
		// TODO: Walking speed should always be the first entry
		_ => html! {<div class="col">
			<h6 class="text-center" style="font-size: 0.8rem; color: var(--bs-card-title-color);">{"Speeds"}</h6>
			<div style="margin-left: 5px; margin-right: 5px;">
				{state.speeds().iter_compiled().map(|(title, value)| {
					html! {
						<span class="d-flex" style="border-style: solid; border-color: var(--bs-border-color); border-width: 0; border-bottom-width: var(--bs-border-width);">
							<span class="flex-grow-1">{title}</span>
							<span class="ps-2">{value}{"ft."}</span>
						</span>
					}
				}).collect::<Vec<_>>()}
			</div>
		</div>},
	};
	let senses_html = match state.senses().len() {
		0 => html! {},
		1 => {
			let (title, value) = state.senses().iter_compiled().next().unwrap();
			html! {<div class="col">
				<SingleValue title={title.to_owned()} amount={value} />
			</div>}
		}
		_ => html! {<div class="col">
			<h6 class="text-center" style="font-size: 0.8rem; color: var(--bs-card-title-color);">{"Senses"}</h6>
			<div style="margin-left: 5px; margin-right: 5px;">
				{state.senses().iter_compiled().map(|(title, value)| {
					html! {
						<span class="d-flex" style="border-style: solid; border-color: var(--bs-border-color); border-width: 0; border-bottom-width: var(--bs-border-width);">
							<span class="flex-grow-1">{title}</span>
							<span class="ps-2">{value}{"ft."}</span>
						</span>
					}
				}).collect::<Vec<_>>()}
			</div>
		</div>},
	};

	let width = match (state.speeds().len(), state.senses().len()) {
		(0, 1) | (1, 0) => "120px",
		(n, 0) | (0, n) if n > 1 => "160px",
		(1, 1) => "240px",
		_ => "100%",
	};

	let onclick = context_menu::use_control_action({
		move |_, _context| context_menu::Action::open_root(format!("Speeds & Senses"), html!(<Modal />))
	});

	html! {
		<div class="card my-1" style={format!("width: {width};")} {onclick}>
			<div class="card-body" style="padding: 5px 5px;">
				<div class="row" style="--bs-gutter-x: 0;">
					{speed}
					{divider}
					{senses_html}
				</div>
			</div>
		</div>
	}
}

static SENSE_DESC: [(&'static str, &'static str); 3] = [
	(
		"Blindsight",
		"A creature with blindsight can perceive its surroundings \
		without relying on sight, within a specific radius.
		Creatures without eyes, such as grimlocks and gray oozes, typically have this special sense, \
		as do creatures with echolocation or heightened senses, such as bats and true dragons.
		If a creature is naturally blind, it has a parenthetical note to this effect, indicating that \
		the radius of its blindsight defines the maximum range of its perception.",
	),
	(
		"Darkvision",
		"A creature with darkvision can see in the dark within a specific radius. \
		The creature can see in dim light within the radius as if it were bright light, \
		and in darkness as if it were dim light. The creature can't discern color in darkness, \
		only shades of gray. Many creatures that live underground have this special sense.",
	),
	(
		"Tremorsense",
		"A creature with tremorsense can detect and pinpoint the origin of vibrations \
		within a specific radius, provided that the creature and the source of the \
		vibrations are in contact with the same ground or substance.
		Tremorsense can't be used to detect flying or incorporeal creatures. \
		Many burrowing creatures, such as ankhegs, have this special sense.",
	),
];

#[function_component]
fn Modal() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let speed_stats =
		state.speeds().iter_compiled().map(|(name, value)| (Arc::new(name.to_owned()), value)).collect::<Vec<_>>();
	let sense_stats =
		state.senses().iter_compiled().map(|(name, value)| (Arc::new(name.to_owned()), value)).collect::<Vec<_>>();

	html! {<div class="w-100 h-100 scroll-container-y">
		{speed_stats.into_iter().map(|(name, value)| {
			let operations = state.speeds().get(name.as_str());
			let operations = operations.cloned().sorted().collect::<Vec<_>>();
			html!(<Stat kind="Speed" {name} {value} {operations} />)
		}).collect::<Vec<_>>()}
		{sense_stats.into_iter().map(|(name, value)| {
			let operations = state.senses().get(name.as_str());
			let operations = operations.cloned().sorted().collect::<Vec<_>>();
			html!(<Stat kind="Sense" {name} {value} {operations} />)
		}).collect::<Vec<_>>()}

		<div>
			<h6>{"Additional Information"}</h6>
			{SENSE_DESC.iter().map(|(title, desc)| html! {
				<div>
					<strong>{*title}{". "}</strong>
					<span class="text-block" style="font-size: 14px;">
						{*desc}
					</span>
				</div>
			}).collect::<Vec<_>>()}
		</div>
	</div>}
}

#[derive(Clone, PartialEq, Properties)]
struct StatProps {
	kind: &'static str,
	name: Arc<String>,
	value: u32,
	operations: Vec<(StatOperation, PathBuf)>,
}

#[function_component]
fn Stat(StatProps { kind, name, value, operations }: &StatProps) -> Html {
	html!(<div class="mb-2">
		<h4>{name}{" ("}{kind}{") = "}{*value}</h4>
		{operations.into_iter().map(|(operation, source)| {
			let (name, value) = match operation {
				StatOperation::MinimumValue(value) => (html!("Minimum"), html!(*value)),
				StatOperation::MinimumStat(value) => (html!("Minimum"), html!(format!("equal to {value}"))),
				StatOperation::Base(value) => (html!("Base"), html!(*value)),
				StatOperation::MultiplyDivide(value) if *value >= 0 => (html!("Multiply"), html!(value.abs())),
				StatOperation::MultiplyDivide(value) => (html!("Divide"), html!(value.abs())),
				StatOperation::AddSubtract(value) if *value >= 0 => (html!("Add"), html!(value.abs())),
				StatOperation::AddSubtract(value) => (html!("Subtract"), html!(value.abs())),
				StatOperation::MaximumValue(value) => (html!("Maximum"), html!(*value)),
			};
			html! {
				<div class="mx-2 mb-1">
					<strong>{name}</strong>
					<span class="ms-2">{value}</span>
					<span class="ms-2">{crate::data::as_feature_path_text(source)}</span>
				</div>
			}
		}).collect::<Vec<_>>()}
	</div>)
}
