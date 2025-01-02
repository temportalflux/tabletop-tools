use std::rc::Rc;

use crate::page::characters::sheet::sync;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct Control(UseReducerHandle<State>);
impl From<UseReducerHandle<State>> for Control {
	fn from(value: UseReducerHandle<State>) -> Self {
		Self(value)
	}
}
impl std::ops::Deref for Control {
	type Target = State;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, PartialEq, Default)]
pub struct State {
	is_shown: bool,
}
impl Reducible for State {
	type Action = bool;

	fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
		Rc::new(Self { is_shown: action })
	}
}

#[function_component]
pub fn Provider(props: &html::ChildrenProps) -> Html {
	let control = Control::from(use_reducer(|| State::default()));
	html! {
		<ContextProvider<Control> context={control.clone()}>
			{props.children.clone()}
		</ContextProvider<Control>>
	}
}

#[function_component]
pub fn Sidebar() -> Html {
	let Some(state) = use_context::<crate::page::characters::sheet::CharacterHandle>() else { return html!() };
	let Some(control) = use_context::<Control>() else { return html!() };

	let collapse = Callback::from({
		let control = control.clone();
		move |_: MouseEvent| control.0.dispatch(false)
	});
	let expand = Callback::from({
		let control = control.clone();
		move |_: MouseEvent| control.0.dispatch(true)
	});

	let changelist_desc = state.is_loaded().then(|| html!(<div>
		<div>{"Changes:"}</div>
		{state.persistent().changelist().iter().rev().map(|change| {
			html!(<div>{format!("{change:?}")}</div>)
		}).collect::<Vec<_>>()}
	</div>));

	// TODO: Autosync when installing modules should prevent any changes to modules or character pages.
	// There should be a fullscreen takeover of the content of those pages until syncing/installing is complete.
	html! {
		<div class={classes!("sidebar", control.is_shown.then(|| "active"))}>
			<div class="content">
				<CharacterSyncStatusDisplay />
				{changelist_desc}
			</div>
			<i type="button" class="bi bi-chevron-bar-left close" onclick={collapse} />
			<i type="button" class="bi bi-chevron-bar-right open" onclick={expand} />
		</div>
	}
}

#[function_component]
pub fn CharacterSyncStatusDisplay() -> Html {
	let status = use_context::<sync::Status>().unwrap();
	html! {
		<div class="character-sync-status d-flex justify-content-start align-items-center">
			<div class="d-flex flex-column align-items-center">
				{status.stages().iter().enumerate().map(|(idx, stage)| {
					html! {
						<crate::page::app::SyncStateDisplay
							id={idx.to_string()} stage={stage.clone()} title_classes={classes!()}
						/>
					}
				}).collect::<Vec<_>>()}
			</div>
		</div>
	}
}
