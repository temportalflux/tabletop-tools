use super::IndirectCondition;
use crate::kdl_ext::{NodeContext, NodeReader};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::str::FromStr;

mod activation;
pub use activation::*;
mod attack;
pub use attack::*;
mod limited_uses;
pub use limited_uses::*;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Action {
	pub activation_kind: ActivationKind,
	pub attack: Option<Attack>,
	/// Dictates how many times this action can be used until it is reset.
	pub limited_uses: Option<LimitedUses>,
	/// Conditions applied when the action is used.
	pub conditions_to_apply: Vec<IndirectCondition>,
}

impl FromKdl<NodeContext> for Action {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut NodeReader<'doc>) -> anyhow::Result<Self> {
		let activation_kind = match (node.next_str_opt()?, node.query_opt("scope() > activation")?) {
			(Some(str), None) => ActivationKind::from_str(str)?,
			(None, Some(mut node)) => ActivationKind::from_kdl(&mut node)?,
			_ => return Err(MissingActivation(node.to_string()).into()),
		};

		let attack = node.query_opt_t::<Attack>("scope() > attack")?;
		let limited_uses = node.query_opt_t::<LimitedUses>("scope() > limited_uses")?;

		let conditions_to_apply = node.query_all_t::<IndirectCondition>("scope() > condition")?;

		Ok(Self { activation_kind, attack, limited_uses, conditions_to_apply })
	}
}

impl AsKdl for Action {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = self.activation_kind.as_kdl();
		node.child(("attack", &self.attack));
		node.child(("limited_uses", &self.limited_uses));
		node.children(("condition", self.conditions_to_apply.iter()));
		node
	}
}

#[derive(thiserror::Error, Debug)]
#[error("Action node is missing activation property: {0:?}")]
pub struct MissingActivation(String);

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::{
			kdl_ext::{test_utils::*, NodeContext},
			system::{
				dnd5e::{
					data::{
						roll::{Die, EvaluatedRoll},
						Ability, Condition, DamageRoll, DamageType, Resource, ResourceReset, Rest,
					},
					evaluator::GetLevelInt,
					Value,
				},
				generics, SourceId,
			},
			utility,
		};

		static NODE_NAME: &str = "action";

		fn node_ctx() -> NodeContext {
			NodeContext::registry(generics::Registry::default_with_eval::<GetLevelInt>())
		}

		#[test]
		fn basic() -> anyhow::Result<()> {
			let doc = "action \"Action\"";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: None,
				limited_uses: None,
				conditions_to_apply: Vec::new(),
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn attack() -> anyhow::Result<()> {
			let doc = "
				|action \"Action\" {
				|    attack {
				|        kind \"Melee\"
				|        check \"AttackRoll\" (Ability)\"Dexterity\" proficient=true
				|        damage base=1 {
				|            roll (Roll)\"2d6\"
				|            damage_type \"Fire\"
				|        }
				|    }
				|}
			";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: Some(Attack {
					kind: Some(AttackKindValue::Melee { reach: 5 }),
					check: AttackCheckKind::AttackRoll {
						ability: Ability::Dexterity,
						proficient: utility::Value::Fixed(true),
						bonus: 0,
					},
					area_of_effect: None,
					damage: Some(DamageRoll {
						roll: Some(EvaluatedRoll::from((2, Die::D6))),
						base_bonus: 1,
						damage_type: DamageType::Fire,
					}),
					weapon_kind: None,
					classification: None,
					properties: Vec::new(),
				}),
				limited_uses: None,
				conditions_to_apply: Vec::new(),
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn limited_uses_fixed() -> anyhow::Result<()> {
			let doc = "
				|action \"Action\" {
				|    limited_uses {
				|        max_uses 1
				|        reset_on \"Long\"
				|    }
				|}
			";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: None,
				limited_uses: Some(LimitedUses::Usage(Resource {
					capacity: Value::Fixed(1),
					reset: Some(ResourceReset { rest: Value::Fixed(Rest::Long.to_string()), rate: None }),
					..Default::default()
				})),
				conditions_to_apply: Vec::new(),
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn limited_uses_scaling() -> anyhow::Result<()> {
			let doc = "
				|action \"Action\" {
				|    limited_uses {
				|        max_uses (Evaluator)\"get_level\" {
				|            level 2 1
				|            level 5 2
				|            level 10 4
				|            level 14 5
				|            level 20 -1
				|        }
				|        reset_on \"Long\"
				|    }
				|}
			";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: None,
				limited_uses: Some(LimitedUses::Usage(Resource {
					capacity: Value::Evaluated(
						GetLevelInt {
							class_name: None,
							order_map: [(2, 1), (5, 2), (10, 4), (14, 5), (20, -1)].into(),
						}
						.into(),
					),
					reset: Some(ResourceReset { rest: Value::Fixed(Rest::Long.to_string()), rate: None }),
					..Default::default()
				})),
				conditions_to_apply: Vec::new(),
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn condition_by_id() -> anyhow::Result<()> {
			let doc = "
				|action \"Action\" {
				|    condition \"condition/invisible.kdl\"
				|    condition \"condition/unconscious.kdl\"
				|}
			";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: None,
				limited_uses: None,
				conditions_to_apply: vec![
					IndirectCondition::Id(SourceId { path: "condition/invisible.kdl".into(), ..Default::default() }),
					IndirectCondition::Id(SourceId { path: "condition/unconscious.kdl".into(), ..Default::default() }),
				],
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn condition_custom() -> anyhow::Result<()> {
			let doc = "
				|action \"Action\" {
				|    condition \"Specific\" name=\"Slippery\"
				|}
			";
			let data = Action {
				activation_kind: ActivationKind::Action,
				attack: None,
				limited_uses: None,
				conditions_to_apply: vec![IndirectCondition::Object(Condition {
					name: "Slippery".into(),
					..Default::default()
				})],
			};
			assert_eq_fromkdl!(Action, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
