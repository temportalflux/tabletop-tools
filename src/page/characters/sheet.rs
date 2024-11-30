use crate::{
	components::{
		database::{self, use_query},
		mobile, Spinner,
	},
	database::{module::ModuleInSystem, Module, Query},
	storage::autosync,
	system::{
		dnd5e::{components::GeneralProp, DnD5e},
		SourceId,
	},
};
use yew::prelude::*;

mod handle;
pub use handle::*;
pub mod joined;
pub mod paged;

#[derive(Clone, Copy, PartialEq, Debug)]
enum View {
	Display,
	Editor,
}

#[derive(Clone, PartialEq, Properties)]
pub struct ViewProps {
	pub swap_view: Callback<()>,
}

#[function_component]
pub fn Sheet(props: &GeneralProp<SourceId>) -> Html {
	let character = use_character(props.value.clone());

	let autosync_channel = use_context::<autosync::Channel>().unwrap();
	crate::components::hook::use_document_visibility({
		let character = character.clone();
		let autosync_channel = autosync_channel.clone();
		move |vis| {
			if vis == web_sys::VisibilityState::Visible && character.is_loaded() {
				// TODO: This should not use autosync, instead there should be a separate request channel for handling per-character storage requests.
				autosync_channel.try_send_req(autosync::Request::UpdateFile(character.id().clone()));
			}
		}
	});

	let screen_size = mobile::use_mobile_kind();
	let view_handle = use_state_eq({
		let is_new = !props.value.has_path();
		move || match is_new {
			true => View::Editor,
			false => View::Display,
		}
	});
	let swap_view = Callback::from({
		let view_handle = view_handle.clone();
		move |_| {
			view_handle.set(match *view_handle {
				View::Display => View::Editor,
				View::Editor => View::Display,
			});
		}
	});

	use_effect_with(props.value.clone(), {
		let character = character.clone();
		move |id: &SourceId| {
			if character.is_loaded() {
				log::info!("Reloading character with updated id {id:?}");
				character.unload();
			}
		}
	});
	if !character.is_loaded() {
		return html!(<Spinner />);
	}

	let content = match (screen_size, *view_handle) {
		(mobile::Kind::Desktop, View::Display) => {
			html!(<joined::Display {swap_view} />)
		}
		(mobile::Kind::Desktop, View::Editor) => {
			html!(<joined::editor::Editor {swap_view} />)
		}
		(mobile::Kind::Mobile, View::Display) => {
			html!(<paged::Display {swap_view} />)
		}
		(mobile::Kind::Mobile, View::Editor) => {
			html!("Paged Editor TODO")
		}
	};
	html! {
		<ContextProvider<CharacterHandle> context={character.clone()}>
			<div class="w-100 h-100" style="--theme-frame-color: #BA90CB; --theme-frame-color-muted: #BA90CB80; --theme-roll-modifier: #ffffff;">
				<div class="page-root d-flex flex-row">
					<SheetSidebar />
					{content}
				</div>
				<crate::components::context_menu::ContextMenu />
			</div>
		</ContextProvider<CharacterHandle>>
	}
}

#[function_component]
pub fn SheetSidebar() -> Html {
	// TODO: maybe use some variant of OffCanvas Vertical NavBar to group both app nav and character nav into the same left-vertical sidebar
	// https://getbootstrap.com/docs/5.3/components/navbar/#offcanvas
	// Or the sidebar is just for in-character mode where the character's name and details + sync status are in the side bar,
	// and the main panel is the character's stats.
	// TODO: Autosync when installing modules should prevent any changes to modules or character pages.
	// There should be a fullscreen takeover of the content of those pages until syncing/installing is complete.
	html! {
		<div class="sheet-sidebar d-flex flex-row">
			<div class="content collapse collapse-horizontal" id="sidebar-collapse">
				<div class="content d-flex flex-column">
					
				</div>
			</div>
			<i
				type="button" data-bs-toggle="collapse" data-bs-target="#sidebar-collapse" aria-expanded="false"
				class="bi bi-chevron-bar-left collapse-toggle close"
			/>
			<i
				type="button" data-bs-toggle="collapse" data-bs-target="#sidebar-collapse" aria-expanded="false"
				class="bi bi-chevron-bar-right collapse-toggle open"
			/>
		</div>
	}
}
