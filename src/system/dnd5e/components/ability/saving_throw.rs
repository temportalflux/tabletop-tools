use crate::{
	bootstrap::components::Tooltip,
	components::context_menu,
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{
		components::glyph,
		data::{roll::Modifier, Ability},
	},
};
use enumset::EnumSet;
use yew::prelude::*;

static TEXT: &'static str = "\
A saving throw — also called a save — represents an attempt to resist a spell, a trap, a poison, \
a disease, or a similar threat. You don't normally decide to make a saving throw; you are forced \
to make one because your character or monster is at risk of harm.

To make a saving throw, roll a d20 and add the appropriate ability modifier. \
For example, you use your Dexterity modifier for a Dexterity saving throw.

A saving throw can be modified by a situational bonus or penalty and can be \
affected by advantage and disadvantage, as determined by the DM.

Each class gives proficiency in at least two saving throws. The wizard, for example, \
is proficient in Intelligence saves. As with skill proficiencies, proficiency in a saving throw \
lets a character add their proficiency bonus to saving throws made using a \
particular ability score. Some monsters have saving throw proficiencies as well.

The Difficulty Class for a saving throw is determined by the effect that causes it. \
For example, the DC for a saving throw allowed by a spell is determined by the caster's \
spellcasting ability and proficiency bonus.

The result of a successful or failed saving throw is also detailed in the effect \
that allows the save. Usually, a successful save means that a creature \
suffers no harm, or reduced harm, from an effect.";

#[derive(Clone, PartialEq, Properties)]
pub struct SavingThrowProps {
	pub ability: Ability,
	pub abbreviated: bool,
}

#[function_component]
pub fn SavingThrow(SavingThrowProps { ability, abbreviated }: &SavingThrowProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let mut modifier = state.ability_scores()[*ability].score().modifier();
	let proficiency = state.saving_throws()[*ability].proficiencies();
	modifier += proficiency.value() * state.proficiency_bonus();

	let mut contextless_bonuses = Vec::with_capacity(10);
	let mut context_bonuses = Vec::with_capacity(10);
	for (bonus, context, source) in state.saving_throws()[*ability].bonuses().iter() {
		if let Some(context) = context {
			context_bonuses.push((*bonus, context, source));
		} else {
			modifier += *bonus as i32;
			contextless_bonuses.push((*bonus, source));
		}
	}

	html! {<tr>
		<Tooltip tag={"td"} classes={"text-center"} use_html={true} content={abbreviated.then(|| {
			crate::data::as_feature_paths_html(proficiency.iter().map(|(_level, path)| path))
		}).flatten()}>
			<glyph::ProficiencyLevel value={proficiency.value()} />
		</Tooltip>
		<td class={"text-center"}>{match *abbreviated {
			true => ability.abbreviated_name().to_uppercase(),
			false => ability.long_name().to_owned(),
		}}</td>
		<Tooltip tag={"td"} classes={"text-center"} use_html={true} content={
			crate::data::as_feature_paths_html(contextless_bonuses.iter().map(|(_, source)| *source))
		}>
			<span style="font-weight: 700; color: var(--theme-roll-modifier);">
				{if modifier >= 0 { "+" } else { "-" }}{modifier.abs()}
			</span>
		</Tooltip>
		{(!abbreviated).then(|| html! {<td>
			{proficiency.iter().filter_map(|(_level, path)| {
				crate::data::as_feature_path_text(path)
			}).map(|text| html! {<div>{text}</div>}).collect::<Vec<_>>()}
			{context_bonuses.iter().filter_map(|(value, context, source)| {
				let Some(source) = crate::data::as_feature_path_text(source) else { return None };
				let sign = if *value >= 0 { "+" } else { "-" };
				Some(html!(<div>
					{sign}{value.abs()}{"when "}{context}{" ("}{source}{")"}
				</div>))
			}).collect::<Vec<_>>()}
		</td>}).unwrap_or_default()}
	</tr>}
}

#[function_component]
pub fn SavingThrowContainer() -> Html {
	let on_click = context_menu::use_control_action({
		|_, _context| context_menu::Action::open_root("Saving Throws", html!(<Modal />))
	});

	html! {
		<div id="saving-throw-container" class="card" onclick={on_click}>
			<div class="card-body" style="padding: 5px;">
				<h5 class="card-title text-center" style="font-size: 0.8rem;">{"Saving Throws"}</h5>
				<div class="row" style="--bs-gutter-x: 0; margin: 0 0 10px 0;">
					<div class="col">
						<table class="table table-compact" style="margin-bottom: 0;">
							<tbody>
								<SavingThrow ability={Ability::Strength} abbreviated={true} />
								<SavingThrow ability={Ability::Dexterity} abbreviated={true} />
								<SavingThrow ability={Ability::Constitution} abbreviated={true} />
							</tbody>
						</table>
					</div>
					<div class="col-auto p-0" style="margin: 0 5px;"><div class="vr" style="min-height: 100%;" /></div>
					<div class="col">
						<table class="table table-compact" style="margin-bottom: 0;">
							<tbody>
								<SavingThrow ability={Ability::Intelligence} abbreviated={true} />
								<SavingThrow ability={Ability::Wisdom} abbreviated={true} />
								<SavingThrow ability={Ability::Charisma} abbreviated={true} />
							</tbody>
						</table>
					</div>
				</div>
				<SavingThrowModifiers show_tooltip={true} />
			</div>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
pub struct ModifiersProps {
	#[prop_or_default]
	pub show_tooltip: bool,
	#[prop_or_default]
	pub show_none_label: bool,
}

#[function_component]
pub fn SavingThrowModifiers(ModifiersProps { show_tooltip, show_none_label }: &ModifiersProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let iter = state.saving_throws().iter();
	let iter = iter
		.map(|(ability, saving_throw)| {
			saving_throw
				.modifiers()
				.iter_all()
				.map(move |(modifier, context, source)| (ability, modifier, context, source))
		})
		.flatten();
	let modifiers = iter
		.map(|(ability, modifier, context, source)| {
			let entry = saving_throw_modifier(ability, modifier, context);
			if !*show_tooltip {
				return html!(<div>{entry}</div>);
			}
			html! {
				<Tooltip content={crate::data::as_feature_path_text(source)}>
					{entry}
				</Tooltip>
			}
		})
		.collect::<Vec<_>>();
	let content = match (modifiers.is_empty(), *show_none_label) {
		(false, _) => html! {<>{modifiers}</>},
		(true, false) => html!(),
		(true, true) => html!("None"),
	};
	html! {
		<div style="font-size: 11px;">
			{content}
		</div>
	}
}

pub fn saving_throw_modifier(ability: Ability, modifier: Modifier, context: &Option<String>) -> Html {
	let style = "height: 14px; margin-right: 2px; margin-top: -2px; width: 14px; vertical-align: middle;";
	html! {<>
		<span class="d-inline-flex" aria-label="Advantage" {style}>
			<glyph::RollModifier value={modifier} />
		</span>
		<span>{"on "}{ability.abbreviated_name().to_uppercase()}</span>
		<span>
			{context.as_ref().map(|target| format!(" against {target}")).unwrap_or_default()}
		</span>
	</>}
}

#[function_component]
fn Modal() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let abilities = {
		let mut all = EnumSet::<Ability>::all().into_iter().collect::<Vec<_>>();
		all.sort();
		all
	};
	let modifiers_section = {
		let iter = state.saving_throws().iter();
		let iter = iter
			.map(|(ability, saving_throw)| {
				saving_throw
					.modifiers()
					.iter_all()
					.map(move |(modifier, context, source)| (ability, modifier, context, source))
			})
			.flatten();
		let modifier_rows = iter
			.map(|(ability, modifier, context, source)| {
				let style = "height: 14px; margin-right: 2px; margin-top: -2px; width: 14px; vertical-align: middle;";
				html! {
					<tr>
						<td class="text-center">
							<span class="d-inline-flex" aria-label="Advantage" {style}>
								<glyph::RollModifier value={modifier} />
							</span>
						</td>
						<td class="text-center">{ability.long_name()}</td>
						<td class="text-center">{context.clone().unwrap_or_default()}</td>
						<td>{crate::data::as_feature_path_text(source)}</td>
					</tr>
				}
			})
			.collect::<Vec<_>>();
		if modifier_rows.is_empty() {
			html! {}
		} else {
			html! {<>
				<h5 class="text-center" style="margin-top: 15px;">{"Modifiers"}</h5>
				<table class="table table-compact table-striped m-0">
					<thead>
						<tr class="text-center" style="color: var(--bs-heading-color);">
							<th scope="col">{""}</th>
							<th scope="col">{"Ability"}</th>
							<th scope="col">{"Target"}</th>
							<th scope="col">{"Sources"}</th>
						</tr>
					</thead>
					<tbody>
						{modifier_rows}
					</tbody>
				</table>
			</>}
		}
	};

	html! {<div class="w-100 h-100 scroll-container-y">
		<table class="table table-compact table-striped m-0">
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Prof"}</th>
					<th scope="col">{"Ability"}</th>
					<th scope="col">{"Bonus"}</th>
					<th scope="col">{"Sources"}</th>
				</tr>
			</thead>
			<tbody>
				{abilities.into_iter().map(|ability| html! {
					<SavingThrow {ability} abbreviated={false} />
				}).collect::<Vec<_>>()}
			</tbody>
		</table>
		{modifiers_section}
		<div class="text-block" style="margin-top: 15px;">
			{TEXT}
		</div>
	</div>}
}
