use crate::{
	components::{mobile, Spinner},
	system::{dnd5e::components::GeneralProp, SourceId},
};
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncOptions};

mod handle;
pub use handle::*;
pub mod joined;
pub mod paged;
pub mod sidebar;
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
			<sidebar::Provider>
				<sync::CharacterSyncProvider>
					<SheetContent character_id={props.value.clone()} is_loaded={character.is_loaded()} />
				</sync::CharacterSyncProvider>
			</sidebar::Provider>
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
		<div class="sheet-app" style="--theme-frame-color: #BA90CB; --theme-frame-color-muted: #BA90CB80; --theme-roll-modifier: #ffffff;">
			<div class="sheet d-flex flex-row flex-grow-1">
				<sidebar::Sidebar />
				{(!props.is_loaded).then(|| html!(<div class="loading">
					<Spinner />
					{"Loading character"}
				</div>))}
				{props.is_loaded.then_some(content)}
			</div>
			<crate::components::context_menu::ContextMenu />
		</div>
	)
}
