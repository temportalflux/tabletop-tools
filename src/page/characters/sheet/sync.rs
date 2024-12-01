use crate::{
	database::Database,
	page::characters::sheet::CharacterHandle,
	storage::autosync,
	system::{self, ModuleId},
};
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncOptions};

pub type Request = ();

#[derive(Clone, PartialEq)]
pub struct Channel(autosync::channel::Channel<Request>);
impl std::ops::Deref for Channel {
	type Target = autosync::channel::Channel<Request>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, PartialEq)]
pub struct Status(autosync::status::Status);
impl std::ops::Deref for Status {
	type Target = autosync::status::Status;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[function_component]
pub fn CharacterSyncProvider(props: &html::ChildrenProps) -> Html {
	let character = use_context::<CharacterHandle>().unwrap();
	let database = use_context::<Database>().unwrap();
	let system_depot = use_context::<system::Registry>().unwrap();

	let channel = Channel(autosync::channel::use_channel::<Request>());
	let status = Status(autosync::status::use_status());
	use_async_with_options::<_, _, ()>(
		{
			let character = character.clone();
			let recv_req = channel.receiver().clone();
			let status = status.clone();
			async move {
				while let Ok(_req) = recv_req.recv().await {
					if let Err(err) = fetch_character_changes(&character, &database, &system_depot, &status).await {
						log::error!(target: "character-sync", "{err:?}");
					}
				}
				Ok(())
			}
		},
		UseAsyncOptions::enable_auto(),
	);

	html! {
		<ContextProvider<Channel> context={channel}>
			<ContextProvider<Status> context={status}>
				{props.children.clone()}
			</ContextProvider<Status>>
		</ContextProvider<Channel>>
	}
}

async fn fetch_character_changes(
	character: &CharacterHandle, database: &Database, system_depot: &system::Registry, status: &Status,
) -> Result<(), autosync::StorageSyncError> {
	#[cfg(target_family = "wasm")]
	let storage = {
		let auth_status = yewdux::Dispatch::<crate::auth::Status>::global().get();
		let Some(storage) = crate::storage::get(&*auth_status) else {
			log::error!(target: "character-sync", "No storage available, cannot process request");
			return Ok(());
		};
		storage
	};
	#[cfg(target_family = "windows")]
	let storage = github::GithubClient::new("INVALID", crate::storage::APP_USER_AGENT).unwrap();

	let mut query = match &character.id().module {
		Some(ModuleId::Github { user_org, repository }) => autosync::FindRepository {
			client: storage.clone(),
			owner: user_org.clone(),
			repository: repository.clone(),
		},
		_ => return Ok(()),
	};
	let Some(repository) = query.run().await? else { return Ok(()) };

	// TODO: update character remote version

	Ok(())
}
