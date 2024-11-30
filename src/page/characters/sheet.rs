use crate::{
	components::{
		database::{self, use_query},
		mobile, Spinner,
	},
	database::{module::ModuleInSystem, Module, Query},
	page::app::AutosyncStatusDisplay,
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

#[hook]
fn use_update_character_modules(character: &CharacterHandle) {
	use crate::system::System;
	use futures_util::FutureExt;
	let is_first_mount_after_load = use_mut_ref(|| true);
	// message channel for initiating module syncing from persistent storage to local data
	let autosync_channel = use_context::<autosync::Channel>().unwrap();
	// (auto) Query for all dnd5e modules that the local user has access to
	let modules_query = use_query(Some(DnD5e::id()), |database, system| {
		async move {
			let query = Query::subset(&database, Some(ModuleInSystem::new(system))).await?;
			Ok(query.collect::<Vec<_>>().await) as Result<Vec<Module>, ::database::Error>
		}
		.boxed_local()
	});

	// When character is loaded, initial module query is received, and this is the first load after mounting,
	// then initiate an autosync request to load the character's data.
	let modules_query_complete = matches!(modules_query.status(), database::QueryStatus::Success(_));
	if character.is_loaded() && modules_query_complete && *is_first_mount_after_load.borrow_mut() {
		*is_first_mount_after_load.borrow_mut() = false;

		if let database::QueryStatus::Success(modules) = modules_query.status() {
			let mut modules_to_sync = Vec::with_capacity(modules.len());

			if let Some(character_module_id) = &character.persistent().id.module {
				let found_module = modules.iter().filter(|module| &module.id == character_module_id).count() > 0;
				if found_module {
					modules_to_sync.push(character_module_id.clone());
				}
			}

			if !modules_to_sync.is_empty() {
				log::info!(target: "character", "Initiating module update request for: {:?}",
					modules_to_sync.iter().map(ToString::to_string).collect::<Vec<_>>()
				);
				autosync_channel.try_send_req(autosync::Request::UpdateModules(modules_to_sync));
			}
		}
	}
}

#[function_component]
pub fn Sheet(props: &GeneralProp<SourceId>) -> Html {
	let character = use_character(props.value.clone());
	use_update_character_modules(&character);

	let autosync_channel = use_context::<autosync::Channel>().unwrap();
	crate::components::hook::use_document_visibility({
		let character = character.clone();
		let autosync_channel = autosync_channel.clone();
		move |vis| {
			if vis == web_sys::VisibilityState::Visible && character.is_loaded() {
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
	let autosync_status = use_context::<autosync::Status>().unwrap();
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
					{autosync_status.is_active().then_some(html!(<AutosyncStatusDisplay value={autosync_status} />))}
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
