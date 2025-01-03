use crate::{
	bootstrap::components::Tooltip,
	components::{context_menu, Style},
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{
		components::glyph,
		data::{Ability, Skill},
	},
};
use enumset::{EnumSet, EnumSetType};
use itertools::Itertools;
use multimap::MultiMap;
use yew::prelude::*;

#[derive(Debug, EnumSetType)]
enum Presentation {
	Alphabetical,
	ByAbility,
}
impl Presentation {
	fn display_name(&self) -> &'static str {
		match self {
			Self::Alphabetical => "Alphabetical",
			Self::ByAbility => "By Ability",
		}
	}

	fn icon_html(&self) -> Html {
		match self {
			Self::Alphabetical => html! { <i class="bi bi-sort-alpha-down" /> },
			Self::ByAbility => html! { <i class="bi bi-menu-up" /> },
		}
	}
}
impl std::str::FromStr for Presentation {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Alphabetical" => Ok(Self::Alphabetical),
			"By Ability" => Ok(Self::ByAbility),
			_ => Err(()),
		}
	}
}

#[function_component]
pub fn SkillTable() -> Html {
	let presentation = use_state(|| Presentation::Alphabetical);

	let insert_ability_col_at = match *presentation {
		Presentation::Alphabetical => Some(1),
		Presentation::ByAbility => None,
	};
	let mut headers = vec![
		html! {<th scope="col">{"Prof"}</th>},
		// min-width is to ensure all segments have a wide enough column
		// to account for the largest skill name (so all segments look uniform).
		html! {<th scope="col" style="min-width: 135px;">{"Skill"}</th>},
		html! {<th scope="col">{"Bonus"}</th>},
		html! {<th scope="col">{"Passive"}</th>},
	];
	if let Some(idx) = &insert_ability_col_at {
		headers.insert(*idx, html! {<th scope="col">{"Ability"}</th>});
	}

	let skills: Vec<(Option<Ability>, Vec<Skill>)> = {
		let all = enumset::EnumSet::<Skill>::all().into_iter();
		// Transform the uniform unsorted list of skills based on what categories they should be sorted by.
		let skills = match *presentation {
			// Straight up only-alphabetical has no major category, so all of the skills are lumped together.
			Presentation::Alphabetical => vec![(None, all.into_iter().collect::<Vec<_>>())],
			// Group skills by their core ability.
			Presentation::ByAbility => {
				let skills: MultiMap<Option<Ability>, Skill> =
					all.map(|skill| (Some(skill.ability()), skill)).collect();
				let mut skills = skills.into_iter().collect::<Vec<_>>();
				skills.sort_by(|(ability_a, _), (ability_b, _)| ability_a.cmp(&ability_b));
				skills
			}
		};
		// Always alphabetize the subsections by skill display name
		skills
			.into_iter()
			.map(|(ability, mut skills)| {
				skills.sort_by(|a, b| a.display_name().cmp(&b.display_name()));
				(ability, skills)
			})
			.collect()
	};
	let mut segments = Vec::new();
	for (ability, skills) in skills.into_iter() {
		if let Some(ability) = ability {
			segments.push(html! {
				<div class="text-center" style="width: 100%;">{ability.long_name()}</div>
			});
		}
		segments.push(html! {
			<table class="table table-compact table-striped m-0">
				<thead>
					<tr class="text-center" style="font-size: 0.7rem;">{headers.clone()}</tr>
				</thead>
				<tbody>
					{skills.into_iter().map(|skill| html! {
						<Row {skill} ability_name_col={insert_ability_col_at} />
					}).collect::<Vec<_>>()}
				</tbody>
			</table>
		});
	}

	let on_change_presentation = {
		let state = presentation.clone();
		Callback::from(move |e: MouseEvent| {
			use std::str::FromStr;
			let Some(element) = e.target_dyn_into::<web_sys::HtmlElement>() else {
				return;
			};
			let value = element
				.get_attribute("value")
				.map(|s| Presentation::from_str(&s).ok())
				.flatten()
				.unwrap_or(Presentation::Alphabetical);
			state.set(value);
		})
	};

	html! {<>
		<div class="dropdown">
			<button class="btn btn-secondary dropdown-toggle" type="button" data-bs-toggle="dropdown" aria-expanded="false">
				{presentation.icon_html()}
			</button>
			<div class="dropdown-menu dropdown-menu-end" style="--bs-dropdown-min-width: 0rem;">
				{EnumSet::<Presentation>::all().into_iter().map(|value| html! {
					<a class="dropdown-item" value={value.display_name()} onclick={on_change_presentation.clone()}>
						{value.icon_html()}
						<span style="margin-left: 5px;">{value.display_name()}</span>
					</a>
				}).collect::<Vec<_>>()}
			</div>
		</div>
		{segments}
	</>}
}

#[derive(Clone, PartialEq, Properties)]
struct RowProps {
	pub skill: Skill,
	pub ability_name_col: Option<usize>,
}
#[function_component]
fn Row(RowProps { skill, ability_name_col }: &RowProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let proficiency = state.skills()[*skill].proficiencies();

	let mut modifier = state.ability_scores()[skill.ability()].score().modifier();
	modifier += proficiency.value() * state.proficiency_bonus();

	let mut bonuses = Vec::with_capacity(10);
	for (value, context, source) in state.skills()[skill.ability()].bonuses().iter() {
		bonuses.push((*value, context.clone(), source.clone()));
		if context.is_none() {
			modifier += *value as i32;
		}
	}
	for (value, context, source) in state.skills()[*skill].bonuses().iter() {
		bonuses.push((*value, context.clone(), source.clone()));
		if context.is_none() {
			modifier += *value as i32;
		}
	}
	let modifier_tooltip = crate::data::as_feature_paths_html_custom(
		bonuses.iter(),
		|(value, context, source)| ((*value, context.as_ref()), source.as_path()),
		|(value, context), path_str| {
			let sign = if value >= 0 { "+" } else { "-" };
			let context = context.map(|context| format!(" when {context}")).unwrap_or_else(|| " - included".to_owned());
			format!("<div>{sign}{}{context} ({})</div>", value.abs(), path_str)
		},
	);

	let passive = 10 + modifier;

	let prof_tooltip = crate::data::as_feature_paths_html_custom(
		proficiency.iter(),
		|(level, source)| (level, source.as_path()),
		|prof, path_str| format!("<div>{} ({})</div>", prof.as_display_name(), path_str),
	);

	let roll_modifier = {
		let iter_skill = state.skills()[*skill].modifiers().iter();
		let iter_ability = state.skills()[skill.ability()].modifiers().iter();
		let iter = iter_ability.chain(iter_skill);
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
				classes="d-block my-auto"
				style={Style::from([ ("width", "16px"), ("height", "16px") ])}
			/>
		}
	});

	let mut table_data = vec![
		html! {
			<Tooltip tag={"td"} classes={"text-center"} content={prof_tooltip} use_html={true}>
				<glyph::ProficiencyLevel value={proficiency.value()} />
			</Tooltip>
		},
		html! { <td>
			<div class="d-flex">
				<span class="flex-grow-1">{skill.display_name()}</span>
				{roll_modifier}
			</div>
		</td> },
		html! {
			<Tooltip tag={"td"} classes={"text-center"} content={modifier_tooltip} use_html={true}>
				{if modifier >= 0 { "+" } else { "-" }}{modifier.abs()}
			</Tooltip>
		},
		html! { <td class="text-center">{passive}</td> },
	];
	if let Some(idx) = ability_name_col {
		table_data.insert(*idx, html! {
			<td class="text-center" style="font-size: 12px; vertical-align: middle;">
				{skill.ability().abbreviated_name().to_uppercase()}
			</td>
		});
	}

	let onclick = context_menu::use_control_action({
		let skill = *skill;
		move |_, _context| {
			context_menu::Action::open_root(
				format!("{} ({})", skill.display_name(), skill.ability().long_name()),
				html!(<SkillModal {skill} />),
			)
		}
	});

	html! {<tr {onclick}>{table_data}</tr>}
}

#[derive(Clone, PartialEq, Properties)]
struct SkillModalProps {
	pub skill: Skill,
}

#[function_component]
fn SkillModal(SkillModalProps { skill }: &SkillModalProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let proficiency = state.skills()[*skill].proficiencies();
	let mut skill_modifier = state.ability_scores()[skill.ability()].score().modifier();
	skill_modifier += proficiency.value() * state.proficiency_bonus();

	let modifier_bonuses = {
		let iter_skill = state.skills()[*skill].bonuses().iter();
		let iter_ability = state.skills()[skill.ability()].bonuses().iter();
		let iter = iter_ability.chain(iter_skill);
		let mut modifier_bonuses = Vec::new();
		for (bonus, context, source) in iter {
			if context.is_none() {
				skill_modifier += *bonus as i32;
			}
			modifier_bonuses.push(html! {<tr>
				<td class="text-center">{*bonus}</td>
				<td class="text-center">{context.as_ref().map(String::as_str).unwrap_or("Always")}</td>
				<td>{crate::data::as_feature_path_text(&source).unwrap_or_default()}</td>
			</tr>});
		}
		modifier_bonuses
	};

	let proficiency_rows = proficiency
		.iter()
		.map(|(level, source)| {
			let source_text = crate::data::as_feature_path_text(source).unwrap_or_default();
			html! {
				<tr>
					<td class="d-flex align-items-center">
						<glyph::ProficiencyLevel value={level} />
						<span class="flex-grow-1 text-center" style="margin-left: 5px;">{level.as_display_name()}</span>
					</td>
					<td>{source_text}</td>
				</tr>
			}
		})
		.collect::<Vec<_>>();

	let prof_table = match proficiency_rows.is_empty() {
		true => html!(),
		false => html!(<div style="margin-bottom: 10px;">
			<table class="table table-compact table-striped m-0">
				<thead>
					<tr class="text-center" style="color: var(--bs-heading-color);">
						<th scope="col" style="width: 180px;">{"Proficiency"}</th>
						<th scope="col">{"Sources"}</th>
					</tr>
				</thead>
				<tbody>
					{proficiency_rows}
				</tbody>
			</table>
		</div>),
	};

	let modifier_rows = {
		let iter_skill = state.skills()[*skill].modifiers().iter_all();
		let iter_ability = state.skills()[skill.ability()].modifiers().iter_all();
		let iter = iter_ability.chain(iter_skill);
		iter.map(|(modifier, context, source)| {
			html! {
				<tr>
					<td class="d-flex">
						<glyph::RollModifier value={modifier}
							classes="d-block my-auto"
							style={Style::from([ ("width", "16px"), ("height", "16px") ])}
						/>
						<span class="flex-grow-1 text-center" style="margin-left: 5px;">{modifier.display_name()}</span>
					</td>
					<td class="text-center">{context.clone().unwrap_or_else(|| "--".into())}</td>
					<td>{crate::data::as_feature_path_text(source).unwrap_or_default()}</td>
				</tr>
			}
		})
		.collect::<Vec<_>>()
	};

	let roll_modifiers_table = match modifier_rows.is_empty() {
		true => html!(),
		false => html!(<div style="margin-bottom: 10px;">
			<table class="table table-compact table-striped m-0">
				<thead>
					<tr class="text-center" style="color: var(--bs-heading-color);">
						<th scope="col" style="width: 150px;">{"Modifier"}</th>
						<th scope="col">{"Target"}</th>
						<th scope="col">{"Source"}</th>
					</tr>
				</thead>
				<tbody>
					{modifier_rows}
				</tbody>
			</table>
		</div>),
	};

	html! {<div class="w-100 h-100 scroll-container-y">
		<div class="text-center fs-5" style="width: 100%; margin-bottom: 10px;">
			<span>{"Bonus:"}</span>
			<span style="margin-left: 5px;">{match skill_modifier >= 0 { true => "+", false => "-", }}{skill_modifier.abs()}</span>
		</div>
		{prof_table}

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
				{modifier_bonuses}
			</tbody>
		</table>

		{roll_modifiers_table}
		<div class="text-block">{skill.description()}</div>
	</div>}
}
