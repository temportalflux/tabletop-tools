use crate::{
	bootstrap::components::Tooltip,
	components::AnnotatedNumber,
	data::ContextMut,
	system::dnd5e::data::{character::Character, Ability},
};
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct ScoreProps {
	pub ability: Ability,
}

#[function_component]
pub fn Score(ScoreProps { ability }: &ScoreProps) -> Html {
	let state = use_context::<ContextMut<Character>>().unwrap();
	let (score, attributed_to) = state.ability_score(*ability);
	let tooltip = (!attributed_to.is_empty()).then(|| {
		format!(
			"<div class=\"attributed-tooltip\">{}</div>",
			attributed_to
				.iter()
				.fold(String::new(), |mut content, (path, value)| {
					let source_text = crate::data::as_feature_path_text(&path);
					let sign = source_text
						.is_some()
						.then(|| match *value >= 0 {
							true => "+",
							false => "-",
						})
						.unwrap_or_default();
					let path_name = source_text.unwrap_or("original score".into());
					let value = value.abs();
					content += format!("<span>{sign}{value} ({path_name})</span>").as_str();
					content
				})
		)
	});
	html! {
		<div class="card ability-card" style="margin: 10px 5px; border-color: var(--theme-frame-color-muted);">
			<div class="card-body text-center">
				<h6 class="card-title">{ability.long_name()}</h6>
				<div class="primary-stat">
					<AnnotatedNumber value={score.modifier()} show_sign={true} />
				</div>
				<Tooltip classes={"secondary-stat"} content={tooltip} use_html={true}>{*score}</Tooltip>
			</div>
		</div>
	}
}