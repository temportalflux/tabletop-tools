use crate::{
	components::{context_menu, stop_propagation},
	page::characters::sheet::CharacterHandle,
	system::dnd5e::{
		components::UseCounterDelta,
		data::{DeathSave, HitPoint},
	},
	utility::InputExt,
};
use std::cmp::Ordering;
use yew::prelude::*;

static TEXT_HIT_POINTS: &'static str = "\
Hit points represent a combination of physical and mental durability, \
the will to live, and luck. Creatures with more hit points are more \
difficult to kill. Those with fewer hit points are more fragile.

A creature's current hit points (usually just called hit points) \
can be any number from the creature's hit point maximum down to 0. \
This number changes frequently as a creature takes damage or receives healing.

Whenever a creature takes damage, that damage is subtracted from its hit points. \
The loss of hit points has no effect on a creature's capabilities \
until the creature drops to 0 hit points.";
static TEXT_TEMP_HP: &'static str = "\
Some spells and special abilities confer temporary hit points to a creature. \
Temporary hit points aren't actual hit points; they are a buffer against damage, \
a pool of hit points that protect you from injury.

When you have temporary hit points and take damage, the temporary hit points \
are lost first, and any leftover damage carries over to your normal hit points. \
For example, if you have 5 temporary hit points and take 7 damage, \
you lose the temporary hit points and then take 2 damage.

Because temporary hit points are separate from your actual hit points, \
they can exceed your hit point maximum. A character can, therefore, \
be at full hit points and receive temporary hit points.

Healing can't restore temporary hit points, and they can't be added together. \
If you have temporary hit points and receive more of them, you decide whether \
to keep the ones you have or to gain the new ones. For example, if a spell \
grants you 12 temporary hit points when you already have 10, \
you can have 12 or 10, not 22.

If you have 0 hit points, receiving temporary hit points doesn't restore you \
to consciousness or stabilize you. They can still absorb damage directed at \
you while you're in that state, but only true healing can save you.

Unless a feature that grants you temporary hit points has a duration, \
they last until they're depleted or you finish a long rest.";
static TEXT_HEALING: &'static str = "\
Unless it results in death, damage isn't permanent. Even death is reversible \
through powerful magic. Rest can restore a creature's hit points, \
and magical methods such as a cure wounds spell or a \
potion of healing can remove damage in an instant.

When a creature receives healing of any kind, hit points regained are added \
to its current hit points. A creature's hit points can't exceed its \
hit point maximum, so any hit points regained in excess of \
this number are lost. For example, a druid grants a ranger \
8 hit points of healing. If the ranger has 14 current hit points \
and has a hit point maximum of 20, the ranger regains 6 hit points from the druid, not 8.

A creature that has died can't regain hit points until magic \
such as the revivify spell has restored it to life.";
static TEXT_DROP_TO_ZERO: &'static str = "\
When you drop to 0 hit points, you either die outright or fall unconscious, \
as explained in the following sections.";
static TEXT_DTZ_INSTANT_DEATH: &'static str = "\
Massive damage can kill you instantly. When damage reduces you to 0 hit points and there is \
damage remaining, you die if the remaining damage equals or exceeds your hit point maximum.

For example, a cleric with a maximum of 12 hit points currently has 6 hit points. \
If she takes 18 damage from an attack, she is reduced to 0 hit points, but 12 damage remains. \
Because the remaining damage equals her hit point maximum, the cleric dies.";
static TEXT_DTZ_FALLING_UNCONSCIOUS: &'static str = "\
If damage reduces you to 0 hit points and fails to kill you, you fall unconscious. \
This unconsciousness ends if you regain any hit points.";
static TEXT_DTZ_SAVING_THROWS: &'static str = "\
Whenever you start your turn with 0 hit points, you must make a special saving throw, \
called a death saving throw, to determine whether you creep closer to death or hang onto life. \
Unlike other saving throws, this one isn't tied to any ability score. \
You are in the hands of fate now, aided only by spells and features that improve your \
chances of succeeding on a saving throw.";
static TEXT_DTZ_SAVING_THROWS_ROLL: &'static str = "\
If the roll is 10 or higher, you succeed. Otherwise, you fail. \
A success or failure has no effect by itself. On your third success, you become stable. \
On your third failure, you die. The successes and failures don't need to be consecutive; \
keep track of both until you collect three of a kind. \
The number of both is reset to zero when you regain any hit points or become stable.";
static TEXT_DTZ_SAVING_THROWS_ROLL_CRIT: &'static str = "\
When you make a death saving throw and roll a 1 on the d20, it counts as two failures. \
If you roll a 20 on the d20, you regain 1 hit point.";
static TEXT_DTZ_SAVING_THROWS_DMG: &'static str = "\
If you take any damage while you have 0 hit points, you suffer a death saving throw failure. \
If the damage is from a critical hit, you suffer two failures instead. If the damage equals \
or exceeds your hit point maximum, you suffer instant death.";
static TEXT_DTZ_STABILIZING: &'static str = "\
The best way to save a creature with 0 hit points is to heal it. If healing is unavailable, \
the creature can at least be stabilized so that it isn't killed by a failed death saving throw.

You can use your action to administer first aid to an unconscious creature and \
attempt to stabilize it, which requires a successful DC 10 Wisdom (Medicine) check.

A stable creature doesn't make death saving throws, even though it has 0 hit points, \
but it does remain unconscious. The creature stops being stable, and must start making \
death saving throws again, if it takes any damage. A stable creature that isn't \
healed regains 1 hit point after 1d4 hours.";

#[function_component]
pub fn HitPointMgmtCard() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let on_open_modal = context_menu::use_control_action({
		|_: web_sys::MouseEvent, _context| context_menu::Action::open_root(format!("Hit Points"), html!(<Modal />))
	});
	let current_hp = state.get_hp(HitPoint::Current);
	html! {
		<div class="card m-1 hit-points">
			<div class="card-body">
				{match current_hp > 0 {
					true => html! { <HitPointsBody {on_open_modal} /> },
					false => html! { <DeathSavesBody {on_open_modal} /> },
				}}
			</div>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct BodyProps {
	on_open_modal: Callback<MouseEvent>,
}
#[function_component]
fn HitPointsBody(BodyProps { on_open_modal }: &BodyProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let hp_input_node = use_node_ref();
	let take_hp_input = Callback::from({
		let node = hp_input_node.clone();
		move |_: ()| {
			let Some(value) = node.input_value() else {
				return None;
			};
			if value.is_empty() {
				return Some(1);
			}
			let Ok(value) = value.parse::<u32>() else {
				return None;
			};
			if let Some(input) = node.target_input() {
				input.set_value("");
			}
			Some(value)
		}
	});
	let max_hp = state.get_hp(HitPoint::Max);
	let onclick_heal = state.dispatch_change({
		let take_hp_input = take_hp_input.clone();
		move |evt: MouseEvent| {
			use crate::system::dnd5e::change::*;
			evt.stop_propagation();
			let Some(amt) = take_hp_input.emit(()) else { return None };
			Some(hit_points::HealOrDamage(amt as i32))
		}
	});
	let onclick_dmg = state.dispatch_change({
		let take_hp_input = take_hp_input.clone();
		move |evt: MouseEvent| {
			use crate::system::dnd5e::change::*;
			evt.stop_propagation();
			let Some(amt) = take_hp_input.emit(()) else { return None };
			Some(hit_points::HealOrDamage(-1 * (amt as i32)))
		}
	});

	html! {
		<div class="d-flex details hit-points">
			<div class="flex-grow-1" onclick={on_open_modal.clone()}>
				<h5 class="text-center" style="font-size: 0.8rem; color: var(--bs-card-title-color); margin: 0 0 2px 0;">{"Hit Points"}</h5>
				<div class="row text-center m-0" style="--bs-gutter-x: 0;">
					<div class="col" style="min-width: 50px;">
						<div style="font-size: 0.75rem; padding: 0 5px;">{"Current"}</div>
						<div style="font-size: 26px; font-weight: 500;">{state.get_hp(HitPoint::Current)}</div>
					</div>
					<div class="col-auto">
						<div style="min-height: 1.2rem;"></div>
						<div style="font-size: 23px; font-weight: 300;">{"/"}</div>
					</div>
					<div class="col" style="min-width: 50px;">
						<div style="font-size: 0.75rem; padding: 0 5px;">{"Max"}</div>
						<div style="font-size: 26px; font-weight: 500;">{max_hp}</div>
					</div>
					<div class="col" style="min-width: 50px; margin: 0 5px;">
						<div style="font-size: 0.75rem;">{"Temp"}</div>
						<div style="font-size: 26px; font-weight: 300;">{state.get_hp(HitPoint::Temp)}</div>
					</div>
				</div>
			</div>
			<div style="width: 80px;">
				<button
					type="button" class="btn btn-success btn-xs"
					style="vertical-align: top; width: 100%;"
					onclick={onclick_heal}
				>{"Heal"}</button>
				<input ref={hp_input_node}
					type="number" class="form-control text-center" id="hp-amount"
					style="padding: 0; margin: 0 0 4px 0; height: 20px;"
					min="0"
					onclick={stop_propagation()} onkeydown={validate_uint_only()}
				/>
				<button
					type="button" class="btn btn-danger btn-xs"
					style="vertical-align: top; width: 100%;"
					onclick={onclick_dmg}
				>{"Damage"}</button>
			</div>
		</div>
	}
}

#[function_component]
fn DeathSavesBody(BodyProps { on_open_modal }: &BodyProps) -> Html {
	html! {
		<div class="death-saves" onclick={on_open_modal.clone()}>
			<h5 class="text-center" style="font-size: 0.8rem; color: var(--bs-card-title-color); margin: 0 0 2px 0;">{"Death Saves"}</h5>
			<div class="row my-0 mx-2">
				<div class="col-auto p-0">
					<div style="height: 100%;" class="d-flex align-items-center">
						<span class="death-save-icon" />
					</div>
				</div>
				<div class="col">
					<div class="death-save-label">{"FAILURE"}</div>
					<div class="death-save-label">{"SUCCESS"}</div>
				</div>
				<div class="col-auto p-0" onclick={stop_propagation()}>
					<DeathSaveBoxes save={DeathSave::Failure} />
					<DeathSaveBoxes save={DeathSave::Success} />
				</div>
			</div>
		</div>
	}
}

pub fn validate_uint_only() -> Callback<KeyboardEvent> {
	Callback::from(|evt: KeyboardEvent| {
		if !evt.cancelable() {
			log::error!("Cannot cancel input::onkeydown event");
			return;
		}
		if evt.key().len() == 1 && evt.key().parse::<u32>().is_err() {
			evt.prevent_default();
		}
	})
}

#[function_component]
fn Modal() -> Html {
	html! {<>
		<ModalSectionDeathSaves />
		<ModalSectionCurrentStats />
		<span class="hr my-3" />
		<ModalSectionApplyChangeForm />
		<span class="hr my-3" />
		<ModalSectionHitDice />
		<span class="hr my-3" />
		<ModalSectionInfo />
	</>}
}

#[function_component]
fn ModalSectionDeathSaves() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	if state.get_hp(HitPoint::Current) > 0 {
		return html! {};
	}
	html! {
		<div class="death-saves">
			<h6 class="text-center">{"Death Saving Throws"}</h6>
			<div class="row m-0 justify-content-center">
				<div class="col-auto py-0 px-4">
					<h6>{"Failures"}</h6>
					<DeathSaveBoxes save={DeathSave::Failure} />
				</div>
				<div class="col-auto py-0 px-4">
					<h6>{"Successes"}</h6>
					<DeathSaveBoxes save={DeathSave::Success} />
				</div>
			</div>
			<span class="hr my-3" />
		</div>
	}
}

#[function_component]
fn ModalSectionCurrentStats() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let apply_temp_hp = state.dispatch_change(move |evt: web_sys::Event| {
		use crate::system::dnd5e::change::hit_points;
		let Some(value) = evt.input_value_t::<u32>() else { return None };
		Some(hit_points::TempHP(value))
	});
	html! {
		<div class="row my-1" style="--bs-gutter-x: 0;">
			<div class="col text-center">
				<h6>{"CURRENT HP"}</h6>
				<div style="font-size: 26px; font-weight: 500;">{state.get_hp(HitPoint::Current)}</div>
			</div>
			<div class="col text-center">
				<h6>{"MAX HP"}</h6>
				<div style="font-size: 26px; font-weight: 500;">{state.get_hp(HitPoint::Max)}</div>
			</div>
			<div class="col text-center">
				<h6>{"TEMP HP"}</h6>
				<input
					type="number" class="form-control text-center"
					style="font-size: 26px; font-weight: 500; padding: 0; height: 40px;"
					min="0"
					value={format!("{}", state.get_hp(HitPoint::Temp))}
					onkeydown={validate_uint_only()}
					onchange={apply_temp_hp}
				/>
			</div>
		</div>
	}
}

#[function_component]
fn ModalSectionApplyChangeForm() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let delta = use_state_eq(|| 0i32);
	let (delta_sig, delta_abs) = (delta.signum(), delta.abs() as u32);
	let prev_hp = state.get_hp(HitPoint::Current);
	let max_hp = state.get_hp(HitPoint::Max);
	let prev_temp = state.get_hp(HitPoint::Temp);
	let next_hit_points = state.persistent().hit_points().clone().plus_hp(*delta, max_hp);
	let healing_amt = delta_sig.max(0) as u32 * delta_abs;
	let damage_amt = (-delta_sig).max(0) as u32 * delta_abs;
	let new_hp_color_classes = match next_hit_points.current.cmp(&prev_hp) {
		Ordering::Greater => classes!("heal"),
		Ordering::Less => classes!("damage"),
		Ordering::Equal => classes!(),
	};
	let temp_hp_color_classes = match next_hit_points.temp.cmp(&prev_temp) {
		Ordering::Greater => classes!("heal"),
		Ordering::Less => classes!("damage"),
		Ordering::Equal => classes!(),
	};
	let temp_hp_classes = (prev_temp <= 0).then(|| classes!("d-none")).unwrap_or_default();

	let onchange_healing = Callback::from({
		let delta = delta.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.input_value_t::<u32>() else {
				return;
			};
			delta.set(value as i32);
		}
	});
	let onchange_damage = Callback::from({
		let delta = delta.clone();
		move |evt: web_sys::Event| {
			let Some(value) = evt.input_value_t::<u32>() else {
				return;
			};
			delta.set(value as i32 * -1);
		}
	});
	let onclick_add = Callback::from({
		let delta = delta.clone();
		move |_| {
			delta.set(delta.saturating_add(1));
		}
	});
	let onclick_sub = Callback::from({
		let delta = delta.clone();
		move |_| {
			delta.set(delta.saturating_sub(1));
		}
	});
	let apply_delta = state.dispatch_change({
		let delta = delta.clone();
		move |evt: MouseEvent| {
			use crate::system::dnd5e::change::*;
			evt.stop_propagation();
			let amt = *delta;
			delta.set(0);
			Some(hit_points::HealOrDamage(amt))
		}
	});
	let clear_delta = Callback::from({
		let delta = delta.clone();
		move |_| {
			delta.set(0);
		}
	});

	html! {
		<div class="row my-1">
			<div class="col">

				<div class="row mx-0 my-2">
					<div class="col-4 p-0">
						<label class="col-form-label text-center theme-healing" for="inputHealing" style="width: 100%">{"Healing"}</label>
					</div>
					<div class="col">
						<input
							class="form-control text-center theme-healing"
							type="number" id="inputHealing"
							style="font-size: 20px; font-weight: 500; padding: 0; height: 100%;"
							min="0" value={healing_amt.to_string()}
							onkeydown={validate_uint_only()}
							onchange={onchange_healing}
						/>
					</div>
				</div>

				<div class="d-flex justify-content-center">
					<button type="button" class="btn btn-theme hp-action sub" onclick={onclick_sub} />
					<button type="button" class="btn btn-theme hp-action add" onclick={onclick_add} />
				</div>

				<div class="row mx-0 my-2">
					<div class="col-4 p-0">
						<label
							class={classes!(
								"col-form-label",
								"text-center",
								"theme-damage"
							)}
							for="inputDamage" style="width: 100%"
						>{"Damage"}</label>
					</div>
					<div class="col">
						<input
							class={classes!(
								"form-control",
								"text-center",
								"theme-damage"
							)}
							type="number" id="inputDamage"
							style="font-size: 20px; font-weight: 500; padding: 0; height: 100%;"
							min="0" value={damage_amt.to_string()}
							onkeydown={validate_uint_only()}
							onchange={onchange_damage}
						/>
					</div>
				</div>

			</div>
			<div class="col-auto text-center m-auto">

				<div class="row m-0">
					<div class={{
						let mut classes = classes!("col");
						classes.extend(new_hp_color_classes.clone());
						classes
					}}>
						<h6 class="m-0 new-hp-header">{"NEW HP"}</h6>
						<div style="font-size: 40px; font-weight: 500; margin-top: -10px;">{next_hit_points.current}</div>
					</div>
					<div class={{
						let mut classes = classes!("col");
						classes.extend(temp_hp_color_classes.clone());
						classes.extend(temp_hp_classes);
						classes
					}}>
						<h6 class="m-0 new-hp-header">{"TEMP HP"}</h6>
						<div style="font-size: 40px; font-weight: 500; margin-top: -10px;">{next_hit_points.temp}</div>
					</div>
				</div>

				<button
					type="button"
					class="m-2 btn btn-theme"
					disabled={*delta == 0}
					onclick={apply_delta}
				>{"Apply Changes"}</button>
				<button
					type="button"
					class="m-2 btn btn-outline-theme"
					disabled={*delta == 0}
					onclick={clear_delta}
				>{"Cancel"}</button>
			</div>
		</div>
	}
}

#[function_component]
fn ModalSectionHitDice() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	static SECTION_DESC: &str = "\
		Manual access to your available hit dice per class. Generally hit dice are only used during a \
		Short Rest and reset on a Long Rest, but some special class features, feats, and spells take \
		and force the usage of hit dice outside of rests. These inputs will not change your hit points, \
		just the number of hit dice you have available until your next Long Rest.";

	let mut sections = Vec::new();
	for (die, capacity) in state.hit_dice().dice() {
		if *capacity == 0 {
			continue;
		}
		let Some(data_path) = state.hit_points().hit_dice_selectors[die].get_data_path() else {
			continue;
		};
		let consumed_uses = state.get_first_selection_at::<u32>(&data_path);
		let consumed_uses = consumed_uses.map(Result::ok).flatten().unwrap_or_default();
		sections.push(html! {<>
			<div class="uses d-flex">
				<span class="d-inline-block me-4" style="width: 100px; font-weight: 600px;">
					{die.to_string()}
				</span>
				<UseCounterDelta
					max_uses={*capacity as u32}
					consumed_uses={consumed_uses}
					on_apply={state.dispatch_change(move |uses_remaining_delta: i32| {
						Some(crate::system::dnd5e::change::hit_points::HitDice { die, delta: -uses_remaining_delta })
					})}
				/>
			</div>
			{state.hit_dice().sources().iter().map(|(roll, source)| {
				let path = crate::data::as_feature_path_text(source).unwrap_or("Unknown".into());
				html!(<div class="ms-3">{format!("{roll} - {path}")}</div>)
			}).collect::<Vec<_>>()}
		</>});
	}

	html! {
		<div>
			<h4>{"Hit Dice"}</h4>
			<div class="mb-2">{SECTION_DESC}</div>
			{sections}
		</div>
	}
}

#[function_component]
fn MaxHitPointsTable() -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let rows = state.max_hit_points().sources().iter().fold(Vec::new(), |mut html, (source, bonus)| {
		html.push(html! {
			<tr>
				<td class="text-center">{*bonus}</td>
				<td>{crate::data::as_feature_path_text(source).unwrap_or_default()}</td>
			</tr>
		});
		html
	});
	html! {
		<table class="table table-compact table-striped m-0">
			<thead>
				<tr class="text-center" style="color: var(--bs-heading-color);">
					<th scope="col">{"Bonus"}</th>
					<th scope="col">{"Source"}</th>
				</tr>
			</thead>
			<tbody>
				{rows}
			</tbody>
		</table>
	}
}

#[function_component]
fn ModalSectionInfo() -> Html {
	html! {
		<div class="accordion" id="hitPointsInformation">
			<div class="accordion-item">
				<h2 class="accordion-header">
					<button
						class="accordion-button collapsed" type="button"
						data-bs-toggle="collapse" data-bs-target="#collapseMaxHP"
					>{"Max HP Breakdown"}</button>
				</h2>
				<div id="collapseMaxHP" class="accordion-collapse collapse" data-bs-parent="#hitPointsInformation">
					<div class="accordion-body text-block">
						<MaxHitPointsTable />
					</div>
				</div>
			</div>
			<div class="accordion-item">
				<h2 class="accordion-header">
					<button
						class="accordion-button collapsed" type="button"
						data-bs-toggle="collapse" data-bs-target="#collapseHitPoints"
					>{"Hit Points"}</button>
				</h2>
				<div id="collapseHitPoints" class="accordion-collapse collapse" data-bs-parent="#hitPointsInformation">
					<div class="accordion-body text-block">
						{TEXT_HIT_POINTS}
					</div>
				</div>
			</div>
			<div class="accordion-item">
				<h2 class="accordion-header">
					<button
						class="accordion-button collapsed" type="button"
						data-bs-toggle="collapse" data-bs-target="#collapseTempHP"
					>{"Temporary Hit Points"}</button>
				</h2>
				<div id="collapseTempHP" class="accordion-collapse collapse" data-bs-parent="#hitPointsInformation">
					<div class="accordion-body text-block">
						{TEXT_TEMP_HP}
					</div>
				</div>
			</div>
			<div class="accordion-item">
				<h2 class="accordion-header">
					<button
						class="accordion-button collapsed" type="button"
						data-bs-toggle="collapse" data-bs-target="#collapseHealing"
					>{"Healing"}</button>
				</h2>
				<div id="collapseHealing" class="accordion-collapse collapse" data-bs-parent="#hitPointsInformation">
					<div class="accordion-body text-block">
						{TEXT_HEALING}
					</div>
				</div>
			</div>
			<div class="accordion-item">
				<h2 class="accordion-header">
					<button
						class="accordion-button collapsed" type="button"
						data-bs-toggle="collapse" data-bs-target="#collapseDTZ"
					>{"Dropping to 0 Hit Points"}</button>
				</h2>
				<div id="collapseDTZ" class="accordion-collapse collapse" data-bs-parent="#hitPointsInformation">
					<div class="accordion-body text-block">
						{TEXT_DROP_TO_ZERO}
						<span class="d-block my-2" />
						<div class="accordion" id="drop-to-zero">
							<div class="accordion-item">
								<h2 class="accordion-header">
									<button
										class="accordion-button collapsed" type="button"
										data-bs-toggle="collapse" data-bs-target="#collapseDTZInstantDeath"
									>{"Instant Death"}</button>
								</h2>
								<div id="collapseDTZInstantDeath" class="accordion-collapse collapse" data-bs-parent="#drop-to-zero">
									<div class="accordion-body text-block">
										{TEXT_DTZ_INSTANT_DEATH}
									</div>
								</div>
							</div>
							<div class="accordion-item">
								<h2 class="accordion-header">
									<button
										class="accordion-button collapsed" type="button"
										data-bs-toggle="collapse" data-bs-target="#collapseDTZUnconscious"
									>{"Falling Unconscious"}</button>
								</h2>
								<div id="collapseDTZUnconscious" class="accordion-collapse collapse" data-bs-parent="#drop-to-zero">
									<div class="accordion-body text-block">
										{TEXT_DTZ_FALLING_UNCONSCIOUS}
									</div>
								</div>
							</div>
							<div class="accordion-item">
								<h2 class="accordion-header">
									<button
										class="accordion-button collapsed" type="button"
										data-bs-toggle="collapse" data-bs-target="#collapseDTZSavingThrows"
									>{"Death Saving Throws"}</button>
								</h2>
								<div id="collapseDTZSavingThrows" class="accordion-collapse collapse" data-bs-parent="#drop-to-zero">
									<div class="accordion-body text-block">
										{TEXT_DTZ_SAVING_THROWS}
										<br /><br />
										<strong>{"Roll a d20. "}</strong>
										{TEXT_DTZ_SAVING_THROWS_ROLL}
										<br /><br />
										<strong>{"Rolling 1 or 20. "}</strong>
										{TEXT_DTZ_SAVING_THROWS_ROLL_CRIT}
										<br /><br />
										<strong>{"Damage at 0 Hit Points. "}</strong>
										{TEXT_DTZ_SAVING_THROWS_DMG}
									</div>
								</div>
							</div>
							<div class="accordion-item">
								<h2 class="accordion-header">
									<button
										class="accordion-button collapsed" type="button"
										data-bs-toggle="collapse" data-bs-target="#collapseDTZStabilizing"
									>{"Stabilizing a Creature"}</button>
								</h2>
								<div id="collapseDTZStabilizing" class="accordion-collapse collapse" data-bs-parent="#drop-to-zero">
									<div class="accordion-body text-block">
										{TEXT_DTZ_STABILIZING}
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	}
}

#[derive(Clone, PartialEq, Properties)]
struct DeathSaveBoxesProps {
	save: DeathSave,
}
#[function_component]
fn DeathSaveBoxes(DeathSaveBoxesProps { save }: &DeathSaveBoxesProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();

	let mut classes: Classes = classes!("form-check-input");
	classes.push(save.as_str().to_owned());

	let onchange = state.dispatch_change({
		let save = *save;
		move |evt: web_sys::Event| {
			use crate::system::dnd5e::change::hit_points;
			let Some(checked) = evt.input_checked() else { return None };
			Some(hit_points::DeathSaves {
				save,
				delta: match checked {
					true => 1,
					false => -1,
				},
			})
		}
	});

	let save_count = state.hit_points().saves[*save];
	html! {
		<div>
			<input class={classes.clone()} type="checkbox" onchange={onchange.clone()} checked={save_count >= 1} />
			<input class={classes.clone()} type="checkbox" onchange={onchange.clone()} checked={save_count >= 2} />
			<input class={classes.clone()} type="checkbox" onchange={onchange.clone()} checked={save_count >= 3} />
		</div>
	}
}
