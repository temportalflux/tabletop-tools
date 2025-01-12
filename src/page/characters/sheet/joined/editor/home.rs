use crate::{
	page::characters::sheet::CharacterHandle,
	system::dnd5e::change::{ApplyDescription, UpdateSettings},
	utility::InputExt,
};
use yew::prelude::*;

#[function_component]
pub fn HomeTab() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	// TODO: In here is where module selection will go.
	// Modules enabled for the character is a subset of the modules the user has access to.
	// Only modules enabled for the character are available in the editor/viewer.

	// More robust pronoun selection? https://twitter.com/Patch_Games/status/1423706763841347586
	html! {<div class="mx-4">
		<div class="my-3">
			<div class="d-flex align-items-center mb-2">
				<h4 class="mb-0">{"Character Info"}</h4>
				<span class="ms-auto" style="color: rgba(var(--bs-body-color-rgb), .65);">
					{"id: "}
					{match state.id().path.components().last() {
						Some(comp) => comp.as_os_str().to_str().unwrap().strip_suffix(".kdl").unwrap(),
						None => "Not Yet Saved",
					}}
				</span>
			</div>
			<p>{"Who is your character? These options are also available in the Description tab."}</p>
			<div class="row">
				<div class="col-5">
					<NameEditor />
				</div>
				<div class="col">
					<PronounEditor />
				</div>
			</div>
		</div>
		<div class="my-3">
			<SettingsEditor />
		</div>
	</div>}
}

#[function_component]
fn NameEditor() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onchange = state.dispatch_change(move |evt: web_sys::Event| {
		let value = evt.input_value()?;
		Some(ApplyDescription::Name(value))
	});
	html! {<>
		<label for="nameInput" class="form-label">{"Name"}</label>
		<input  id="nameInput" class="form-control" type="text"
			value={state.persistent().description.name.clone()}
			{onchange}
		/>
	</>}
}

#[function_component]
fn PronounEditor() -> Html {
	static PROVIDED_OPTIONS: [(&'static str, &'static str); 3] =
		[("she/her", "She / Her"), ("he/him", "He / Him"), ("they/them", "They / Them")];
	let state = use_context::<CharacterHandle>().unwrap();
	let onchange = state.dispatch_change(move |evt: web_sys::Event| {
		let input = evt.target_input()?;
		let is_checkbox = input.type_() == "checkbox";
		let is_checked = input.checked();
		let value = input.value();
		match (is_checkbox, is_checked) {
			(true, true) => Some(ApplyDescription::InsertPronoun(value)),
			(true, false) => Some(ApplyDescription::RemovePronoun(value)),
			(false, _) => Some(ApplyDescription::CustomPronouns(value.trim().to_owned())),
		}
	});
	html! {
		<div class="pronouns-group">
			<label for="pronouns" class="form-label">{"Pronouns"}</label>
			<div class="input-group">
				{PROVIDED_OPTIONS.iter().map(|(value, label)| html! {
					<div class="input-group-text">
						<label for={value.to_owned()} class="form-check-label me-2">{label.to_owned()}</label>
						<input  id={value.to_owned()}
							class="form-check-input mt-0 success" type="checkbox"
							value={value.to_owned()}
							checked={state.persistent().description.pronouns.contains(*value)}
							onchange={onchange.clone()}
						/>
					</div>
				}).collect::<Vec<_>>()}
				<input id="pronouns"
					class="form-control" type="text"
					placeholder="and/or add custom pronouns"
					onchange={onchange.clone()}
					value={state.persistent().description.custom_pronouns.clone()}
				/>
			</div>
		</div>
	}
}

#[function_component]
fn SettingsEditor() -> Html {
	html! {<>
		<h4>{"Settings"}</h4>
		<AutoExchangeSwitch />
	</>}
}

#[function_component]
pub fn AutoExchangeSwitch() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onchange = state.dispatch_change(|evt: web_sys::Event| {
		let value = evt.input_checked()?;
		Some(UpdateSettings::AutoExchange(value))
	});
	html! {
		<div class="form-check form-switch">
			<input
				class="form-check-input"
				type="checkbox" role="switch" id="auto_exchange"
				aria-describedby="auto_exchange-help"
				onchange={onchange}
				checked={state.persistent().settings.currency_auto_exchange}
			/>
			<label class="form-check-label" for="auto_exchange">
				<strong>{"Currency: "}</strong>
				{"Auto-Exchange"}
			</label>
			<div id="auto_exchange-help" class="form-text text-block">
				{"If enabled, coinage will be automatically exchanged for smaller kinds when removing coinage.
				The exchange button also becomes available, which allows you to convert smaller coinage \
				into the largest possible coins."}
			</div>
		</div>
	}
}
