use crate::{
	auth,
	components::{use_media_query, Nav, NavDisplay, TabContent},
	database::{Database, Entry},
	page::characters::sheet::{CharacterHandle, ViewProps},
	system::{
		dnd5e::{
			components::{
				ability, panel, rest, ArmorClass, ConditionsCard, DefensesCard, HitPointMgmtCard, InitiativeBonus,
				Inspiration, ProfBonus, Proficiencies, SpeedAndSenses,
			},
			data::{character::Persistent, Ability},
		},
		Block, ModuleId, SourceId,
	},
};
use kdlize::NodeId;
use yew::prelude::*;
use yew_router::prelude::use_navigator;
use yewdux::prelude::use_store;

mod header;
pub use header::*;

#[function_component]
pub fn Display(ViewProps { swap_view }: &ViewProps) -> Html {
	let database = use_context::<Database>().unwrap();
	let state = use_context::<CharacterHandle>().unwrap();
	let (auth_status, _dispatch) = use_store::<auth::Status>();
	let character_sync_channel = use_context::<crate::page::characters::sheet::sync::Channel>().unwrap();
	let task_dispatch = use_context::<crate::task::Dispatch>().unwrap();
	let navigator = use_navigator().unwrap();

	let fetch_from_storage = Callback::from({
		let channel = character_sync_channel.clone();
		move |_| {
			channel.try_send_req(());
		}
	});
	let fetch_btn = match state.id().has_path() {
		true => {
			html!(<button class="btn btn-warning btn-xs mx-2" onclick={fetch_from_storage}>{"Force Fetch"}</button>)
		}
		false => html!(),
	};

	let save_to_storage = Callback::from({
		let auth_status = auth_status.clone();
		let task_dispatch = task_dispatch.clone();
		let database = database.clone();
		let navigator = navigator.clone();
		let id = state.id().unversioned();
		move |_| {
			let Some(client) = crate::storage::get(&*auth_status) else {
				log::debug!("no storage client");
				return;
			};

			let mut id = id.clone();
			let is_new = !id.has_path();
			id.path = match is_new {
				false => id.path.clone(),
				true => {
					let id = uuid::Uuid::new_v4();
					let mut buffer = uuid::Uuid::encode_buffer();
					let filename = id.as_hyphenated().encode_lower(&mut buffer);
					std::path::Path::new("character").join(format!("{filename}.kdl"))
				}
			};

			let SourceId { module: Some(ModuleId::Github { user_org, repository }), .. } = &id else {
				log::debug!("non-github source id");
				return;
			};
			let path_in_repo = match &id.system {
				None => id.path.clone(),
				Some(system) => std::path::Path::new(&system).join(&id.path),
			};
			let id_str = id.to_string();
			let route = crate::page::characters::Route::sheet(&id);
			let empty_entry = match is_new {
				false => None,
				true => Some(Entry {
					id: id_str.clone(),
					module: id.module.as_ref().map(ModuleId::to_string).unwrap(),
					system: id.system.clone().unwrap(),
					category: Persistent::id().into(),
					version: None,
					metadata: Default::default(),
					kdl: Default::default(),
					file_id: None,
					generator_id: None,
					generated: 0,
				}),
			};
			let message = format!("Manually save character");
			let metadata = state.persistent().clone().to_metadata();
			let content = {
				let doc = state.export_as_kdl();
				let doc = doc.to_string();
				let doc = doc.replace("\\r", "");
				let doc = doc.replace("\\n", "\n");
				let doc = doc.replace("\\t", "\t");
				let doc = doc.replace("    ", "\t");
				doc
			};
			let repo_org = user_org.clone();
			let repo_name = repository.clone();
			let database = database.clone();
			let navigator = navigator.clone();
			task_dispatch.spawn("Update File", None, async move {
				let mut entry = match empty_entry {
					Some(entry) => entry,
					None => match database.get::<Entry>(id_str).await? {
						Some(entry) => entry,
						None => {
							log::debug!("missing entry");
							return Ok(());
						}
					},
				};
				let args = github::repos::contents::update::Args {
					repo_org: &repo_org,
					repo_name: &repo_name,
					path_in_repo: &path_in_repo,
					commit_message: &message,
					content: &content,
					file_id: entry.file_id.as_ref().map(String::as_str),
					branch: None,
				};
				log::debug!("executing update character request {args:?}");
				let response = client.create_or_update_file(args).await?;
				log::debug!("finished update character request {response:?}");

				let module_version = response.version;
				// put the updated content in the database for the persistent character segment
				entry.kdl = content;
				entry.metadata = metadata;
				// with the updated module version
				entry.version = Some(module_version.clone());
				// and updated storage sha/file id (because it changes every time a change is saved on a file)
				entry.file_id = Some(response.file_id);
				// Commit the module version and entry changes to database
				database
					.mutate(move |transaction| {
						use crate::database::{Entry, Module};
						use database::{ObjectStoreExt, TransactionExt};
						Box::pin(async move {
							let module_store = transaction.object_store_of::<Module>()?;
							let entry_store = transaction.object_store_of::<Entry>()?;

							let module_req = module_store.get_record::<Module>(entry.module.clone());
							let mut module = module_req.await?.unwrap();
							module.version = module_version;
							module_store.put_record(&module).await?;

							entry_store.put_record(&entry).await?;

							Ok(())
						})
					})
					.await?;

				if is_new {
					navigator.push(&route);
				}

				Ok(()) as anyhow::Result<()>
			});
		}
	});

	let is_large_page = use_media_query("(min-width: 1400px)");
	let above_panels_content = html! {<>
		<div class="row m-0" style="--bs-gutter-x: 0;">
			<div class="col-auto col-xxl">
				<div class="d-flex align-items-center justify-content-around" style="height: 100%;">
					{is_large_page.then(|| html!(<ProfBonus />)).unwrap_or_default()}
					<InitiativeBonus />
					<ArmorClass />
					<Inspiration />
				</div>
			</div>
			<div class="col">
				<HitPointMgmtCard />
			</div>
		</div>
		<div class="row m-0" style="--bs-gutter-x: 0;">
			{(!*is_large_page).then(|| html! {
				<div class="col-auto">
					<div class="d-flex align-items-center" style="height: 100%;">
						<ProfBonus />
					</div>
				</div>
			}).unwrap_or_default()}
			<div class="col">
				<DefensesCard />
			</div>
			<div class="col">
				<ConditionsCard />
			</div>
		</div>
	</>};

	html! {
		<div class="container overflow-hidden d-flex flex-column">
			<div class="d-flex border-bottom-theme-muted mt-1 mb-2 px-3 pb-1">
				<Header />
				<div class="ms-auto d-flex flex-column justify-content-center">
					<div class="d-flex align-items-center">
						<rest::Button value={crate::system::dnd5e::data::Rest::Short} />
						<rest::Button value={crate::system::dnd5e::data::Rest::Long} />
						<a class="glyph forge" style="margin-right: 0.3rem;" onclick={swap_view.reform(|_| ())} />
					</div>
					<div class="d-flex align-items-center mt-2">
						<div class="ms-auto" />
						{fetch_btn}
						<button class="btn btn-success btn-xs mx-2" onclick={save_to_storage}>{"Save"}</button>
					</div>
				</div>
			</div>
			<div class="row flex-grow-1" style="--bs-gutter-x: 10px;">
				<div class="col-md-auto" style="max-width: 210px;">

					<div class="row m-0" style="--bs-gutter-x: 0;">
						<div class="col">
							<ability::Score ability={Ability::Strength} />
							<ability::Score ability={Ability::Dexterity} />
							<ability::Score ability={Ability::Constitution} />
						</div>
						<div class="col">
							<ability::Score ability={Ability::Intelligence} />
							<ability::Score ability={Ability::Wisdom} />
							<ability::Score ability={Ability::Charisma} />
						</div>
					</div>

					<ability::SavingThrowContainer />
					<Proficiencies />

				</div>
				<div class="col-md-auto">

					<div class="d-flex justify-content-center">
						<SpeedAndSenses />
					</div>

					<div id="skills-container" class="card" style="min-width: 320px; border-color: var(--theme-frame-color);">
						<div class="card-body" style="padding: 5px;">
							<ability::SkillTable />
						</div>
					</div>

				</div>
				<div class="col d-flex flex-column">
					{above_panels_content}

					<div class="card m-1 flex-grow-1">
						<div class="card-body d-flex flex-column" style="padding: 5px;">
							<Nav root_classes={"onesheet-tabs"} disp={NavDisplay::Tabs} default_tab_id={"actions"}>
								<TabContent id="actions" title={html! {{"Actions"}}}>
									<panel::Actions />
								</TabContent>
								<TabContent id="spells" title={html! {{"Spells"}}}>
									<panel::Spells />
								</TabContent>
								<TabContent id="inventory" title={html! {{"Inventory"}}}>
									<panel::Inventory />
								</TabContent>
								<TabContent id="features" title={html! {{"Features & Traits"}}}>
									<panel::Features />
								</TabContent>
								<TabContent id="description" title={html! {{"Description"}}}>
									<panel::Description />
								</TabContent>
							</Nav>
						</div>
					</div>
				</div>
			</div>
		</div>
	}
}
