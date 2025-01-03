use super::GeneralProp;
use crate::{
	components::{
		context_menu,
		database::{use_query_all_typed, use_typed_fetch_callback, QueryAllArgs, QueryStatus},
		IndirectFetch, ObjectLink, Spinner, Tag, Tags,
	},
	page::characters::sheet::{
		joined::editor::{mutator_list, CollapsableCard},
		CharacterHandle, MutatorImpact,
	},
	system::{
		dnd5e::{
			data::{character::Persistent, Condition, Indirect},
			DnD5e,
		},
		SourceId,
	},
	utility::InputExt,
};
use itertools::Itertools;
use std::{rc::Rc, str::FromStr};
use yew::prelude::*;

fn insert_condition_tag(out: &mut Vec<String>, condition: &Condition) {
	out.push(condition.name.clone());
	for implied in &condition.implied {
		if let Indirect::Custom(condition) = &implied {
			insert_condition_tag(out, condition);
		}
	}
}

#[function_component]
pub fn ConditionsCard() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onclick = context_menu::use_control_action({
		|_, _context| context_menu::Action::open_root("Conditions", html!(<Modal />))
	});
	let mut condition_names = Vec::new();
	for condition in state.persistent().conditions.iter() {
		insert_condition_tag(&mut condition_names, condition);
	}
	html! {
		<div class="card m-1" style="height: 80px;" {onclick}>
			<div class="card-body text-center" style="padding: 5px 5px;">
				<h6 class="card-title mb-1" style="font-size: 0.8rem;">{"Conditions"}</h6>
				<div class="d-flex justify-content-center pe-1" style="overflow: auto; height: 53px;">
					{match condition_names.is_empty() {
						true => html!("None"),
						false => html! {
							<Tags classes={"scroll-content"}>
								{condition_names.into_iter().sorted().map(|name| {
									html!(<Tag>{name}</Tag>)
								}).collect::<Vec<_>>()}
							</Tags>
						},
					}}
				</div>
			</div>
		</div>
	}
}

#[function_component]
fn Modal() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let add_condition_section = {
		use crate::system::System;
		let conditions_handle = use_query_all_typed::<Condition>(
			true,
			Some(QueryAllArgs { system: DnD5e::id().into(), ..Default::default() }),
		);
		let add_condition_by_id = use_typed_fetch_callback(
			"Add Condition".into(),
			state.new_dispatch(Box::new(move |condition: Condition, persistent: &mut Persistent| {
				persistent.conditions.insert(condition);
				MutatorImpact::Recompile
			})),
		);
		let on_add_condition = Callback::from(move |evt: web_sys::Event| {
			let Some(value) = evt.select_value() else {
				return;
			};
			let Ok(source_id) = SourceId::from_str(&value) else {
				return;
			};
			add_condition_by_id.emit(source_id);
		});

		// TODO: This should use a regex-capable search bar, just like searching for an item in the inventory

		let content = match conditions_handle.status() {
			QueryStatus::Pending => html!(<Spinner />),
			QueryStatus::Empty | QueryStatus::Failed(_) => html! {
				<select class="form-select">
					<option value="" selected={true}>{"No conditions available"}</option>
				</select>
			},
			QueryStatus::Success(conditions) => {
				let options = conditions
					.iter()
					.map(|condition| {
						let Some(id) = &condition.id else {
							return html!();
						};
						let id = id.unversioned();
						html! {
							<option
								value={id.to_string()}
								disabled={state.persistent().conditions.contains_id(&id)}
							>
								{condition.name.clone()}
							</option>
						}
					})
					.collect::<Vec<_>>();
				html! {
					<select class="form-select" onchange={on_add_condition}>
						<option value="" selected={true}>{"Pick a Condition..."}</option>
						{options}
					</select>
				}
			}
		};
		html! {
			<div class="input-group mb-3">
				<span class="input-group-text">{"Add a Condition"}</span>
				{content}
			</div>
		}
	};

	let on_remove_condition = Callback::from({
		let state = state.clone();
		move |key| {
			state.dispatch(Box::new(move |persistent: &mut Persistent| {
				persistent.conditions.remove(&key);
				MutatorImpact::Recompile
			}));
		}
	});

	html! {<div class="w-100 h-100 scroll-container-y">
		{add_condition_section}
		<div>
			{state.persistent().conditions.iter_keyed().map(|(key, condition)| {
				let on_remove = on_remove_condition.reform({
					let key = key.clone();
					move |_| key.clone()
				});
				let ref_id = condition.name.replace(" ", "");

				// TODO: Show degrees in body of collapsable card
				html! {
					<CollapsableCard
						id={ref_id}
						header_content={{
							html! {<>
								<span>{condition.name.clone()}</span>
								<button
									type="button" class="btn-close ms-auto" aria-label="Close"
									onclick={on_remove}
								/>
							</>}
						}}
					>
						<ConditionBody value={Rc::new(condition.clone())} />
					</CollapsableCard>
				}
			}).collect::<Vec<_>>()}
		</div>
	</div>}
}

#[function_component]
fn ConditionBody(GeneralProp { value: condition }: &GeneralProp<Rc<Condition>>) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let open_details = context_menu::use_control_action({
		move |condition: Rc<Condition>, context| {
			context_menu::Action::open(
				&context,
				condition.name.clone(),
				html!(<ConditionBody value={condition.clone()} />),
			)
		}
	});

	let mut implications = Vec::with_capacity(condition.implied.len());
	for implied in &condition.implied {
		implications.push(html!(<IndirectFetch<Condition>
			indirect={implied.clone()}
			to_inner={Callback::from({
				let open_details = open_details.clone();
				move |condition: Rc<Condition>| html! {
					<ObjectLink
						title={condition.name.clone()}
						subtitle={"Condition"}
						disabled={false}
						onclick={open_details.reform({
							let condition = condition.clone();
							move |_| condition.clone()
						})}
					/>
				}
			})}
		/>));
	}

	html! {<>
		<div class="text-block">{condition.description.clone()}</div>
		{(!implications.is_empty()).then(|| html! {
			<div class="d-flex flex-row">
				<span class="me-2">{"Implied Conditions:"}</span>
				<div>{implications}</div>
			</div>
		})}
		{mutator_list(&condition.mutators, Some(&state))}
	</>}
}
