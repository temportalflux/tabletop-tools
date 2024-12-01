use crate::{
	components::{mobile, Spinner},
	system::{dnd5e::components::GeneralProp, SourceId},
};
use yew::prelude::*;

mod handle;
pub use handle::*;
use yew_hooks::{use_async_with_options, UseAsyncOptions};
pub mod joined;
pub mod paged;
pub mod sync;

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
	html! {
		<ContextProvider<CharacterHandle> context={character.clone()}>
			<sync::CharacterSyncProvider>
				<SheetContent character_id={props.value.clone()} is_loaded={character.is_loaded()} />
			</sync::CharacterSyncProvider>
		</ContextProvider<CharacterHandle>>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct SheetContentProps {
	character_id: SourceId,
	is_loaded: bool,
}

#[function_component]
fn SheetContent(props: &SheetContentProps) -> Html {
	// Query character for version updates at a regular interval (e.g. 5 minutes)
	let character_sync_channel = use_context::<sync::Channel>().unwrap();
	use_async_with_options::<_, (), ()>(
		{
			let channel = character_sync_channel.clone();
			let duration_between_updates = std::time::Duration::from_secs(60 * 5);
			async move {
				loop {
					channel.try_send_req(());
					let _ = wasm_timer::Delay::new(duration_between_updates).await;
				}
			}
		},
		UseAsyncOptions::enable_auto(),
	);

	let screen_size = mobile::use_mobile_kind();
	let view_handle = use_state_eq({
		let is_new = !props.character_id.has_path();
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

	html!(
		<div class="w-100 h-100" style="--theme-frame-color: #BA90CB; --theme-frame-color-muted: #BA90CB80; --theme-roll-modifier: #ffffff;">
			<div class="page-root d-flex flex-row">
				<SheetSidebar />
				{(!props.is_loaded).then(|| html!(<div>
					<Spinner />
					{"Loading character"}
				</div>))}
				{props.is_loaded.then_some(content)}
			</div>
			<crate::components::context_menu::ContextMenu />
		</div>
	)
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
					<CharacterSyncStatusDisplay />
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
