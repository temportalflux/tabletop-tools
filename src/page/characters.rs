use super::app;
use crate::{
	components::{
		database::{use_query, QueryStatus},
		modal, Spinner,
	},
	database::{entry::EntryInSystemWithType, Database, Entry, Query},
	system::{
		dnd5e::{
			components::GeneralProp,
			data::character::{Persistent, PersistentMetadata},
			DnD5e,
		},
		Block, ModuleId, SourceId, System,
	},
	task::Signal,
	utility::InputExt,
	GeneralError,
};
use database::{ObjectStoreExt, TransactionExt};
use futures_util::FutureExt;
use itertools::Itertools;
use kdlize::{ext::DocumentExt2, NodeId};
use std::{path::Path, rc::Rc};
use yew::prelude::*;
use yew_router::{
	prelude::{use_navigator, Link},
	Routable,
};
use yewdux::prelude::use_store;

pub mod sheet;
use sheet::Sheet;

#[function_component]
pub fn Switch() -> Html {
	html!(<yew_router::Switch<Route> render={Route::switch} />)
}

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
	#[at("/characters")]
	Landing,
	#[at("/characters/:storage/:module/:system")]
	EmptySheet { storage: String, module: String, system: String },
	#[at("/characters/:storage/:module/:system/*path")]
	Sheet { storage: String, module: String, system: String, path: String },
	#[not_found]
	#[at("/characters/404")]
	NotFound,
}

impl Route {
	/// Returns the character sheet route for a character identified by the source id.
	/// If the id doesn't have a module & system, or otherwise could not be parsed, the NotFound route is returned.
	pub fn sheet(id: &SourceId) -> Self {
		let Some(module_id) = &id.module else {
			return Self::NotFound;
		};
		let Some(system) = &id.system else {
			return Self::NotFound;
		};
		let (storage, module) = match module_id {
			ModuleId::Local { name } => ("local", name.clone()),
			ModuleId::Github { user_org, repository } => ("github", format!("{user_org}:{repository}")),
		};
		Self::Sheet { storage: storage.to_owned(), module, system: system.clone(), path: id.path.display().to_string() }
	}

	fn sheet_id(storage: String, module: String, system: String, path: Option<String>) -> Option<SourceId> {
		let module = match storage.as_str() {
			"local" => ModuleId::Local { name: module },
			"github" => {
				let mut parts = module.split(':');
				let Some(user_org) = parts.next() else {
					return None;
				};
				let Some(repo) = parts.next() else {
					return None;
				};
				ModuleId::Github { user_org: user_org.to_owned(), repository: repo.to_owned() }
			}
			_ => return None,
		};
		Some(SourceId {
			module: Some(module),
			system: Some(system),
			path: match path {
				None => std::path::PathBuf::new(),
				Some(path) => std::path::PathBuf::from(path),
			},
			version: None,
			..Default::default()
		})
	}

	fn switch(self) -> Html {
		match self {
			Self::Landing => html!(<CharacterLanding />),
			Self::NotFound => app::Route::not_found(),
			Self::EmptySheet { storage, module, system } => match Self::sheet_id(storage, module, system, None) {
				None => app::Route::not_found(),
				Some(id) => html!(<Sheet value={id} />),
			},
			Self::Sheet { storage, module, system, path } => {
				match Self::sheet_id(storage, module, system, Some(path)) {
					None => app::Route::not_found(),
					Some(id) => html!(<Sheet value={id} />),
				}
			}
		}
	}
}

#[function_component]
pub fn CharacterLanding() -> Html {
	let auth_status = yewdux::use_store_value::<crate::auth::Status>();
	let navigator = use_navigator().unwrap();
	let task_dispatch = use_context::<crate::task::Dispatch>().unwrap();
	let modal_dispatcher = use_context::<modal::Context>().unwrap();

	let on_create = Callback::from({
		let auth_status = auth_status.clone();
		let navigator = navigator.clone();
		let task_dispatch = task_dispatch.clone();
		move |_| {
			let Some(client) = crate::storage::get(&*auth_status) else {
				log::debug!("no storage client");
				return;
			};
			let navigator = navigator.clone();
			task_dispatch.spawn("Prepare Character", None, async move {
				let search_params = github::SearchRepositoriesParams {
					query: github::Query::default()
						.keyed("user", "@me")
						.keyed("repo", crate::storage::USER_HOMEBREW_REPO_NAME)
						.keyed("in", "name"),
					page_size: 1,
				};
				let (_, mut repositories) = client.search_repositories(search_params).await;
				let Some(homebrew_repo) = repositories.pop() else {
					return Ok(());
				};
				let module_id = (&homebrew_repo).into();

				let system = DnD5e::id();
				let source_id_unversioned =
					SourceId { module: Some(module_id), system: Some(system.to_string()), ..Default::default() };

				let route = Route::sheet(&source_id_unversioned);
				navigator.push(&route);
				Ok(()) as anyhow::Result<()>
			});
		}
	});
	let on_delete = modal_dispatcher.callback({
		move |props: ModalDeleteProps| {
			modal::Action::Open(modal::Props {
				centered: true,
				scrollable: true,
				content: html! {<ModalDelete ..props />},
				..Default::default()
			})
		}
	});

	if !matches!(*auth_status, crate::auth::Status::Successful { .. }) {
		return crate::page::app::Route::not_found();
	}

	html! {<>
		<crate::components::modal::GeneralPurpose />
		<div>
			<h3 class="text-center">{"Characters"}</h3>
			<div class="d-flex align-items-center justify-content-center mb-1">
				<button
					class="btn btn-outline-success btn-sm ms-2"
					onclick={on_create}
				>
					{"Open New Character"}
				</button>
			</div>
			<CharacterList value={on_delete} />
		</div>
	</>}
}

#[function_component]
fn CharacterList(GeneralProp { value: on_delete }: &GeneralProp<Callback<ModalDeleteProps>>) -> Html {
	let character_entries = use_query(Some(()), |database, _| {
		async move {
			let index = EntryInSystemWithType::new::<Persistent>(DnD5e::id());
			let query = Query::subset(&database, Some(index)).await?;
			Ok(query.collect::<Vec<_>>().await) as Result<Vec<Entry>, database::Error>
		}
		.boxed_local()
	});
	let on_delete_clicked = Callback::from({
		let character_entries = character_entries.clone();
		move |_| {
			character_entries.run(());
		}
	});

	match character_entries.status() {
		QueryStatus::Pending => html!(<Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No characters"),
		QueryStatus::Success(entries) => {
			let mut cards = Vec::with_capacity(entries.len());
			for entry in entries {
				let Ok(metadata) = serde_json::from_value::<PersistentMetadata>(entry.metadata.clone()) else {
					continue;
				};
				let id = entry.source_id(false);
				let route = Route::sheet(&id);
				let on_delete = match &entry.file_id {
					None => Callback::from(|_| {}),
					Some(file_id) => on_delete.reform({
						let id = id.clone();
						let file_id = file_id.clone();
						let on_delete_clicked = on_delete_clicked.clone();
						move |_| ModalDeleteProps {
							id: id.clone(),
							file_id: file_id.clone(),
							on_click: on_delete_clicked.clone(),
						}
					}),
				};
				cards.push(html!(<CharacterCard
					{id} {route}
					metadata={Rc::new(metadata)}
					on_delete={on_delete}
				/>));
			}
			html! {
				<div class="d-flex align-items-center justify-content-center flex-wrap">
					{cards}
				</div>
			}
		}
	}
}

#[derive(Clone, PartialEq, Properties)]
struct CharacterCardProps {
	id: SourceId,
	route: Route,
	metadata: Rc<PersistentMetadata>,
	on_delete: Callback<()>,
}
#[function_component]
fn CharacterCard(CharacterCardProps { id: _, route, metadata, on_delete }: &CharacterCardProps) -> Html {
	html! {
		<div class="card m-1" style="min-width: 300px;">
			<div class="card-header d-flex align-items-center">
				<span>{&metadata.name}</span>
				<i
					class="bi bi-trash ms-auto"
					style="color: var(--bs-danger);"
					onclick={on_delete.reform(|_| ())}
				/>
			</div>
			<div class="card-body">
				{match metadata.pronouns.is_empty() {
					true => html!(),
					false => html!(<div>{metadata.pronouns.join(", ")}</div>),
				}}
				<div>
					{"Level "}{metadata.level}{": "}
					{metadata.classes.iter().map(|class_name| html! {
						<span>
							{class_name}
						</span>
					}).collect::<Vec<_>>()}
				</div>
				<div>
					{metadata.bundles.iter_all()
						.filter(|(category, _)| {
							vec!["Race", "Race Variant", "Lineage", "Upbringing"].contains(&category.as_str())
						})
						.sorted_by_key(|(category, _)| *category)
						.map(|(_category, items)| items.clone())
						.flatten()
						.collect::<Vec<_>>()
						.join(", ")
					}
				</div>
				<div class="d-flex justify-content-center mt-2">
					<Link<Route>
						to={route.clone()}
						classes="btn btn-success btn-sm me-2"
					>
						{"Open"}
					</Link<Route>>
				</div>
			</div>
		</div>
	}
}

#[function_component]
fn ModalCreate() -> Html {
	let (auth_status, _dispatch) = use_store::<crate::auth::Status>();
	let task_dispatch = use_context::<crate::task::Dispatch>().unwrap();
	let modal_dispatcher = use_context::<modal::Context>().unwrap();
	let database = use_context::<Database>().unwrap();

	// TODO: Select the parent module
	// TODO: Select game system

	let filename = use_state_eq(|| "unnamed".to_owned());
	let on_change_filename = Callback::from({
		let filename = filename.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.input_value() else {
				return;
			};
			filename.set(value);
		}
	});

	let action_in_progress = use_state(|| Signal::new(false));
	let onclick = Callback::from({
		let auth_status = auth_status.clone();
		let task_dispatch = task_dispatch.clone();
		let database = database.clone();
		let filename = filename.clone();
		let action_in_progress = action_in_progress.clone();
		let close_modal = modal_dispatcher.callback(|_: ()| modal::Action::Close);
		move |_| {
			if action_in_progress.value() {
				return;
			}
			let Some(client) = crate::storage::get(&*auth_status) else {
				log::debug!("no storage client");
				return;
			};
			let id_path = Path::new("character").join(format!("{}.kdl", filename.as_str()));
			let database = database.clone();
			let close_modal = close_modal.clone();
			let signal = task_dispatch.spawn("Create Character File", None, async move {
				let search_params = github::SearchRepositoriesParams {
					query: github::Query::default()
						.keyed("user", "@me")
						.keyed("repo", crate::storage::USER_HOMEBREW_REPO_NAME)
						.keyed("in", "name"),
					page_size: 1,
				};
				let (_, mut repositories) = client.search_repositories(search_params).await;
				let Some(homebrew_repo) = repositories.pop() else {
					return Ok(());
				};
				let module_id = ModuleId::from(&homebrew_repo);

				// NOTE: Cannot continue if our local version is not the latest version in storage (github).
				// We need to ensure that all files from the local ddb module are on the correct version,
				// so we aren't accidentally ahead for some files and not for others.
				// e.g. creating a new file without having latest means our module either
				// has an old version and a new file locally,
				// or a new version and missing updates between current local and the new version.
				let local_module_version = match database.get::<crate::database::Module>(module_id.to_string()).await {
					Ok(Some(local_module)) => local_module.version,
					_ => return Ok(()),
				};
				if local_module_version != homebrew_repo.version {
					return Ok(());
				}

				let message = "Add new character";
				let state = Persistent::default();
				let content = state.export_as_kdl().to_string_unescaped();

				let module_id_str = module_id.to_string();
				let system = DnD5e::id();
				let path_in_repo = Path::new(system).join(&id_path);
				let source_id_unversioned = SourceId {
					module: Some(module_id),
					system: Some(system.to_string()),
					path: id_path,
					..Default::default()
				};
				let metadata = match state.clone().to_metadata() {
					serde_json::Value::Null => serde_json::Value::Null,
					serde_json::Value::Object(mut metadata) => {
						metadata.insert("module".into(), serde_json::json!(&module_id_str));
						serde_json::Value::Object(metadata)
					}
					other => {
						return Err(GeneralError(format!(
							"Metadata must be a map, but {} returned {:?}.",
							source_id_unversioned.to_string(),
							other
						))
						.into());
					}
				};

				let args = github::repos::contents::update::Args {
					repo_org: &homebrew_repo.owner,
					repo_name: &homebrew_repo.name,
					path_in_repo: &path_in_repo,
					commit_message: &message,
					content: &content,
					file_id: None,
					branch: None,
				};
				let response = client.create_or_update_file(args).await?;
				let updated_version = response.version;

				let record = crate::database::Entry {
					id: source_id_unversioned.to_string(),
					module: module_id_str.clone(),
					system: system.to_string(),
					category: state.get_id().to_owned(), // KdlNode::get_id
					version: Some(updated_version.clone()),
					metadata,
					kdl: content.clone(),
					file_id: Some(response.file_id),
					generator_id: None,
					generated: 0,
				};
				if let Err(err) = database
					.mutate(move |transaction| {
						use crate::database::{Entry, Module};
						let module_id_str = module_id_str.clone();
						Box::pin(async move {
							// Update module version in database for the submitted change
							let module_store = transaction.object_store_of::<Module>()?;
							let module_req = module_store.get_record::<Module>(module_id_str);
							let mut module = module_req.await?.unwrap();
							module.version = updated_version;
							module_store.put_record(&module).await?;
							// Insert the character record
							let entry_store = transaction.object_store_of::<Entry>()?;
							entry_store.add_record(&record).await?;
							Ok(())
						})
					})
					.await
				{
					log::error!("{err:?}");
				}

				close_modal.emit(());

				Ok(()) as anyhow::Result<()>
			});
			action_in_progress.set(signal);
		}
	});

	html! {<>
		<div class="modal-header">
			<h1 class="modal-title fs-4">{"New Character"}</h1>
			<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
		</div>
		<div class="modal-body">
			<div class="mb-3">
				<label class="form-label" for="filename">{"Unique File Name"}</label>
				<input
					class="form-control" id="filename" type="text"
					value={(*filename).clone()}
					onchange={on_change_filename}
				/>
				<div class="form-text">{"This is a unique id used to save the file."}</div>
			</div>
			<div class="d-flex justify-content-center">
				<button class="btn btn-success m-2" {onclick}>{"Create Character"}</button>
			</div>
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ModalDeleteProps {
	id: SourceId,
	file_id: String,
	on_click: Callback<()>,
}
#[function_component]
fn ModalDelete(ModalDeleteProps { id, file_id, on_click }: &ModalDeleteProps) -> Html {
	let (auth_status, _dispatch) = use_store::<crate::auth::Status>();
	let task_dispatch = use_context::<crate::task::Dispatch>().unwrap();
	let modal_dispatcher = use_context::<modal::Context>().unwrap();
	let database = use_context::<Database>().unwrap();

	// TODO: This could use a confirmation captcha, requiring that the character's name by typed in.
	let action_in_progress = use_state(|| Signal::new(false));
	let onclick = Callback::from({
		let auth_status = auth_status.clone();
		let id = id.clone();
		let file_id = file_id.clone();
		let action_in_progress = action_in_progress.clone();
		let database = database.clone();
		let on_success = on_click.clone();
		let close_modal = modal_dispatcher.callback(|_| modal::Action::Close);
		move |_| {
			if action_in_progress.value() {
				return;
			}

			let Some(ModuleId::Github { user_org: repo_org, repository: repo_name }) = id.module.clone() else {
				return;
			};
			let Some(system) = &id.system else {
				return;
			};
			let path_in_repo = Path::new(system.as_str()).join(&id.path);
			let module_id_str = id.module.as_ref().unwrap().to_string();

			let message = "Delete character";
			let file_id = file_id.clone();
			let Some(client) = crate::storage::get(&*auth_status) else {
				log::debug!("no storage client");
				return;
			};
			let database = database.clone();
			let id_str = id.to_string();
			let close_modal = close_modal.clone();
			let on_success = on_success.clone();
			let signal = task_dispatch.spawn("Delete Character File", None, async move {
				let args = github::repos::contents::delete::Args {
					repo_org: repo_org.as_str(),
					repo_name: repo_name.as_str(),
					path_in_repo: path_in_repo.as_path(),
					commit_message: &message,
					file_id: &file_id,
					branch: None,
				};
				let updated_version = client.delete_file(args).await?;

				if let Err(err) = database
					.mutate(move |transaction| {
						use crate::database::{Entry, Module};
						let module_id_str = module_id_str.clone();
						let id_str = id_str.clone();
						let updated_version = updated_version.clone();
						Box::pin(async move {
							// Update module version in database for the submitted change
							let module_store = transaction.object_store_of::<Module>()?;
							let module_req = module_store.get_record::<Module>(module_id_str);
							let mut module = module_req.await?.unwrap();
							module.version = updated_version;
							module_store.put_record(&module).await?;
							// Insert the character record
							let entry_store = transaction.object_store_of::<Entry>()?;
							entry_store.delete_record(id_str).await?;
							Ok(())
						})
					})
					.await
				{
					log::error!("{err:?}");
				}

				close_modal.emit(());
				on_success.emit(());
				Ok(()) as anyhow::Result<()>
			});
			action_in_progress.set(signal);
		}
	});
	html! {<>
		<div class="modal-header">
			<h1 class="modal-title fs-4">{"Delete Character"}</h1>
			<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" />
		</div>
		<div class="modal-body">
			{"Are you sure you want to delete this character? You wont be able to undo this action \
			within this app (you need to know how to use git to do so)."}
			<div class="d-flex justify-content-center">
				<button class="btn btn-danger m-2" {onclick}>{"Delete Character"}</button>
			</div>
		</div>
	</>}
}
