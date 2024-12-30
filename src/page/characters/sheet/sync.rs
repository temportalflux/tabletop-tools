use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

use crate::{
	database::Database,
	page::characters::sheet::CharacterHandle,
	storage::autosync,
	system::{self, ModuleId},
};
use anyhow::Context;
use itertools::Itertools;
use kdlize::FromKdl;
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
	let task = use_async_with_options::<_, _, ()>(
		{
			let character = character.clone();
			let recv_req = channel.receiver().clone();
			let status = status.clone();
			async move {
				// TODO: Bug - wont accept new requests because the loaded state is never updated in this body
				while let Ok(_req) = recv_req.recv().await {
					if !character.is_loaded() {
						continue;
					}
					if let Err(err) = fetch_character_changes(&character, &database, &system_depot, &status).await {
						log::error!(target: "character-sync", "{err:?}");
					}
					status.clear_stages();
				}
				Ok(())
			}
		},
		UseAsyncOptions::default(),
	);

	if character.is_loaded() && !task.loading {
		task.run();
	}

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
	use crate::database::{Entry, Module};
	use database::{ObjectStoreExt, TransactionExt};

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

	status.push_stage("Checking for Updates", None);

	let mut query = match &character.id().module {
		Some(ModuleId::Github { user_org, repository }) => autosync::FindRepository {
			client: storage.clone(),
			owner: user_org.clone(),
			repository: repository.clone(),
		},
		_ => return Ok(()),
	};
	let Some(repository) = query.run().await? else { return Ok(()) };
	let module_id: ModuleId = (&repository).into();

	// Update the module's remote version
	let mut module = None::<Module>;
	{
		let transaction = database.write()?;
		let module_store = transaction.object_store_of::<Module>()?;
		let version_changed =
			autosync::update_module_version(&module_store, &module_id, &repository, &mut module).await?;
		if !version_changed {
			return Ok(());
		}
		transaction.commit().await?;
	}
	let Some(mut module) = module else { return Ok(()) };

	// Determine if the character file itself has an upgrade.
	// If the character has not changed on the remote (even if there was an update to the remote),
	// then we dont have any more to do here.
	let files = autosync::find_module_updates(&mut module, status, &storage).await?;
	let iter_files = files.iter();
	let mut iter_files =
		iter_files.map(|autosync::ModuleFileUpdate { file: autosync::ModuleFile { path_in_repo, .. }, .. }| {
			PathBuf::from(&path_in_repo)
		});
	if !iter_files.contains(&character.id().path) {
		return Ok(());
	}

	status.pop_stage(); // Checking for Updates

	status.push_stage("Fetching Updated Content", None);

	// Download the content from (deep) storage
	let remote_content = match &character.id().module {
		Some(ModuleId::Github { user_org, repository }) => {
			let args = github::repos::contents::get::Args {
				owner: user_org.as_str(),
				repo: repository.as_str(),
				path: Path::new(character.id().path.to_str().unwrap()),
				version: module.remote_version.as_str(),
			};
			storage.get_file_content(args).await?
		}
		_ => return Ok(()),
	};

	status.pop_stage(); // Fetching Updated Content

	// Parse the content as a kdl document, and then as a character
	let document = remote_content
		.parse::<kdl::KdlDocument>()
		.with_context(|| format!("Failed to parse content: {remote_content:?}"))?;
	let mut source_id = character.id().clone();
	source_id.version = Some(module.remote_version.clone());
	let Some(node) = document.nodes().first() else { return Ok(()) };
	let Some(system_reg) = system_depot.get(source_id.system.as_ref().unwrap()) else { return Ok(()) };

	let ctx = crate::kdl_ext::NodeContext::new(Arc::new(source_id.clone()), system_reg.node());
	let mut node_reader = crate::kdl_ext::NodeReader::new_root(node, ctx);
	let _new_persistent = system::dnd5e::data::character::Persistent::from_kdl(&mut node_reader)?;

	// TODO: diff new_persistent against the existing character, stopping if they cannot be reconcilled (or otherwise merging).
	//       if there are unsaved local changes, then there has been an issue and the local changes must be stomped on.
	//         Should display a prompt (and a diff view one day when there is a changelog).

	status.push_stage("Updating character", None);

	// update the character idb entry with the new content
	{
		let transaction = database.write()?;
		let entry_store = transaction.object_store_of::<Entry>();
		let entry_store = entry_store.context("Entry store")?;

		let Some(mut entry): Option<Entry> = entry_store.get_record(character.id().to_string()).await? else {
			return Ok(());
		};
		entry.kdl = node.to_string();
		entry.version = source_id.version.clone();
		entry.metadata = system_reg.parse_metadata(node, &source_id)?;
		entry_store.put_record(&entry).await?;

		transaction.commit().await?;
	}
	status.pop_stage(); // Updating character

	Ok(())
}
