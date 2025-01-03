use crate::{
	bootstrap::components::Tooltip,
	components::{context_menu, AnnotatedNumber, AnnotatedNumberCard},
	page::characters::sheet::CharacterHandle,
	system::dnd5e::data::Ability,
};
use yew::prelude::*;

static TEXT: &'static str = "\
Initiative determines the order of turns during combat. When combat starts, every participant makes a \
Dexterity check to determine their place in the initiative order. The DM makes one roll for an entire \
group of identical creatures, so each member of the group acts at the same time.

The DM ranks the combatants in order from the one with the highest Dexterity check total to the one with the lowest. \
This is the order (called the initiative order) in which they act during each round. \
The initiative order remains the same from round to round.

If a tie occurs, the DM decides the order among tied DM-controlled creatures, and the players decide the order \
among their tied characters. The DM can decide the order if the tie is between a monster and a player character. \
Optionally, the DM can have the tied characters and monsters each roll \
a d20 to determine the order, highest roll going first.";

#[function_component]
pub fn InitiativeBonus() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let mut modifier = state.ability_scores()[Ability::Dexterity].score().modifier();
	modifier += state.initiative().proficiencies().value() * state.proficiency_bonus();

	let mut contextless_bonuses = Vec::with_capacity(10);
	let mut context_bonuses = Vec::with_capacity(10);
	for (bonus, context, source) in state.initiative().bonuses().iter() {
		if let Some(context) = context {
			context_bonuses.push((*bonus, context.clone(), source.clone()));
		} else {
			modifier += *bonus as i32;
			contextless_bonuses.push((*bonus, source.clone()));
		}
	}

	let on_click = context_menu::use_control_action({
		move |_, _context| {
			context_menu::Action::open_root(
				format!("Initiative Bonus"),
				html! {<div class="w-100 h-100 scroll-container-y">
					<div class="text-center fs-5" style="width: 100%; margin-bottom: 10px;">
						<span>{Ability::Dexterity.long_name()}{":"}</span>
						<Tooltip tag={"span"} style={"margin-left: 5px;"} use_html={true} content={
							crate::data::as_feature_paths_html_custom(
								contextless_bonuses.iter(),
								|(bonus, source)| (bonus, source.as_path()),
								|bonus, path_str| {
									let bonus_sign = if *bonus >= 0 { "+" } else { "-" };
									let bonus_abs = bonus.abs();
									format!("<div>{bonus_sign}{bonus_abs}: {path_str}</div>")
								}
							)
						}>
							{match modifier >= 0 { true => "+", false => "-", }}{modifier.abs()}
						</Tooltip>
					</div>
					<div class="text-block">
						{TEXT}

						{context_bonuses.iter().filter_map(|(value, context, source)| {
							let Some(source) = crate::data::as_feature_path_text(source) else { return None };
							let sign = if *value >= 0 { "+" } else { "-" };
							Some(html!(<div>
								{sign}{value.abs()}{"when "}{context}{" ("}{source}{")"}
							</div>))
						}).collect::<Vec<_>>()}
					</div>
				</div>},
			)
		}
	});
	html! {
		<AnnotatedNumberCard header={"Initiative"} footer={"Bonus"} {on_click}>
			<AnnotatedNumber value={modifier} show_sign={true} />
		</AnnotatedNumberCard>
	}
}
