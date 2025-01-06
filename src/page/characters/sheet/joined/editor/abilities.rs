use crate::{
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{
		change::ApplyAbilityScores,
		components::{ability, validate_uint_only},
		data::Ability,
	},
	utility::InputExt,
};
use enum_map::{Enum, EnumMap};
use enumset::{EnumSet, EnumSetType};
use itertools::Itertools;
use std::{collections::HashSet, ops::RangeInclusive, str::FromStr};
use yew::prelude::*;

#[function_component]
pub fn AbilitiesTab() -> Html {
	html! {<div class="mx-4 mt-3">
		<AbilityScoreInputRow />
		<span class="hr my-3" />
		<GenerationSection />
		<span class="hr my-3" />
		<AllStatBreakdown />
	</div>}
}

#[function_component]
fn AbilityScoreInputRow() -> Html {
	html! {
		<div class="d-flex justify-content-center">
			{EnumSet::<Ability>::all().into_iter().map(|ability | html! {
				<AbilityScoreInput {ability} />
			}).collect::<Vec<_>>()}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct AbilityScoreInputProps {
	pub ability: Ability,
}

#[function_component]
fn AbilityScoreInput(AbilityScoreInputProps { ability }: &AbilityScoreInputProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onchange = state.dispatch_change({
		let ability = *ability;
		move |evt: web_sys::Event| {
			let value = evt.input_value_t::<u32>()?;
			Some(ApplyAbilityScores([(ability, value)].into()))
		}
	});
	html! {
		<div class="mx-3">
			<div class="text-center">{ability.long_name()}</div>
			<input
				type="number" class="form-control text-center mx-auto p-0"
				style="font-size: 26px; font-weight: 500; height: 40px; width: 80px;"
				min="0"
				value={format!("{}", state.persistent().ability_scores[*ability])}
				onkeydown={validate_uint_only()}
				onchange={onchange}
			/>
			<div class="d-flex justify-content-center">
				{"Total: "}
				{*state.ability_scores().get(*ability).score()}
			</div>
		</div>
	}
}

#[derive(Debug, EnumSetType, Enum, PartialOrd, Ord, Hash)]
enum GeneratorMethod {
	PointBuy,
	StandardArray,
}
static PT_BUY_RANGE: RangeInclusive<u32> = 8..=15;
static PT_BUY_BUDGET: u32 = 27;
static STD_ARRAY_OPTIONS: [u32; 6] = [8, 10, 12, 13, 14, 15];
fn get_pt_buy_cost(score: u32) -> u32 {
	match score {
		8..=13 => score - 8,
		14 => 7,
		15 => 9,
		_ => 0,
	}
}
impl GeneratorMethod {
	pub fn display_name(&self) -> &'static str {
		match self {
			Self::PointBuy => "Point Buy",
			Self::StandardArray => "Standard Array",
		}
	}
}
impl ToString for GeneratorMethod {
	fn to_string(&self) -> String {
		match self {
			Self::PointBuy => "PointBuy",
			Self::StandardArray => "StandardArray",
		}
		.to_owned()
	}
}
impl FromStr for GeneratorMethod {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"PointBuy" => Ok(Self::PointBuy),
			"StandardArray" => Ok(Self::StandardArray),
			_ => Err(()),
		}
	}
}

#[function_component]
fn GenerationSection() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let method = use_state_eq(|| None);
	let scores = use_state_eq(|| EnumMap::<Ability, u32>::default());
	let onchange = Callback::from({
		let method = method.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.select_value() else {
				return;
			};
			method.set(GeneratorMethod::from_str(&value).ok());
		}
	});
	let apply_scores = state.dispatch_change({
		let scores = scores.clone();
		move |_| Some(ApplyAbilityScores(scores.iter().map(|(a, v)| (a, *v)).collect()))
	});
	use_effect_with((*method).clone(), {
		let scores = scores.clone();
		let state = state.clone();
		move |method: &Option<GeneratorMethod>| match method {
			None => {}
			Some(GeneratorMethod::PointBuy) => {
				scores.set({
					let mut scores = state.persistent().ability_scores.clone();
					let mut budget_left = PT_BUY_BUDGET;
					for score in scores.values_mut() {
						let cost = get_pt_buy_cost(*score);
						if !PT_BUY_RANGE.contains(score) || budget_left < cost {
							*score = 8;
						} else {
							budget_left = budget_left.saturating_sub(cost);
						}
					}
					scores
				});
			}
			Some(GeneratorMethod::StandardArray) => scores.set({
				let mut scores = state.persistent().ability_scores.clone();
				let mut unused_options = HashSet::from(STD_ARRAY_OPTIONS);
				for score in scores.values_mut() {
					match unused_options.contains(score) {
						true => {
							unused_options.remove(score);
						}
						false => {
							*score = 0;
						}
					}
				}
				scores
			}),
		}
	});
	let has_changes = method.is_some() && *scores != state.persistent().ability_scores;
	html! {
		<div>
			<div class="input-group mb-3 w-50 mx-auto">
				<label class="input-group-text" for="generatorMethod">{"Select Generator"}</label>
				<select class="form-select" id="generatorMethod" onchange={onchange}>
					<option
						value=""
						selected={method.is_none()}
					>{"Choose Method..."}</option>
					{EnumSet::<GeneratorMethod>::all().into_iter().sorted().map(|item| html! {
						<option
							value={item.to_string()}
							selected={*method == Some(item)}
						>{item.display_name()}</option>
					}).collect::<Vec<_>>()}
				</select>
				<button
					type="button"
					class={{
						let mut classes = classes!("btn");
						classes.push(match has_changes {
							true => "btn-success",
							false => "btn-secondary",
						});
						classes
					}}
					onclick={apply_scores}
					disabled={!has_changes}
				>{"Apply Changes"}</button>
			</div>
			{match *method {
				None => html! {},
				Some(GeneratorMethod::PointBuy) => html! {<PointBuy ability_scores={scores.clone()} />},
				Some(GeneratorMethod::StandardArray) => html! {<StandardArray ability_scores={scores.clone()} />},
			}}
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct GeneratorMethodProps {
	ability_scores: UseStateHandle<EnumMap<Ability, u32>>,
}

#[function_component]
fn PointBuy(GeneratorMethodProps { ability_scores }: &GeneratorMethodProps) -> Html {
	let set_score = Callback::from({
		let scores = ability_scores.clone();
		move |(ability, value)| {
			scores.set({
				let mut scores = (*scores).clone();
				scores[ability] = value;
				scores
			});
		}
	});
	let parse_u32 = Callback::from(|evt: web_sys::Event| {
		let Some(value) = evt.select_value_t::<u32>() else {
			return None;
		};
		Some(value)
	});
	let onchange = Callback::from({
		let parse_u32 = parse_u32.clone();
		let set_score = set_score.clone();
		move |(evt, ability)| {
			if let Some(value) = parse_u32.emit(evt) {
				set_score.emit((ability, value));
			}
		}
	});
	let used_points = ability_scores.iter().map(|(_, score)| get_pt_buy_cost(*score)).sum::<u32>();
	let points_remaining = PT_BUY_BUDGET - used_points;
	html! {<>
		<div class="text-center">
			<div>
				<h6 class="m-0">{"Points Remaining"}</h6>
				<span style="font-size: 30px; font-weight: 500px;">
					{points_remaining}
					{" / "}
					{PT_BUY_BUDGET}
				</span>
			</div>
		</div>
		<div class="d-flex justify-content-center">
			{EnumSet::<Ability>::all().into_iter().map(|ability| html! {
				<div class="mx-3">
					<div class="text-center">{ability.long_name()}</div>
					<select
						class="form-select"
						style="width: 80px;"
						onchange={onchange.reform(move |evt| (evt, ability))}
					>
						{PT_BUY_RANGE.clone().into_iter().map(|score| {
							let additional_cost = get_pt_buy_cost(score).saturating_sub(
								get_pt_buy_cost(ability_scores[ability])
							);
							html! {
								<option
									value={format!("{score}")}
									selected={ability_scores[ability] == score}
									disabled={additional_cost > points_remaining}
								>{format!("{score}")}</option>
							}
						}).collect::<Vec<_>>()}
					</select>
				</div>
			}).collect::<Vec<_>>()}
		</div>
	</>}
}

#[function_component]
fn StandardArray(GeneratorMethodProps { ability_scores }: &GeneratorMethodProps) -> Html {
	let set_score = Callback::from({
		let scores = ability_scores.clone();
		move |(ability, value)| {
			scores.set({
				let mut scores = (*scores).clone();
				scores[ability] = value;
				scores
			});
		}
	});
	let parse_u32 = Callback::from(|evt: web_sys::Event| evt.select_value_t::<u32>());
	let onchange = Callback::from({
		let parse_u32 = parse_u32.clone();
		let set_score = set_score.clone();
		move |(evt, ability)| {
			if let Some(value) = parse_u32.emit(evt) {
				set_score.emit((ability, value));
			}
		}
	});
	let remaining_options = HashSet::from(STD_ARRAY_OPTIONS)
		.difference(&ability_scores.values().cloned().collect())
		.sorted()
		.cloned()
		.collect::<HashSet<_>>();
	html! {<>
		<div class="d-flex justify-content-center">
			{EnumSet::<Ability>::all().into_iter().map(|ability| html! {
				<div class="mx-3">
					<div class="text-center">{ability.long_name()}</div>
					<select
						class="form-select"
						style="width: 80px;"
						onchange={onchange.reform(move |evt| (evt, ability))}
					>
						<option
							value={"0"}
							selected={ability_scores[ability] == 0}
						>{"--"}</option>
						{STD_ARRAY_OPTIONS.clone().into_iter().map(|score| {
							html! {
								<option
									value={format!("{score}")}
									selected={ability_scores[ability] == score}
									disabled={!remaining_options.contains(&score)}
								>{format!("{score}")}</option>
							}
						}).collect::<Vec<_>>()}
					</select>
				</div>
			}).collect::<Vec<_>>()}
		</div>
	</>}
}

#[function_component]
fn AllStatBreakdown() -> Html {
	let col_card = |ability: Ability| {
		html! {
			<div class="col">
				<div class="card">
					<div class="card-header">{ability.long_name()}</div>
					<div class="card-body">
						<ability::ScoreBreakdown ability={ability} />
					</div>
				</div>
			</div>
		}
	};
	html! {<>
		<div class="row gx-2 mb-2">
			{col_card(Ability::Strength)}
			{col_card(Ability::Dexterity)}
			{col_card(Ability::Constitution)}
		</div>
		<div class="row gx-2">
			{col_card(Ability::Intelligence)}
			{col_card(Ability::Wisdom)}
			{col_card(Ability::Charisma)}
		</div>
	</>}
}
