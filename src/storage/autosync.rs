use crate::{
	auth::OAuthProvider,
	components::auth::LocalUser,
	database::{Database, Entry, Module, Query, UserSettingsRecord},
	system::{self, generator, generics, ModuleId, SourceId},
};
use anyhow::{Context, Error};
use database::Transaction;
use futures::StreamExt;
use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	sync::Arc,
};
use yew::{html::ChildrenProps, prelude::*};
use yew_hooks::*;

pub mod channel;
mod operations;
pub mod status;
use operations::*;

#[derive(Debug, Clone)]
pub enum Request {
	// Only poll for what the latest version is of all installed modules.
	// This should not actually download any updates.
	FetchLatestVersionAllModules,
	// Download and install modules (or uninstall from database).
	InstallModules(HashMap<ModuleId, /*install vs uninstall*/ bool>),
	// Download updates to modules that are out of date (does not query for versions in storage).
	UpdateModules(Vec<ModuleId>),
	// Poll what the latest version is for this specific source file.
	// If there is an update, download the updates.
	UpdateFile(SourceId),
}

// The async request channel of the global autosync operation.
#[derive(Clone, PartialEq)]
pub struct Channel(channel::Channel<Request>);
impl std::ops::Deref for Channel {
	type Target = channel::Channel<Request>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

// The status of the global autosync operation.
#[derive(Clone, PartialEq)]
pub struct Status(status::Status);
impl std::ops::Deref for Status {
	type Target = status::Status;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(thiserror::Error, Debug, Clone)]
enum StorageSyncError {
	#[error(transparent)]
	Database(#[from] database::Error),
	#[error(transparent)]
	StorageError(#[from] github::Error),
	#[error("{0}")]
	Generic(String),
}
impl From<anyhow::Error> for StorageSyncError {
	fn from(value: Error) -> Self {
		Self::Generic(format!("{value:?}"))
	}
}

#[function_component]
pub fn Provider(props: &ChildrenProps) -> Html {
	let database = use_context::<Database>().unwrap();
	let system_depot = use_context::<crate::system::Registry>().unwrap();
	let channel = Channel(channel::use_channel::<Request>());
	let status = Status(status::use_status());
	use_async_with_options::<_, _, StorageSyncError>(
		{
			let database = database.clone();
			let system_depot = system_depot.clone();
			let recv_req = channel.receiver().clone();
			let status = status.clone();
			async move {
				while let Ok(req) = recv_req.recv().await {
					if let Err(err) = process_request(req, &database, &system_depot, &status.0).await {
						log::error!(target: "autosync", "{err:?}");
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

async fn process_request(
	req: Request, database: &Database, system_depot: &system::Registry, status: &status::Status,
) -> Result<(), StorageSyncError> {
	#[cfg(target_family = "wasm")]
	let storage = {
		let auth_status = yewdux::Dispatch::<crate::auth::Status>::global().get();
		let Some(storage) = crate::storage::get(&*auth_status) else {
			log::error!(target: "autosync", "No storage available, cannot progess request {req:?}");
			return Ok(());
		};
		storage
	};
	#[cfg(target_family = "windows")]
	let storage = github::GithubClient::new("INVALID", crate::storage::APP_USER_AGENT).unwrap();

	#[cfg(target_family = "wasm")]
	let user_id = yewdux::Dispatch::<LocalUser>::global();
	#[cfg(target_family = "windows")]
	let user_id = yewdux::Dispatch::<LocalUser>::new(&Default::default());

	let mut scan_storage_for_modules = false;
	let mut modules = BTreeMap::new();
	let mut modules_to_fetch = BTreeSet::new();
	let mut modules_to_install = BTreeSet::new();
	let mut modules_to_uninstall = BTreeSet::new();
	// list of systems where content has been added to, updated, or removed from.
	let mut systems_changed = BTreeSet::new();
	match req {
		Request::FetchLatestVersionAllModules => {
			scan_storage_for_modules = true;
			let mut query = Query::<Module>::all(&database).await?;
			while let Some(module) = query.next().await {
				modules.insert(module.id.clone(), module);
			}
		}
		Request::InstallModules(new_installation_status) => {
			for (id, should_be_installed) in new_installation_status {
				let module = database.get::<Module>(id.to_string()).await?;
				let Some(module) = module else {
					continue;
				};
				if should_be_installed {
					modules_to_install.insert(module.id.clone());
				} else {
					modules_to_uninstall.insert(module.id.clone());
				}
				modules.insert(module.id.clone(), module);
			}
		}
		Request::UpdateModules(module_ids) => {
			for id in module_ids {
				let module = database.get::<Module>(id.to_string()).await?;
				let Some(module) = module else {
					continue;
				};
				modules_to_fetch.insert(module.id.clone());
				modules_to_install.insert(module.id.clone());
				modules.insert(module.id.clone(), module);
			}
		}
		Request::UpdateFile(source_id) => {
			if let Some(id) = source_id.module {
				let module = database.get::<Module>(id.to_string()).await?;
				let Some(module) = module else {
					log::error!(target: "autosync", "Failed to find module {id:?}");
					return Ok(());
				};
				modules_to_fetch.insert(module.id.clone());
				modules_to_install.insert(module.id.clone());
				modules.insert(module.id.clone(), module);
			}
		}
	}

	status.push_stage("Checking authentiation", None);
	let (viewer, repo_owners) = {
		let mut query_module_owners =
			QueryModuleOwners { status: status.clone(), client: storage.clone(), user: None, found_homebrew: false };
		let owners = query_module_owners.run().await?;

		// If the homebrew repo was not found when querying who the user is,
		// then we need to generate one, since this is where their user data is stored
		// and is the default location for any creations.
		if !query_module_owners.found_homebrew {
			let generate_homebrew = GenerateHomebrew { status: status.clone(), client: storage.clone() };
			generate_homebrew.run().await?;
		}

		(query_module_owners.user.take(), owners)
	};
	status.pop_stage();

	user_id.set(match &viewer {
		None => LocalUser::default(),
		Some(viewer) => {
			let user_id = crate::data::UserId { provider: OAuthProvider::Github, id: viewer.clone() };
			LocalUser::from(user_id)
		}
	});

	// Scan storage or check for updates
	let mut remote_repositories = BTreeMap::new();
	if scan_storage_for_modules {
		status.push_stage("Scanning Storage", None);
		let scan_for_modules = ScanForModules { client: storage.clone(), owners: repo_owners };
		let repositories = scan_for_modules.run().await?;
		for repository in repositories {
			remote_repositories.insert((&repository).into(), repository);
		}
		status.pop_stage();
	} else {
		status.push_stage("Checking for module updates", None);
		let module_names = modules_to_fetch.iter().map(ModuleId::to_string).collect();
		let mut find_modules = FindModules { status: status.clone(), client: storage.clone(), names: module_names };
		let repositories = find_modules.run().await?;
		for repository in repositories {
			remote_repositories.insert((&repository).into(), repository);
		}
		status.pop_stage();
	}

	// Update module versions or add modules
	if !remote_repositories.is_empty() {
		use database::{ObjectStoreExt, TransactionExt};
		status.push_stage("Updating database", None);

		let transaction = database.write()?;
		let module_store = transaction.object_store_of::<Module>()?;

		for (module_id, repository) in remote_repositories {
			// if the module is already in memory, then update that entry and include in database update
			if let Some(module) = modules.get_mut(&module_id) {
				module.remote_version = repository.version.clone();
				module_store.put_record(module).await?;
				continue;
			}
			// if the module is not in memory, but could be in the database (not all requests fill the modules list),
			// then we fetch from database, update the version, and copy the entry both back to database and to our in-memory listing.
			if let Some(mut module) = module_store.get_record::<Module>(module_id.to_string()).await? {
				module.remote_version = repository.version.clone();
				if !module.installed {
					module.version = module.remote_version.clone();
				}
				module_store.put_record(&module).await?;
				modules.insert(module_id.clone(), module);
				continue;
			}
			// doesn't exist at all, so we need a new entry to be added to database.
			let module = Module {
				id: module_id.clone(),
				name: module_id.to_string(),
				systems: repository.root_trees.iter().cloned().collect(),
				version: repository.version.clone(),
				remote_version: repository.version.clone(),
				installed: false,
			};
			module_store.put_record(&module).await?;
			modules.insert(module_id.clone(), module);
		}

		transaction.commit().await?;

		status.pop_stage();
	}

	// Install homebrew
	if let Some(homebrew_id) = user_id.get().homebrew_module_id() {
		modules_to_uninstall.remove(&homebrew_id);
		if let Some(module) = modules.get(&homebrew_id) {
			if !module.installed {
				modules_to_install.insert(homebrew_id);
				//log::debug!(target: "autosync", "homebrew isn't installed and it should be");
			}
		}
	}

	//log::debug!(target: "autosync", "To Uninstall: {modules_to_uninstall:?}");
	//log::debug!(target: "autosync", "To Install: {modules_to_install:?}");

	// Uninstall undesired modules
	// This deletes all content from those modules, including variants of content in the modules
	// (variants are created by generators using some basis, and thus considered a part of the original module).
	if !modules_to_uninstall.is_empty() {
		let transaction = database.write()?;
		for module_id in &modules_to_uninstall {
			use crate::database::entry::ModuleSystem;
			use database::{ObjectStoreExt, TransactionExt};

			let Some(module) = modules.get_mut(module_id) else {
				continue;
			};

			let module_store = transaction.object_store_of::<Module>()?;
			module.installed = false;
			module_store.put_record(module).await?;

			let entry_store = transaction.object_store_of::<Entry>()?;
			let idx_module_system = entry_store.index_of::<ModuleSystem>();
			let idx_module_system = idx_module_system.map_err(database::Error::from)?;
			for system in &module.systems {
				let query = ModuleSystem { module: module.id.to_string(), system: system.clone() };
				let cursor = idx_module_system.open_cursor(Some(&query)).await;
				let mut cursor = cursor.map_err(database::Error::from)?;
				let mut removed_any = false;
				while let Some(entry) = cursor.next().await {
					entry_store.delete_record(entry.id).await?;
					removed_any = true;
				}
				if removed_any {
					systems_changed.insert(system.clone());
				}
			}
		}
		transaction.commit().await.map_err(database::Error::from)?;
	}

	// Install desired modules
	if !modules_to_install.is_empty() {
		struct ModuleUpdate {
			module_id: ModuleId,
			files: Vec<ModuleFileUpdate>,
		}

		status.push_stage("Installing modules", None);

		let mut module_updates = Vec::with_capacity(modules_to_install.len());
		status.push_stage("Gathering updates", Some(modules_to_install.len()));
		for module_id in modules_to_install {
			status.increment_progress();

			let ModuleId::Github { user_org, repository } = &module_id else {
				// ERROR: Invalid module id to scan
				continue;
			};
			let Some(module) = modules.get_mut(&module_id) else {
				continue;
			};

			// For prev uninstalled modules, scan the remote for all files at the latest state.
			if !module.installed {
				module.installed = true;

				let scan = ScanRepository {
					status: status.clone(),
					client: storage.clone(),
					owner: user_org.clone(),
					name: repository.clone(),
					tree_id: None,
				};
				let files = scan.run().await?;
				let files = files
					.into_iter()
					.map(|file| ModuleFileUpdate { file, status: github::ChangedFileStatus::Added })
					.collect();

				module_updates.push(ModuleUpdate { module_id, files });
			}
			// For module updates, ask repo for changed files since current version.
			else if module.version != module.remote_version {
				let scan = FindFileUpdates {
					client: storage.clone(),
					owner: user_org.clone(),
					name: repository.clone(),
					old_version: module.version.clone(),
					new_version: module.remote_version.clone(),
				};
				module.version = module.remote_version.clone();

				let files = scan.run().await?;
				module_updates.push(ModuleUpdate { module_id, files });
			}
		}
		status.pop_stage(); // Gathering Updates

		// For all files to fetch, across all modules, fetch each file and update progress.
		// Iterate per module so updates can be committed to database as each is fetched.
		status.push_stage("Downloading Modules", Some(module_updates.len()));
		for ModuleUpdate { module_id, files } in module_updates {
			use database::{ObjectStoreExt, TransactionExt};

			status.increment_progress();

			let Some(module) = modules.get(&module_id) else {
				continue;
			};

			// Download all changed files (additions, changes, removals, etc)
			let download = DownloadFileUpdates {
				status: status.clone(),
				client: storage.clone(),
				module_id: module.id.clone(),
				version: module.remote_version.clone(),
				files,
			};
			let (files_to_parse, removed_file_ids) = download.run().await?;

			// Parse downloaded content into records
			let parse_files = ParseFiles {
				status: status.clone(),
				system_depot: system_depot.clone(),
				module_id: module.id.clone(),
				version: module.remote_version.clone(),
				files: files_to_parse,
			};
			let RecordsToUpdate { entries, user_settings } = parse_files.run().await?;

			status.push_stage(format!("Installing {}", module.id.to_string()), None);
			{
				let transaction = database.write()?;

				// Update module data
				let module_store = transaction.object_store_of::<Module>()?;
				module_store.put_record(module).await?;

				// Update new or changed records
				let user_settings_store = transaction.object_store_of::<UserSettingsRecord>();
				let user_settings_store = user_settings_store.context("UserSettingsRecord store")?;
				for record in user_settings {
					user_settings_store.put_record(&record).await?;
				}

				let entry_store = transaction.object_store_of::<Entry>();
				let entry_store = entry_store.context("Entry store")?;
				for record in entries {
					systems_changed.insert(record.system.clone());
					entry_store.put_record(&record).await?;
				}

				// Delete entries by module and file-id
				let entry_ids_to_remove = {
					// Convert removed_file_ids (a list of storage file ids) to the list of database ids
					let idx_module = entry_store.index_of::<crate::database::entry::Module>();
					let idx_module = idx_module.map_err(database::Error::from)?;
					let index = crate::database::entry::Module { module: module.id.to_string() };
					let cursor = idx_module.open_cursor(Some(&index)).await;
					let mut cursor = cursor.map_err(database::Error::from)?;
					let mut entry_ids_to_remove = Vec::with_capacity(removed_file_ids.len());
					// Find the database id of any entry with a storage file id matching any entry in removed_file_ids
					while let Some(entry) = cursor.next().await {
						let Some(file_id) = &entry.file_id else {
							continue;
						};
						if removed_file_ids.contains(file_id) {
							entry_ids_to_remove.push((entry.id.clone(), entry.system.clone()));
						}
					}
					entry_ids_to_remove
				};
				for (entry_id, system) in entry_ids_to_remove {
					systems_changed.insert(system);
					entry_store.delete_record(entry_id).await?;
				}

				transaction.commit().await?;
			}
			status.pop_stage(); // Installing owner/repo
		}
		status.pop_stage(); // Downloading Files
		status.pop_stage(); // Installing Modules
	}

	// Run generators on each system which had content changed
	if !systems_changed.is_empty() {
		use database::{ObjectStoreExt, TransactionExt};

		status.push_stage("Generating Content Variants", Some(systems_changed.len()));
		for system in &systems_changed {
			let system_reg = system_depot.get(&system);
			let system_reg = system_reg.ok_or(StorageSyncError::Database(database::Error::Internal(format!(
				"Missing system registry for {system}"
			))))?;

			status.push_stage(format!("Generating Variants for {system}"), Some(0));

			let transaction = database.read()?;
			let mut queue = gather_generators(system, &transaction, system_reg.node()).await?;
			let mut variant_cache = gather_variants(system, &transaction).await?;
			drop(transaction);

			status.set_progress_max(queue.len());

			// Process each generator in sequence, based on its priority in the queue.
			log::debug!(target: "autosync", "processing generators");
			while let Some(generator) = queue.dequeue() {
				//log::debug!(target: "autosync", "processing generator {}", generator.source_id().unversioned());
				let args = generator::Args { system_registry: system_depot, system: system_reg, database };
				let mut object_list = match generator.execute(args).await {
					Ok(out) => out,
					Err(err) => {
						log::error!(target: "generator", "Failed to execute generator {}: {err:?}", generator.source_id().unversioned());
						continue;
					}
				};

				let mut count_added_to_queue = 0;
				for generator in object_list.drain_generators() {
					//log::debug!(target: "autosync", "enqueuing generator {}", generator.source_id().unversioned());
					queue.enqueue(generator);
					count_added_to_queue += 1;
				}
				if count_added_to_queue > 0 {
					let prev_progress_max = status.progress_max();
					let prev_progress_max =
						prev_progress_max.expect("stage cannot have none progress, it was initialized with some");
					status.set_progress_max(prev_progress_max + count_added_to_queue);
				}

				for variant in object_list.drain_variants() {
					//log::debug!(target: "autosync", "found variant {}", variant.source_id(false));
					variant_cache.insert_update(variant);
				}

				status.increment_progress();
			}

			// Since variants are not valid sources for generators based on database entries,
			// its safe to insert updates into the transaction AFTER all generators are done.
			// Variants can include generator entries if the generator was a product of another generator.
			{
				log::debug!(target: "autosync", "submitting variant changes to database");
				let transaction = database.write()?;
				let entries = transaction.object_store_of::<Entry>()?;
				// New entries get added to the database
				for new_entry in variant_cache.drain_new() {
					entries.put_record(&new_entry).await?;
				}
				// Updated entries get put in the database
				for updated_entry in variant_cache.drain_updated() {
					entries.put_record(&updated_entry).await?;
				}
				// Stale entries are removed from the database
				for stale_entry in variant_cache.drain_stale() {
					entries.delete_record(&stale_entry.id).await?;
				}
				transaction.commit().await?;
			}

			status.pop_stage(); // Generating Variants for {system}

			status.increment_progress(); // Generating Content Variants progress
		}
		status.pop_stage(); // Generating Content Variants
	}

	Ok(())
}

async fn gather_generators(
	system: &str, transaction: &Transaction, node_reg: Arc<generics::Registry>,
) -> Result<generator::Queue, StorageSyncError> {
	use crate::database::entry::SystemCategory;
	use database::{ObjectStoreExt, TransactionExt};
	use kdlize::NodeId;

	let mut queue = generator::Queue::new(node_reg.clone());

	let entry_store = transaction.object_store_of::<Entry>()?;
	let idx_system_category = entry_store.index_of::<SystemCategory>();
	let idx_system_category = idx_system_category.map_err(database::Error::from)?;

	let query = SystemCategory { system: system.into(), category: generator::Generic::id().into() };
	let cursor = idx_system_category.open_cursor(Some(&query)).await;
	let mut cursor = cursor.map_err(database::Error::from)?;
	while let Some(entry) = cursor.next().await {
		//log::debug!(target: "autosync", "generator entry found {}", entry.id);
		let Some(value) = entry.parse_kdl::<generator::Generic>(node_reg.clone()) else {
			//log::debug!(target: "autosync", "failed to parse generator from {}", entry.id);
			continue;
		};
		queue.enqueue(value);
	}

	Ok(queue)
}

async fn gather_variants(system: &str, transaction: &Transaction) -> Result<generator::VariantCache, StorageSyncError> {
	use crate::database::entry::SystemVariants;
	use database::{ObjectStoreExt, TransactionExt};

	let mut cache = generator::VariantCache::default();

	let entry_store = transaction.object_store_of::<Entry>()?;
	let idx_system_category = entry_store.index_of::<SystemVariants>();
	let idx_system_category = idx_system_category.map_err(database::Error::from)?;

	//log::debug!(target: "autosync", "gathering existing variants");
	let cursor = idx_system_category.open_cursor(Some(&SystemVariants::new(system, true))).await;
	let mut cursor = cursor.map_err(database::Error::from)?;
	while let Some(entry) = cursor.next().await {
		cache.insert_original(entry);
	}

	Ok(cache)
}

pub struct ModuleFile {
	// The path within the module of the file (including game system root).
	pub path_in_repo: String,
	// The file-id sha in the github repo.
	pub file_id: String,
}
pub struct ModuleFileUpdate {
	pub file: ModuleFile,
	pub status: github::ChangedFileStatus,
}
