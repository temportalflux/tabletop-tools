use crate::{components::auth, page, storage::autosync, theme};
use yew::prelude::*;
use yew_router::prelude::*;

#[function_component]
pub fn App() -> Html {
	let autosync_channel = use_context::<autosync::Channel>().unwrap();
	auth::use_on_auth_success(move |_auth_status| {
		autosync_channel.try_send_req(autosync::Request::FetchLatestVersionAllModules);
	});

	html! {
		<BrowserRouter>
			<Header />
			<Switch<Route> render={Route::switch} />
		</BrowserRouter>
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct AutosyncStatusProps {
	pub value: autosync::Status,
}
#[function_component]
pub fn AutosyncStatusDisplay(AutosyncStatusProps { value }: &AutosyncStatusProps) -> Html {
	html! {
		<div class="sync-status d-flex flex-column justify-content-start align-items-center">
			{value.stages().iter().enumerate().map(|(idx, stage)| {
				html! {
					<div class="stage w-100 d-flex flex-column">
						<div class="d-flex justify-content-start align-items-center">
							{(idx == 0).then_some(html!(<div class="spinner-border me-2" role="status" />))}
							<div class="title">{&stage.title}</div>
						</div>
						{stage.progress.as_ref().map(|status| {
							let progress = (status.progress as f64 / status.max as f64) * 100f64;
							html! {
								<div>
									<div class="progress" role="progressbar">
										<div class="progress-bar bg-success" style={format!("width: {progress}%")} />
									</div>
									<div class="progress-label-float">
										{status.progress} {"/"} {status.max}
									</div>
								</div>
							}
						})}
					</div>
				}
			}).collect::<Vec<_>>()}
		</div>
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Routable)]
pub enum Route {
	#[at("/")]
	Home,
	#[at("/modules")]
	Modules,
	#[at("/characters")]
	Characters,
	#[at("/characters/*")]
	CharacterSheets,
	#[not_found]
	#[at("/404")]
	NotFound,
}

impl Route {
	pub fn not_found() -> Html {
		html!(<Redirect<Self> to={Self::NotFound} />)
	}

	fn switch(self) -> Html {
		match self {
			Self::Home => html!(<page::Home />),
			Self::Modules => html!(<page::ModulesLanding />),
			Self::Characters | Self::CharacterSheets => html!(<page::characters::Switch />),
			Self::NotFound => html!(<page::NotFound />),
		}
	}
}

#[function_component]
fn Header() -> Html {
	let auth_status = yewdux::use_store_value::<crate::auth::Status>();
	let autosync_status = use_context::<autosync::Status>().unwrap();
	let is_authenticated = matches!(*auth_status, crate::auth::Status::Successful { .. });

	let cls_disabled = (autosync_status.is_active() || !is_authenticated).then_some("disabled");
	let auth_content = html!(<auth::LoginButton />);
	html! {
		<header>
			<nav class="navbar navbar-expand-lg sticky-top bg-body-tertiary">
				<div class="container-fluid">
					<Link<Route> classes={classes!("navbar-brand", cls_disabled)} to={Route::Home}>{"Integro Tabletop"}</Link<Route>>
					<button
						class="navbar-toggler" type="button"
						data-bs-toggle="collapse" data-bs-target="#navContent"
						aria-controls="navContent" aria-expanded="false" aria-label="Toggle navigation"
					>
						<span class="navbar-toggler-icon"></span>
					</button>
					<div class="collapse navbar-collapse" id="navContent">
						<ul class="navbar-nav">
							<li class="nav-item">
								<Link<Route>
									classes={classes!("nav-link", cls_disabled)}
									to={Route::Characters}
								>{"My Characters"}</Link<Route>>
							</li>
							<li class="nav-item">
								<Link<Route>
									classes={classes!("nav-link", cls_disabled)}
									to={Route::Modules}
								>{"Modules"}</Link<Route>>
							</li>
						</ul>
						<ul class="navbar-nav flex-row flex-wrap ms-md-auto">
							<theme::Dropdown />
							{auth_content}
						</ul>
					</div>
				</div>
			</nav>
		</header>
	}
}
