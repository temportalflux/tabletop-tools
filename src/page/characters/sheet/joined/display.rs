use crate::{
	auth,
	components::{use_media_query, Nav, NavDisplay, TabContent},
	page::characters::sheet::{CharacterHandle, ViewProps},
	system::dnd5e::{
		components::{
			ability, panel, rest, ArmorClass, ConditionsCard, DefensesCard, HitPointMgmtCard, InitiativeBonus,
			Inspiration, ProfBonus, Proficiencies, SpeedAndSenses,
		},
		data::Ability,
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
