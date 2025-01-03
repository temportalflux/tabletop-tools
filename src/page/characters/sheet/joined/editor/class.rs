use crate::{
	components::{
		database::{use_query_all_typed, use_typed_fetch_callback, QueryAllArgs, QueryStatus},
		Spinner,
	},
	page::characters::sheet::{joined::editor::mutator_list, CharacterHandle, MutatorImpact},
	system::dnd5e::{
		change,
		data::{character::Persistent, roll::Die, Class, Level},
		DnD5e,
	},
	utility::InputExt,
};
use convert_case::{Case, Casing};
use std::{collections::HashSet, sync::Arc};
use yew::prelude::*;

#[function_component]
pub fn ClassTab() -> Html {
	html! {<div class="mx-4 mt-3">
		<ActiveClassList />
		<BrowserSection />
	</div>}
}

#[function_component]
fn BrowserSection() -> Html {
	let browser_collapse = use_node_ref();
	let is_browser_open = use_state_eq(|| false);
	let toggle_browser = Callback::from({
		let is_browser_open = is_browser_open.clone();
		move |_| {
			is_browser_open.set(!*is_browser_open);
		}
	});
	html! {<>
		<div class="d-flex justify-content-center">
			<ClassBrowerToggle is_open={*is_browser_open} on_click={toggle_browser.clone()} />
		</div>
		<div class="collapse" id="classBrowser" ref={browser_collapse}>
			<ClassBrowser on_added={toggle_browser.clone()} />
		</div>
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct ClassBrowerToggleProps {
	is_open: bool,
	on_click: Callback<()>,
}

#[function_component]
fn ClassBrowerToggle(ClassBrowerToggleProps { is_open, on_click }: &ClassBrowerToggleProps) -> Html {
	let mut classes = classes!("btn");
	classes.push(match *is_open {
		false => "btn-outline-success",
		true => "btn-danger",
	});
	let text = match *is_open {
		true => "Close Class Browser",
		false => "Open Class Browser",
	};
	html! {
		<button
			type="button" class={classes}
			data-bs-toggle="collapse" data-bs-target="#classBrowser"
			onclick={on_click.reform(|_| ())}
		>
			{text}
		</button>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct ClassBrowserProps {
	on_added: Callback<()>,
}

#[function_component]
fn ClassBrowser(ClassBrowserProps { on_added }: &ClassBrowserProps) -> Html {
	use crate::system::System;

	let state = use_context::<CharacterHandle>().unwrap();

	let query_args = QueryAllArgs::<Class> {
		system: DnD5e::id().into(),
		adjust_listings: Some(Arc::new({
			let iter_classes = state.persistent().classes.iter();
			let iter_ids = iter_classes.map(|class| class.id.unversioned());
			let existing_class_ids = iter_ids.collect::<HashSet<_>>();
			move |mut listings| {
				listings.retain(|class| !existing_class_ids.contains(&class.id.unversioned()));
				listings.sort_by(|a, b| a.name.cmp(&b.name));
				listings
			}
		})),
		..Default::default()
	};
	let classes_handle = use_query_all_typed::<Class>(true, Some(query_args));

	let update = use_force_update();
	let on_add_class = use_typed_fetch_callback(
		"Add Class".into(),
		Callback::from({
			let state = state.clone();
			let on_added = on_added.clone();
			let update = update.clone();
			move |mut class_to_add: Class| {
				class_to_add.current_level = 1;
				state.dispatch(Box::new(move |persistent: &mut Persistent| {
					persistent.add_class(class_to_add);
					MutatorImpact::Recompile
				}));
				on_added.emit(());
				update.force_update();
			}
		}),
	);

	let content = match classes_handle.status() {
		QueryStatus::Pending => html!(<Spinner />),
		QueryStatus::Empty | QueryStatus::Failed(_) => html!("No classes available"),
		QueryStatus::Success(classes) => html! {<>
			{classes.iter().map(|class| {
				let id = class.name.to_case(Case::Snake);
				html! {
					<div class="accordion-item">
						<h2 class="accordion-header">
							<button class="accordion-button collapsed" type="button" data-bs-toggle="collapse" data-bs-target={format!("#{id}")}>
								{class.name.clone()}
							</button>
						</h2>
						<div {id} class="accordion-collapse collapse" data-bs-parent={"#all-entries"}>
							<div class="accordion-body">
								<button
									type="button" class="btn btn-success my-1 w-100"
									data-bs-toggle="collapse" data-bs-target="#classBrowser"
									onclick={on_add_class.reform({
										let class_id = class.id.unversioned();
										move |_: MouseEvent| class_id.clone()
									})}
								>{"Add"}</button>
								{class_body(class, None)}
								{class_levels(class, None)}
							</div>
						</div>
					</div>
				}
			}).collect::<Vec<_>>()}
		</>},
	};

	html! {
		<div class="accordion my-2" id="all-entries">
			{content}
		</div>
	}
}

#[function_component]
fn ActiveClassList() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onclick_add = Callback::from({
		let state = state.clone();
		move |idx: usize| {
			state.dispatch(Box::new(move |persistent: &mut Persistent| {
				let Some(class) = persistent.classes.get_mut(idx) else {
					return MutatorImpact::None;
				};
				class.current_level += 1;
				// TODO: Only recompile on exit
				MutatorImpact::Recompile
			}));
		}
	});

	let remove_class = Callback::from({
		let state = state.clone();
		move |idx| {
			state.dispatch(Box::new(move |persistent: &mut Persistent| {
				let _ = persistent.classes.remove(idx);
				// TODO: Only recompile on exit
				MutatorImpact::Recompile
			}));
		}
	});
	let onclick_remove = Callback::from({
		let state = state.clone();
		move |idx: usize| {
			state.dispatch(Box::new(move |persistent: &mut Persistent| {
				let remove_class = {
					let Some(class) = persistent.classes.get_mut(idx) else {
						return MutatorImpact::None;
					};
					class.current_level = class.current_level.saturating_sub(1);
					class.current_level == 0
				};
				if remove_class {
					let _ = persistent.classes.remove(idx);
				}
				// TODO: Only recompile on exit
				MutatorImpact::Recompile
			}));
		}
	});
	html! {<>
		{state.persistent().classes.iter().enumerate().map(|(idx, class)| {
			html! {
				<div class="card my-2">
					<div class="card-header d-flex">
						{class.name.clone()}
						<button
							type="button"
							class="btn-close ms-auto" aria-label="Close"
							onclick={remove_class.reform(move |_| idx)}
						/>
					</div>
					<div class="card-body">
						{class_body(class, Some(&state))}
						<div class="d-flex justify-content-between mt-3">
							<button
								type="button" class="btn btn-success mx-2"
								onclick={onclick_add.reform(move |_| idx)}
							>{"Add Level"}</button>
							<h5>{"Levels"}</h5>
							<button
								type="button" class="btn btn-danger mx-2"
								onclick={onclick_remove.reform(move |_| idx)}
							>{match class.current_level {
								1 => "Remove Class".to_owned(),
								_ => format!("Remove Level {}", class.current_level),
							}}</button>
						</div>
						{class_levels(class, Some(&state))}
					</div>
				</div>
			}
		}).collect::<Vec<_>>()}
	</>}
}

fn class_body(value: &Class, state: Option<&CharacterHandle>) -> Html {
	html! {<>
		<div class="text-block">
			{value.description.clone()}
		</div>
		{mutator_list(&value.mutators, state)}
	</>}
}

fn class_levels(value: &Class, state: Option<&CharacterHandle>) -> Html {
	let class_level_div_id = format!("{}-level", value.name.to_case(Case::Snake));
	let iter_levels = value.iter_levels(state.is_none());
	let iter_levels = iter_levels.filter(|entry| state.is_some() || !entry.level().is_empty());
	html! {
		<div class="my-2">
			{iter_levels.map(|entry| {
				let idx = entry.index();
				let level = entry.level();
				html! {
					<CollapsableCard
						id={format!("{}-{}", class_level_div_id, idx + 1)}
						collapse_btn_classes={level.is_empty().then_some("v-hidden").unwrap_or_default()}
						header_content={{
							html! {<>
								<span>{"Level "}{idx + 1}</span>
								{state.is_some().then(move || html! {
									<LevelHitPoints
										class_name={value.name.clone()}
										level_idx={idx}
										data_path={level.hit_points.get_data_path()}
										die={value.hp_die}
									/>
								}).unwrap_or_default()}
							</>}
						}}
					>
						{level_body(level, state)}
					</CollapsableCard>
				}
			}).collect::<Vec<_>>()}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct CollapsableCardProps {
	pub id: AttrValue,

	#[prop_or_default]
	pub root_classes: Classes,

	#[prop_or_default]
	pub header_classes: Classes,
	#[prop_or_default]
	pub header_content: Html,
	#[prop_or_default]
	pub collapse_btn_classes: Classes,

	#[prop_or_default]
	pub body_classes: Classes,

	#[prop_or_default]
	pub children: Children,
}
#[function_component]
pub fn CollapsableCard(props: &CollapsableCardProps) -> Html {
	let CollapsableCardProps {
		id,
		root_classes,
		header_classes,
		header_content,
		collapse_btn_classes,
		body_classes,
		children,
	} = props;
	static START_SHOWN: bool = false;
	let card_classes = classes!("card", "collapsable", root_classes.clone());
	let header_classes = classes!("card-header", "d-flex", "align-items-center", header_classes.clone());
	let body_classes = classes!("card-body", body_classes.clone());
	let mut collapse_btn_classes = classes!("arrow", "me-2", collapse_btn_classes.clone());
	let mut collapse_div_classes = classes!("collapse");
	match START_SHOWN {
		true => {
			collapse_div_classes.push("show");
		}
		false => {
			collapse_btn_classes.push("collapsed");
		}
	}

	html! {
		<div class={card_classes}>
			<div class={header_classes}>
				<button
					role="button" class={collapse_btn_classes}
					data-bs-toggle="collapse"
					data-bs-target={format!("#{}", id.as_str())}
				/>
				{header_content.clone()}
			</div>
			<div {id} class={collapse_div_classes}>
				<div class={body_classes}>
					{children.clone()}
				</div>
			</div>
		</div>
	}
}

fn level_body(value: &Level, state: Option<&CharacterHandle>) -> Html {
	html! {<>
		{mutator_list(&value.mutators, state)}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct LevelHitPointsProps {
	class_name: AttrValue,
	level_idx: usize,
	data_path: Option<std::path::PathBuf>,
	die: Die,
}
#[function_component]
fn LevelHitPoints(LevelHitPointsProps { class_name, level_idx, data_path, die }: &LevelHitPointsProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(hp_path) = data_path else { return Html::default() };
	let hp_value = state.get_first_selection_at::<u32>(hp_path).map(|res| res.ok()).flatten();
	let mut classes = classes!("form-select", "hit-points", "py-0", "w-auto");
	if hp_value.is_none() {
		classes.push("missing-value");
	}
	let onchange = state.dispatch_change({
		let class_name = class_name.clone();
		let level_idx = *level_idx;
		move |evt: web_sys::Event| {
			let Some(value) = evt.select_value_t::<u32>() else { return None };
			Some(change::hit_points::LevelHP { class_name: class_name.as_str().to_owned(), level_idx, value })
		}
	});
	let info_text = hp_value.is_none().then(|| {
		html! {
			<span class="me-2" style="color: var(--bs-warning);">
				{"Roll your Hit Points!"}
			</span>
		}
	});
	html! {
		<div class="d-inline-flex align-items-center ms-auto">
			{info_text.unwrap_or_default()}
			<span class="glyph heart me-1" />
			<select class={classes} {onchange}>
				<option selected={hp_value == None}></option>
				{(1..=die.value()).map(|value| {
					html! {
						<option
							value={value.to_string()}
							selected={hp_value == Some(value)}
						>
							{value}
						</option>
					}
				}).collect::<Vec<_>>()}
			</select>
		</div>
	}
}
