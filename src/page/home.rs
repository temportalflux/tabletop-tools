use crate::{
	auth::{self, OAuthProvider},
	components::{
		auth::LocalUser,
		database::{use_query, QueryStatus},
		Spinner,
	},
	data::{UserId, UserSettings},
	database::{Database, Module, Query, UserSettingsRecord},
	system::ModuleId,
	utility::InputExt,
};
use database::{ObjectStoreExt, TransactionExt};
use futures_util::{FutureExt, StreamExt};
use kdlize::{AsKdl, NodeId};
use yew::prelude::*;
use yewdux::use_store_value;

#[function_component]
pub fn Home() -> Html {
	let local_user = use_store_value::<LocalUser>();
	html! {<>
		<crate::components::modal::GeneralPurpose />
		<div>
			{"This is the home page!"}
			<div class="d-flex justify-content-center">
				{local_user.homebrew_module_id().map(|module_id| html!(<FriendsList {module_id} />))}
			</div>
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
pub struct FriendsListProps {
	pub module_id: ModuleId,
}

#[function_component]
pub fn FriendsList(FriendsListProps { module_id }: &FriendsListProps) -> Html {
	let auth_status = use_store_value::<auth::Status>();
	let database = use_context::<Database>().unwrap();
	let search_text = use_state_eq(|| String::default());
	let error_text = use_state_eq(|| None::<String>);
	let settings = use_query(Some(UserSettings::homebrew_id(module_id)), |database, settings_id| {
		async move {
			let query = Query::<UserSettingsRecord>::single(&database, &settings_id.to_string()).await?;
			let entry = query.into_inner().next().await;
			Ok(entry.unwrap_or_default()) as Result<UserSettingsRecord, database::Error>
		}
		.boxed_local()
	});

	let oninput = Callback::from({
		let search_text = search_text.clone();
		move |evt: InputEvent| {
			search_text.set(evt.input_value().unwrap_or_default());
		}
	});

	let mutate_list = Callback::from({
		let record_id = UserSettings::homebrew_id(module_id);
		let auth_status = auth_status.clone();
		let database = database.clone();
		move |id_diff: itertools::Either<UserId, UserId>| {
			let record_id = record_id.clone();
			let auth_status = auth_status.clone();
			let database = database.clone();
			Box::pin(async move {
				let Some(client) = crate::storage::get(&*auth_status) else {
					log::debug!("no storage client");
					return Ok(None);
				};

				let mut record = {
					let transaction = database.read()?;
					let store = transaction.object_store_of::<UserSettingsRecord>()?;
					let record_id_str = record_id.to_string();
					let record_request = store.get_record::<UserSettingsRecord>(&record_id_str);
					match record_request.await? {
						Some(record) => record,
						None => {
							let mut record = UserSettingsRecord::default();
							record.id = record_id.to_string();
							record
						}
					}
				};
				match id_diff {
					itertools::Either::Left(user_id) => {
						if record.user_settings.friends.contains(&user_id) {
							return Ok(None);
						}
						record.user_settings.friends.push(user_id);
					}
					itertools::Either::Right(user_id) => {
						record.user_settings.friends.retain(|id| *id != user_id);
					}
				}

				if let Some(ModuleId::Github { user_org, repository }) = &record_id.module {
					let message = format!("Update friends list");
					let content = {
						use kdlize::ext::DocumentExt2;
						let mut doc = kdl::KdlDocument::new();
						doc.nodes_mut().push(record.user_settings.as_kdl().build(UserSettings::id()));
						doc.to_string_unescaped()
					};
					let args = github::repos::contents::update::Args {
						repo_org: user_org,
						repo_name: repository,
						path_in_repo: &record_id.path,
						commit_message: &message,
						content: &content,
						file_id: record.file_id.as_ref().map(String::as_str),
						branch: None,
					};
					let response = client.create_or_update_file(args).await?;
					record.version = response.version;
					record.file_id = Some(response.file_id);
				}

				let transaction = database.write()?;
				{
					let module_store = transaction.object_store_of::<Module>()?;
					let module_id = record_id.module.as_ref().unwrap().to_string();
					let module_req = module_store.get_record::<Module>(&module_id);
					let mut module = module_req.await?.unwrap();
					module.version = record.version.clone();
					module_store.put_record(&module).await?;
				}
				{
					let store = transaction.object_store_of::<UserSettingsRecord>()?;
					store.put_record(&record).await?;
				}
				transaction.commit().await?;

				Ok(Some(record)) as anyhow::Result<_>
			})
		}
	});

	let add_friend = Callback::from({
		let mutate_list = mutate_list.clone();
		let auth_status = auth_status.clone();
		let search_text = search_text.clone();
		let error_text = error_text.clone();
		let settings = settings.clone();
		move |_: ()| {
			let Some(client) = crate::storage::get(&*auth_status) else {
				log::debug!("no storage client");
				return;
			};
			let search_text = search_text.clone();
			let mutate_list = mutate_list.clone();
			let error_text = error_text.clone();
			let settings = settings.clone();
			let pending = async move {
				error_text.set(None);
				let user_id = UserId { provider: OAuthProvider::Github, id: (*search_text).trim().to_owned() };

				if user_id.id.is_empty() {
					return Ok(());
				}

				match client.find_user(user_id.id.clone()).await? {
					github::FindUserResponse::NotFound => {
						error_text.set(Some(format!("No such Github user named \"{}\".", user_id.id)));
						return Ok(());
					}
					github::FindUserResponse::Viewer => {
						search_text.set(String::default());
						return Ok(());
					}
					github::FindUserResponse::Valid => {
						search_text.set(String::default());
					}
				}

				let mutation = mutate_list.emit(itertools::Either::Left(user_id));
				if let Some(record) = mutation.await? {
					settings.update(record);
				}

				Ok(()) as anyhow::Result<()>
			};
			wasm_bindgen_futures::spawn_local(async move {
				if let Err(err) = pending.await {
					log::error!(target: "user_settings", "{err:?}");
				}
			});
		}
	});

	let remove_friend = Callback::from({
		let mutate_list = mutate_list.clone();
		let settings = settings.clone();
		move |user_id: UserId| {
			let settings = settings.clone();
			let mutation = mutate_list.emit(itertools::Either::Right(user_id));
			let pending = async move {
				if let Some(record) = mutation.await? {
					settings.update(record);
				}
				Ok(()) as anyhow::Result<()>
			};
			wasm_bindgen_futures::spawn_local(async move {
				if let Err(err) = pending.await {
					log::error!(target: "user_settings", "{err:?}");
				}
			});
		}
	});

	let mut input_classes = classes!("form-control");
	if error_text.is_some() {
		input_classes.push("is-invalid");
	}

	html! {
		<div class="d-flex flex-column" style="min-width: 500px;">
			<h3 style="text-align: center;">{"Friends"}</h3>
			<div class="input-group">
				<span class="input-group-text">
					<i class="bi bi-search" />
				</span>
				<input
					type="text" class={input_classes}
					placeholder="username" value={(*search_text).clone()} {oninput}
					// onchange={add_friend.reform(|_| ())}
				/>
				<button type="button" class="btn btn-sm btn-outline-success" onclick={add_friend.reform(|_| ())}>
					<i class="bi bi-plus" style="font-size: 20px;" />
				</button>
				<div class="invalid-feedback">
					{error_text.as_ref().map(|text| text.as_str())}
				</div>
			</div>
			<ul class="list-group mt-2">
				{match settings.status() {
					QueryStatus::Success(settings) => {
						let items = settings.user_settings.friends.iter().map(|user_id| {
							html!(<FriendItem user_id={user_id.clone()} on_remove={remove_friend.clone()} />)
						}).collect::<Vec<_>>();
						html!(<>{items}</>)
					}
					QueryStatus::Pending => html!(<Spinner />),
					_ => html!(),
				}}
			</ul>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct FriendItemProps {
	pub user_id: UserId,
	pub on_remove: Callback<UserId>,
}

#[function_component]
pub fn FriendItem(FriendItemProps { user_id, on_remove }: &FriendItemProps) -> Html {
	let on_click_close = on_remove.reform({
		let user_id = user_id.clone();
		move |_: MouseEvent| user_id.clone()
	});
	html! {
		<li class="d-flex flex-row list-group-item">
			<span class="flex-grow-1">{user_id.id.as_str()}</span>
			<button type="button" class="btn-close" aria-label="Close" onclick={on_click_close} />
		</li>
	}
}
