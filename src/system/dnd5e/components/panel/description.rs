use crate::{
	page::characters::sheet::{joined::pronouns, CharacterHandle},
	system::dnd5e::change::ApplyDescription,
	utility::InputExt,
};
use yew::prelude::*;

#[function_component]
pub fn Description() -> Html {
	html! {
		<div class="panel description">
			<AgePronouns />
			<div class="hr my-2" />
			<SizeHeightWeight />
			<div class="hr my-2" />
			<Personality />
			<div class="hr my-2" />
			<AppearanceEditor />
		</div>
	}
}

#[derive(Clone, PartialEq, Default)]
enum HeightDisplay {
	#[default]
	ImperialMixed,
	Imperial,
	Metric,
}
impl HeightDisplay {
	fn format(&self, inches: u32) -> String {
		match self {
			Self::Imperial => format!("{inches} inches"),
			Self::ImperialMixed => format!("{} ft {} inches", inches / 12, inches % 12),
			Self::Metric => format!("{} cm", ((inches as f32) * 2.54).round() as u32),
		}
	}
}

#[derive(Clone, PartialEq, Default)]
enum WeightDisplay {
	#[default]
	Imperial,
	Metric,
}
impl WeightDisplay {
	fn format(&self, lbs: u32) -> String {
		match self {
			Self::Imperial => format!("{lbs} lbs"),
			Self::Metric => format!("{} kgs", ((lbs as f32) * 0.45359237).round() as u32),
		}
	}
}

#[function_component]
pub fn SizeHeightWeight() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let height_display = use_state_eq(|| HeightDisplay::default());
	let weight_display = use_state_eq(|| WeightDisplay::default());

	// TODO: Save and load display settings from user preferences
	let onclick_height = Callback::from({
		let height_display = height_display.clone();
		move |_| {
			height_display.set(match *height_display {
				HeightDisplay::ImperialMixed => HeightDisplay::Imperial,
				HeightDisplay::Imperial => HeightDisplay::Metric,
				HeightDisplay::Metric => HeightDisplay::ImperialMixed,
			});
		}
	});
	let onclick_weight = Callback::from({
		let weight_display = weight_display.clone();
		move |_| {
			weight_display.set(match *weight_display {
				WeightDisplay::Imperial => WeightDisplay::Metric,
				WeightDisplay::Metric => WeightDisplay::Imperial,
			});
		}
	});

	html! {<>
		<div class="row text-center">
			<div class="col">
				<label style="font-weight: 700;">{"Size"}</label>
				<span style="display: block;">{state.persistent().description.size().to_string()}</span>
			</div>
			<div class="col" onclick={onclick_height}>
				<label style="font-weight: 700;">{"Height"}</label>
				<span style="display: block;">
					{height_display.format(state.persistent().description.height)}
				</span>
			</div>
			<div class="col" onclick={onclick_weight}>
				<label style="font-weight: 600;">{"Weight"}</label>
				<span style="display: block;">
					{weight_display.format(state.persistent().description.weight)}
				</span>
			</div>
		</div>
		<div class="form-text">
			{state.persistent().description.size().description()}
		</div>
	</>}
}

#[function_component]
pub fn AgePronouns() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	html! {
		<div class="row">
			<div class="col-auto">
				<label style="font-weight: 700;">{"Age"}</label>
				<span class="text-center" style="display: block;">
					{match state.persistent().description.age {
						0 => html!("--"),
						age => html!({age}),
					}}
				</span>
			</div>
			<div class="col">
				<label style="font-weight: 700;">{"Pronouns"}</label>
				<span style="display: block;">{pronouns(&state.persistent().description).unwrap_or_default()}</span>
			</div>
		</div>
	}
}

#[function_component]
pub fn Personality() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	html! {
		<div class="mb-3">
			{state.persistent().description.personality.iter().map(|(kind, traits)| {
				html! {
					<div>
						<label style="font-weight: 700;">{kind.to_string()}</label>
						<ul style="margin-bottom: 0;">
							{traits.iter().map(move |trait_str| {
								html!(<li>{trait_str}</li>)
							}).collect::<Vec<_>>()}
						</ul>
					</div>
				}
			}).collect::<Vec<_>>()}
		</div>
	}
}

#[function_component]
pub fn AppearanceEditor() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	// TODO: Display appearance as plaintext (no editor), and switch to a textarea
	// with fixed rows equal to (lines + 5) when double clicked, reverting back when focus is lost
	let onchange_appearance = state.dispatch_change(move |evt: web_sys::Event| {
		let value = evt.input_value()?;
		Some(ApplyDescription::Appearance(value))
	});

	html! {
		<div class="mb-3">
			<label class="form-label" for="appearanceInput" style="font-weight: 700;">{"Appearance"}</label>
			<textarea
				class="form-control" id="appearanceInput"
				rows="5"
				onchange={onchange_appearance}
				value={state.persistent().description.appearance.clone()}
			/>
		</div>
	}
}
