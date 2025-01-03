use crate::{
	bootstrap::components::Tooltip,
	components::{context_menu, mobile, AnnotatedNumber, Style},
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{
		components::glyph,
		data::{Ability, Skill},
	},
};
use enumset::EnumSet;
use itertools::Itertools;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct ScoreProps {
	pub ability: Ability,
}

#[function_component]
pub fn Score(ScoreProps { ability }: &ScoreProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let screen_size = mobile::use_mobile_kind();

	let ability_score = state.ability_scores().get(*ability);

	let onclick = context_menu::use_control_action({
		let ability = *ability;
		move |_, _context| context_menu::Action::open_root(ability.long_name(), html!(<Modal {ability} />))
	});

	let tooltip = (ability_score.iter_bonuses().count() > 0).then(|| {
		format!(
			"<div class=\"attributed-tooltip\">{}</div>",
			ability_score.iter_bonuses().fold(String::new(), |mut content, (bonus, path, included_in_total)| {
				if *included_in_total {
					let source_text = crate::data::as_feature_path_text(&path).unwrap_or_default();
					content += format!("<span>+{} ({source_text})</span>", bonus.value).as_str();
				}
				content
			})
		)
	});

	let mut ability_modifier = ability_score.score().modifier();
	ability_modifier += {
		let iter = state.skills()[*ability].bonuses().iter();
		let iter = iter.filter(|(_bonus, context, _source)| context.is_none());
		let iter = iter.map(|(bonus, _context, _source)| *bonus);
		iter.sum::<i64>() as i32
	};

	let score_modifier = html!(<AnnotatedNumber value={ability_modifier} show_sign={true} />);

	let roll_modifier = {
		let iter = state.skills()[*ability].modifiers().iter();
		let iter = iter.filter_map(|(modifier, items)| (!items.is_empty()).then_some(modifier));
		let iter = iter.unique();
		iter.fold(None, |acc, modifier| match (acc, modifier) {
			(Some(prev), next) if prev != next => None,
			(_, modifier) => Some(modifier),
		})
	};
	let roll_modifier = roll_modifier.map(|modifier| {
		html! {
			<glyph::RollModifier value={modifier}
				style={Style::default().with("width", "15px").with("height", "15px")}
			/>
		}
	});

	match screen_size {
		mobile::Kind::Desktop => html! {
			<div class="card ability-card m-1" style="border-color: var(--theme-frame-color-muted);">
				<div class="card-body text-center" {onclick}>
					<h6 class="card-title">{ability.long_name()}</h6>
					<div class="primary-stat">
						{score_modifier}
					</div>
					<div class="row" style={Style::from([("--bs-gutter-x", "0"), ("--bs-gutter-y", "0")])}>
						<div class="col" />
						<Tooltip classes={"col-auto secondary-stat"} content={tooltip} use_html={true}>
							{*ability_score.score()}
						</Tooltip>
						<div class="col d-flex align-items-center justify-content-end">
							{roll_modifier}
						</div>
					</div>
				</div>
			</div>
		},
		mobile::Kind::Mobile => {
			let saving_throw_prof = state.saving_throws()[*ability].proficiencies();
			let mut saving_throw_modifier = state.ability_scores()[*ability].score().modifier();
			saving_throw_modifier += saving_throw_prof.value() * state.proficiency_bonus();

			html! {
				<div class="p-1 text-center" {onclick}>
					<div class="row" style="--bs-gutter-x: 0;">
						<div class="col">
							<h5>{ability.long_name()}</h5>
						</div>
						<div class="col-3">
							<h5>
								{match ability_score.score().modifier() >= 0 {
									true => "+",
									false => "-",
								}}
								{ability_score.score().modifier().abs()}
							</h5>
						</div>
					</div>
					<div class="row" style="--bs-gutter-x: 0;">
						<div class="col">
							<div style="font-size: 0.75rem;">{"Score"}</div>
						</div>
						<div class="col-3">
							<div>{*ability_score.score()}</div>
						</div>
					</div>
					<div class="row align-items-center" style="--bs-gutter-x: 0;">
						<div class="col-auto" style="font-size: 0.75rem;">
							<glyph::ProficiencyLevel value={saving_throw_prof.value()} />
						</div>
						<div class="col">
							<div style="font-size: 0.75rem;">{"Saving Throw"}</div>
						</div>
						<div class="col-3">
							{match saving_throw_modifier >= 0 {
								true => "+",
								false => "-",
							}}
							{saving_throw_modifier.abs()}
						</div>
					</div>
				</div>
			}
		}
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct AbilityProps {
	pub ability: Ability,
}

#[function_component]
pub fn AbilityModifiers() -> Html {
	let mut modifiers = Vec::new();
	for ability in EnumSet::<Ability>::all() {
		modifiers.push(html!(<AbilityModifiersSingle {ability} />));
	}
	html! {
		<div style="font-size: 11px;">
			{modifiers}
		</div>
	}
}

#[function_component]
pub fn AbilityModifiersSingle(AbilityProps { ability }: &AbilityProps) -> Html {
	let style = "height: 14px; margin-right: 2px; margin-top: -2px; width: 14px; vertical-align: middle;";
	let state = use_context::<CharacterHandle>().unwrap();
	let mut modifiers = Vec::new();
	for (modifier, context, source) in state.skills()[*ability].modifiers().iter_all() {
		modifiers.push(html! {<div>
			<span class="d-inline-flex" aria-label="Advantage" {style}>
				<glyph::RollModifier value={modifier} />
			</span>
			<span>{"on "}{ability.abbreviated_name().to_uppercase()}{" checks"}</span>
			<span>
				{context.as_ref().map(|target| format!(" when {target}")).unwrap_or_default()}
			</span>
			{crate::data::as_feature_path_text(source).map(|path_text| html! {
				<span class="source-path-sm ms-1">
					{format!("({path_text})")}
				</span>
			})}
		</div>});
	}
	html!(<>{modifiers}</>)
}

#[function_component]
pub fn ScoreBreakdown(AbilityProps { ability }: &AbilityProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let ability_score = state.ability_scores().get(*ability);
	let modifier = ability_score.score().modifier();
	html! {<>
		<div class="text-center fs-5" style="width: 100%; margin-bottom: 10px;">
			<span>{"Total Score:"}</span>
			<span style="margin-left: 5px;">{*ability_score.score()}</span>
		</div>

		<div class="text-center fs-5" style="width: 100%; margin-bottom: 10px;">
			<span>{"Modifier:"}</span>
			<span style="margin-left: 5px;">{match modifier >= 0 { true => "+", false => "-", }}{modifier.abs()}</span>
		</div>

		<h6>{"Modifier Bonuses"}</h6>
		<table class="table table-compact table-striped m-0">
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Value"}</th>
					<th scope="col">{"Context"}</th>
					<th scope="col">{"Source"}</th>
				</tr>
			</thead>
			<tbody>
				{state.skills()[*ability].bonuses().iter().map(|(bonus, context, source)| {
					html! {<tr>
						<td class="text-center">{*bonus}</td>
						<td class="text-center">{context.as_ref().map(String::as_str).unwrap_or("Always")}</td>
						<td>{crate::data::as_feature_path_text(&source).unwrap_or_default()}</td>
					</tr>}
				}).collect::<Vec<_>>()}
			</tbody>
		</table>

		<h6>{"Score Bonuses"}</h6>
		<table class="table table-compact table-striped m-0">
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Value"}</th>
					<th scope="col">{"Max Total"}</th>
					<th scope="col">{"Source"}</th>
				</tr>
			</thead>
			<tbody>
				{ability_score.iter_bonuses().map(|(bonus, path, was_included)| {
					html! {<tr>
						<td class="text-center">{bonus.value}</td>
						<td class="text-center">{match &bonus.max_total {
							None => html! {{"None" }},
							Some(max) => html! {<span>
								{max}
								{match was_included {
									true => html!(<i class="bi bi-check-square" style="color: green;" />),
									false => html!(<i class="bi bi-x-square" style="color: red;" />),
								}}
							</span>},
						}}</td>
						<td>{crate::data::as_feature_path_text(&path).unwrap_or_default()}</td>
					</tr>}
				}).collect::<Vec<_>>()}
			</tbody>
		</table>

		<h6>{"Score Maximum Value"}</h6>
		<table class="table table-compact table-striped m-0">
			<caption>{"The largest value of these is used as the maximum bound for the score above."}</caption>
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Value"}</th>
					<th scope="col">{"Source"}</th>
				</tr>
			</thead>
			<tbody>
				{ability_score.iter_max_increases().map(|(value, path)| {
					html! {<tr>
						<td class="text-center">{value}</td>
						<td>{crate::data::as_feature_path_text(&path).unwrap_or_default()}</td>
					</tr>}
				}).collect::<Vec<_>>()}
			</tbody>
		</table>

		<h6>{"Score Minimum Value"}</h6>
		<table class="table table-compact table-striped m-0">
			<caption>{"The largest value of these is used as the minimum bound for the score above."}</caption>
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Value"}</th>
					<th scope="col">{"Source"}</th>
				</tr>
			</thead>
			<tbody>
				{ability_score.iter_minimums().map(|(value, path)| {
					html! {<tr>
						<td class="text-center">{value}</td>
						<td>{crate::data::as_feature_path_text(&path).unwrap_or_default()}</td>
					</tr>}
				}).collect::<Vec<_>>()}
			</tbody>
		</table>

		<h6>{"Roll Modifiers"}</h6>
		<AbilityModifiersSingle ability={*ability} />

	</>}
}

#[function_component]
fn Modal(AbilityProps { ability }: &AbilityProps) -> Html {
	let skills = EnumSet::<Skill>::all().into_iter().filter(|skill| skill.ability() == *ability);
	html! {<div class="w-100 h-100 scroll-container-y">
		<h1>
			<glyph::Ability value={*ability} />
			<span class="ms-2">{ability.long_name()}</span>
		</h1>

		<ScoreBreakdown ability={*ability} />

		<div style="margin-top: 10px; margin-bottom: 10px;">
			{ability.short_description()}
		</div>

		<h6>{ability.long_name()}{" Checks"}</h6>
		<div class="text-block" style="margin-bottom: 10px;">{ability.checks_description()}</div>

		{skills.map(|skill| html! {<>
			<h6>{skill.display_name()}</h6>
			<div class="text-block" style="margin-bottom: 10px;">{skill.description()}</div>
		</>}).collect::<Vec<_>>()}

		{ability.addendum_description().into_iter().map(|(title, content)| html! {<>
			<h6>{title}</h6>
			<div class="text-block" style="margin-bottom: 10px;">{content}</div>
		</>}).collect::<Vec<_>>()}
	</div>}
}
