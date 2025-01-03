use crate::{
	auth,
	components::{context_menu, use_media_query, Nav, NavDisplay, TabContent},
	page::characters::sheet::{CharacterHandle, ViewProps},
	system::{
		change,
		dnd5e::{
			components::{
				ability, panel, rest, ArmorClass, ConditionsCard, DefensesCard, GeneralProp, HitPointMgmtCard,
				InitiativeBonus, Inspiration, ProfBonus, Proficiencies, SpeedAndSenses,
			},
			data::{character::Character, Ability},
		},
		ModuleId,
	},
};
use yew::prelude::*;
use yew_router::prelude::use_navigator;
use yewdux::prelude::use_store;

mod header;
pub use header::*;

#[function_component]
pub fn Display(ViewProps { swap_view }: &ViewProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let (auth_status, _dispatch) = use_store::<auth::Status>();
	let character_sync_channel = use_context::<crate::page::characters::sheet::sync::Channel>().unwrap();
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
		let navigator = navigator.clone();
		let state = state.clone();
		move |_| {
			if let Some(client) = crate::storage::get(&*auth_status) {
				state.save_to_storage(client, navigator.clone());
			}
		}
	});
	let open_changelog = context_menu::use_control_action({
		|_, _context| context_menu::Action::open_root(format!("Changelog"), html!(<Changelog />))
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
					<div class={classes!("d-flex", "align-items-center", "mt-2", (!state.is_loaded()).then_some("disabled"))}>
						<div class="ms-auto" />
						{fetch_btn}
						<button class="btn btn-success btn-xs mx-2" onclick={save_to_storage}>{"Save"}</button>
						<button class="btn btn-theme btn-xs mx-2" onclick={open_changelog}><i class="bi bi-list-ul" /></button>
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

#[derive(Clone, Debug, PartialEq)]
struct CharacterCommit {
	commit: Option<github::queries::file_commit_history::Commit>,
	changes: Vec<change::Generic<Character>>,
}

#[function_component]
fn Changelog() -> Html {
	let Some(state) = use_context::<CharacterHandle>() else { return html!() };
	let Some(task_dispatch) = use_context::<crate::task::Dispatch>() else { return html!() };
	let Some(system_depot) = use_context::<crate::system::Registry>() else { return html!() };
	let (auth_status, _dispatch) = use_store::<auth::Status>();
	let changelist_state = use_state(|| (Vec::<CharacterCommit>::new(), github::Cursor::Start));
	if !state.is_loaded() {
		return html!();
	}

	let fetch_next_batch = Callback::from({
		let auth_status = auth_status.clone();
		let task_dispatch = task_dispatch.clone();
		let system_depot = system_depot.clone();
		let id = std::sync::Arc::new(state.id().unversioned());
		let changelist_state = changelist_state.clone();
		move |_| {
			let Some(storage) = crate::storage::get(&*auth_status) else { return };
			let Some(file_path) = id.storage_path().to_str().map(str::to_owned) else { return };

			let Some(node_context) = system_depot.make_node_context(id.clone()) else { return };
			let id = id.clone();
			let changelist_state = changelist_state.clone();
			task_dispatch.spawn("Fetch Changelog", None, async move {
				let Some(ModuleId::Github { user_org, repository }) = &id.module else { return Ok(()) };

				// Fetch a quantity of commits from the current cursor (default is the start/latest commit).
				// Commits are returned in order of newest to oldest.
				let (found_commits, cursor) = storage
					.query_file_history(github::QueryFileHistoryParams {
						owner: user_org.into(),
						repository: repository.into(),
						ref_name: "main".into(), // TODO: maybe get this by querying the repo?
						file_path,
						cursor: changelist_state.1.clone(),
						page_size: 1,
						max_pages: Some(1),
					})
					.await;

				let mut all_commits = changelist_state.0.clone();
				for commit in found_commits {
					if !commit.message_body.is_empty() {
						use kdlize::ext::DocumentExt2;
						let kdl_doc = commit.message_body.parse::<kdl::KdlDocument>()?;
						// a list of changes that were made in this commit,
						// ordered from newest to oldest
						let changes = kdl_doc.query_all_t::<_, _, anyhow::Error>(&node_context, "scope() > change")?;

						all_commits.push(CharacterCommit { commit: Some(commit), changes });
					}
				}

				changelist_state.set((all_commits, cursor));

				Ok(()) as anyhow::Result<()>
			});
		}
	});

	let mut btn_fetch_classes = classes!("btn", "btn-theme");
	if changelist_state.1 == github::Cursor::End {
		btn_fetch_classes.push("d-none");
	}

	let local_commit = CharacterCommit {
		commit: None,
		// local changes are stored in order of oldest to newest
		changes: state.persistent().changelist().iter().rev().cloned().collect(),
	};

	html!(<div class="changelog">
		<div class="list scroll-container-y">
			<CommitItem value={local_commit} />
			{changelist_state.0.iter()
				// fetched commits are in order of newest to oldest
				.map(|commit| html!(<CommitItem value={commit.clone()} />))
				.collect::<Vec<_>>()}
		</div>
		<button class={btn_fetch_classes} onclick={fetch_next_batch}>{"Fetch More Changes"}</button>
	</div>)
}

#[function_component]
fn CommitItem(GeneralProp { value }: &GeneralProp<CharacterCommit>) -> Html {
	let commit_type = match &value.commit {
		None => "local",
		Some(_) => "remote",
	};
	html!(<div class={classes!("commit", commit_type)}>
		{match &value.commit {
			None => html!{<>
				<span class="name">{"Local Changes"}</span>
			</>},
			Some(commit) => html!{<>
				<span class="name me-2">{&commit.id}</span>
				<span class="date me-2">{"at "}{&commit.commited_date}</span>
			</>},
		}}
		<div class="changes ms-4">
			{value.changes.iter()
				// changes are in order of newest to oldest
				.map(|change| html!(<ChangeItem value={change.clone()} />))
				.collect::<Vec<_>>()}
		</div>
	</div>)
}

#[function_component]
fn ChangeItem(GeneralProp { value }: &GeneralProp<change::Generic<Character>>) -> Html {
	html!(<div>{format!("{value:?}")}</div>)
}
