use crate::{
	components::context_menu,
	page::characters::sheet::{joined::editor::AutoExchangeSwitch, CharacterHandle, MutatorImpact},
	system::dnd5e::{
		components::{glyph, validate_uint_only},
		data::{
			character::Persistent,
			currency::{self, Wallet},
		},
	},
	utility::InputExt,
};
use itertools::Itertools;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Clone, PartialEq, Properties)]
pub struct WalletInlineProps {
	pub wallet: Wallet,
}
#[function_component]
pub fn WalletInline(WalletInlineProps { wallet }: &WalletInlineProps) -> Html {
	let kinds = currency::Kind::all().sorted().rev();
	return html! {<>
		{kinds.filter_map(|coin| {
			match wallet[coin] {
				0 => None,
				amt => Some(html! {
					<span>{amt} <glyph::Coin kind={coin} /></span>
				}),
			}
		}).collect::<Vec<_>>()}
	</>};
}

#[derive(Clone, PartialEq, Properties)]
pub struct WalletContainerProps {
	pub id: Option<Uuid>,
}

fn get_wallet<'c>(state: &'c CharacterHandle, id: &Option<Uuid>) -> Option<&'c Wallet> {
	match id {
		None => Some(state.inventory().wallet()),
		Some(id) => {
			let Some(item) = state.inventory().get_item(id) else {
				return None;
			};
			let Some(container) = &item.items else {
				return None;
			};
			Some(container.wallet())
		}
	}
}

fn get_wallet_mut<'c>(persistent: &'c mut Persistent, id: &Option<Uuid>) -> Option<&'c mut Wallet> {
	match id {
		None => Some(persistent.inventory.wallet_mut()),
		Some(id) => {
			let Some(item) = persistent.inventory.get_mut(id) else {
				return None;
			};
			let Some(container) = &mut item.items else {
				return None;
			};
			Some(container.wallet_mut())
		}
	}
}

#[function_component]
pub fn WalletInlineButton(WalletContainerProps { id }: &WalletContainerProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let onclick = context_menu::use_control_action({
		let id = id.clone();
		move |evt: MouseEvent, _context| {
			evt.stop_propagation();
			context_menu::Action::open_root("Coin Pouch", html!(<Modal {id} />))
		}
	});

	let Some(wallet) = get_wallet(&state, id).cloned() else {
		return Html::default();
	};
	html! {
		<span class="wallet-inline ms-auto py-2" {onclick}>
			{match wallet.is_empty() {
				true => html! { "Empty Coin Pouch" },
				false => html! {<WalletInline {wallet} />},
			}}
		</span>
	}
}

#[function_component]
fn Modal(WalletContainerProps { id }: &WalletContainerProps) -> Html {
	let state = use_context::<CharacterHandle>().unwrap();
	let Some(wallet) = get_wallet(&state, id) else {
		return Html::default();
	};
	let adjustment_wallet = use_state(|| Wallet::default());
	let balance_display = {
		let total_value_gold = wallet.total_value() / currency::Kind::Gold.multiplier();
		html! {
			<div>
				<div class="d-flex">
					<h6>{"My Coins"}</h6>
					<span class="ms-2" style="font-size: 0.8rem;">
						{"(est. "}
						{total_value_gold}
						{" GP"}
						<glyph::Coin classes="ms-1" kind={currency::Kind::Gold}/>
						{")"}
					</span>
				</div>
				{currency::Kind::all().sorted().rev().map(|coin| {
					let amount = wallet[coin];
					html! {<>
						<div class="d-flex py-1" style="font-size: 1.25rem;">
							<glyph::Coin kind={coin} classes="my-auto me-2" large={true} />
							<div class="my-auto">{coin.to_string()}{" ("}{coin.abbreviation()}{")"}</div>
							<div class="my-auto ms-auto me-3">{amount}</div>
						</div>
						<span class="hr my-1" />
					</>}
				}).collect::<Vec<_>>()}
			</div>
		}
	};
	let adjustment_form = {
		let auto_exchange = state.persistent().settings.currency_auto_exchange;
		let is_empty = adjustment_wallet.is_empty();
		let contains_enough = wallet.contains(&*adjustment_wallet, auto_exchange);
		let on_change_adj_coin = Callback::from({
			let wallet = adjustment_wallet.clone();
			move |(evt, coin): (web_sys::Event, currency::Kind)| {
				let Some(value) = evt.input_value_t::<u64>() else {
					return;
				};
				wallet.set({
					let mut wallet = (*wallet).clone();
					wallet[coin] = value;
					wallet
				});
			}
		});
		let onclick_add = Callback::from({
			let adjustments = adjustment_wallet.clone();
			let id = id.clone();
			let state = state.clone();
			move |_| {
				let adjustments = {
					let wallet = *adjustments;
					adjustments.set(Wallet::default());
					wallet
				};
				state.dispatch(Box::new(move |persistent: &mut Persistent| {
					if let Some(target) = get_wallet_mut(persistent, &id) {
						*target += adjustments;
					}
					MutatorImpact::None
				}));
			}
		});
		let onclick_remove = Callback::from({
			let adjustments = adjustment_wallet.clone();
			let id = id.clone();
			let state = state.clone();
			move |_| {
				if !contains_enough {
					return;
				}
				let adjustments = {
					let wallet = *adjustments;
					adjustments.set(Wallet::default());
					wallet
				};
				state.dispatch(Box::new(move |persistent: &mut Persistent| {
					let Some(target) = get_wallet_mut(persistent, &id) else {
						return MutatorImpact::None;
					};
					assert!(target.contains(&adjustments, auto_exchange));
					target.remove(adjustments, auto_exchange);
					MutatorImpact::None
				}));
			}
		});
		let onclick_clear = Callback::from({
			let wallet = adjustment_wallet.clone();
			move |_| {
				wallet.set(Wallet::default());
			}
		});
		let mut exchange_div_classes = classes!("ms-auto");
		if !auto_exchange {
			exchange_div_classes.push("v-hidden");
		}
		let onclick_exchange = Callback::from({
			let id = id.clone();
			let state = state.clone();
			move |_| {
				if !auto_exchange {
					return;
				}
				state.dispatch(Box::new(move |persistent: &mut Persistent| {
					let Some(target) = get_wallet_mut(persistent, &id) else {
						return MutatorImpact::None;
					};
					target.normalize();
					MutatorImpact::None
				}));
			}
		});
		html! {
			<div>
				<div class="d-flex">
					<h6 class="my-auto">{"Adjust Coins"}</h6>
					<div class={exchange_div_classes}>
						<button
							type="button"
							class="btn btn-outline-secondary btn-sm my-1"
							onclick={onclick_exchange}
						>{"Exchange Coins"}</button>
					</div>
				</div>
				<div class="row mb-2 gx-2">
					{currency::Kind::all().sorted().rev().map(|coin| {
						html! {<>
							<div class="col">
								<div class="d-flex justify-content-center">
									<glyph::Coin kind={coin} classes="my-auto me-1" />
									{coin.abbreviation().to_uppercase()}
								</div>
								<input
									type="number" class="form-control text-center p-0"
									min="0"
									value={format!("{}", adjustment_wallet[coin])}
									onkeydown={validate_uint_only()}
									onchange={on_change_adj_coin.reform(move |evt| (evt, coin))}
								/>
							</div>
						</>}
					}).collect::<Vec<_>>()}
				</div>
				<div class="d-flex justify-content-center">
					<button
						type="button" class="btn btn-success btn-sm mx-2"
						disabled={is_empty}
						onclick={onclick_add}
					>{"Add"}</button>
					<button
						type="button" class="btn btn-danger btn-sm mx-2"
						disabled={is_empty || !contains_enough}
						onclick={onclick_remove}
					>{"Remove"}</button>
					<button
						type="button" class="btn btn-secondary btn-sm mx-2"
						disabled={is_empty}
						onclick={onclick_clear}
					>{"Clear"}</button>
				</div>
				<div
					class={contains_enough.then_some("d-none").unwrap_or_default()}
					style="font-size: 0.8rem; font-weight: 650; color: #dc3545;"
				>
					{"Not enough in pouch to remove this amount "}
					{format!("(auto-exchange is {})", match auto_exchange { true => "ON", false => "OFF" })}
				</div>
			</div>
		}
	};
	let settings = {
		html! {
			<div class="collapse" id="settingsCollapse">
				<div class="card card-body mb-3">
					<div class="d-flex">
						<h6>{"Settings"}</h6>
						<button
							type="button"
							class="btn-close ms-auto" aria-label="Close"
							data-bs-toggle="collapse" data-bs-target="#settingsCollapse"
						/>
					</div>
					<AutoExchangeSwitch />
				</div>
			</div>
		}
	};
	html! {<div class="w-100 h-100 scroll-container-y">
		<button
			type="button" class="btn btn-secondary btn-sm px-1 py-0 ms-2"
			data-bs-toggle="collapse" data-bs-target="#settingsCollapse"
		>
			<i class="bi bi-gear-fill me-2" />
			{"Settings"}
		</button>
		{settings}
		{balance_display}
		{adjustment_form}
	</div>}
}
