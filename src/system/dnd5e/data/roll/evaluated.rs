use crate::{
	kdl_ext::{AsKdl, DocumentExt, FromKDL, NodeBuilder, NodeExt, ValueExt},
	system::dnd5e::{
		data::character::Character,
		data::roll::{Die, Roll},
		Value,
	},
};
use std::str::FromStr;

#[derive(Clone, PartialEq, Default, Debug)]
pub struct EvaluatedRoll {
	amount: Value<i32>,
	die: Option<Value<i32>>,
}

impl<T> From<T> for EvaluatedRoll
where
	Roll: From<T>,
{
	fn from(value: T) -> Self {
		let roll = Roll::from(value);
		Self {
			amount: Value::Fixed(roll.amount as i32),
			die: roll.die.map(|die| Value::Fixed(die.value() as i32)),
		}
	}
}

impl EvaluatedRoll {
	pub fn evaluate(&self, character: &Character) -> Roll {
		let amount = self.amount.evaluate(character) as u32;
		let die = match &self.die {
			None => None,
			Some(value) => {
				let die_value = value.evaluate(character) as u32;
				Die::try_from(die_value).ok()
			}
		};
		Roll { amount, die }
	}
}

impl FromKDL for EvaluatedRoll {
	fn from_kdl(
		node: &kdl::KdlNode,
		ctx: &mut crate::kdl_ext::NodeContext,
	) -> anyhow::Result<Self> {
		if let Some(roll_str) = node.get_str_opt(ctx.consume_idx())? {
			return Ok(Self::from(Roll::from_str(roll_str)?));
		}
		let amount = {
			let node = node.query_req("scope() > amount")?;
			let mut ctx = ctx.next_node();
			Value::from_kdl(
				node,
				node.entry_req(ctx.consume_idx())?,
				&mut ctx,
				|value| Ok(value.as_i64_req()? as i32),
			)?
		};
		let die = match node.query_opt("scope() > die")? {
			None => None,
			Some(node) => {
				let mut ctx = ctx.next_node();
				Some(Value::from_kdl(
					node,
					node.entry_req(ctx.consume_idx())?,
					&mut ctx,
					|value| Ok(value.as_i64_req()? as i32),
				)?)
			}
		};
		Ok(Self { amount, die })
	}
}

impl AsKdl for EvaluatedRoll {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			// These first two are when the EvaluatedRoll is a fixed Roll, and thus can be serialized as such
			Self {
				amount: Value::Fixed(amt),
				die: None,
			} => node.with_entry(format!("{amt}")),
			Self {
				amount: Value::Fixed(amt),
				die: Some(Value::Fixed(die)),
			} => node.with_entry_typed(format!("{amt}d{die}"), "Roll"),
			// While this one puts the amount and die into child nodes for evaluator serialization
			Self { amount, die } => {
				node.push_child_t("amount", amount);
				if let Some(die) = die {
					node.push_child_t("die", die);
				}
				node
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::{
			kdl_ext::{test_utils::*, NodeContext},
			system::{core::NodeRegistry, dnd5e::evaluator::GetProficiencyBonus},
		};

		static NODE_NAME: &str = "roll";

		fn node_ctx() -> NodeContext {
			NodeContext::registry(NodeRegistry::default_with_eval::<GetProficiencyBonus>())
		}

		#[test]
		fn basic_fixed() -> anyhow::Result<()> {
			let doc = "roll \"1\"";
			let data = EvaluatedRoll {
				amount: Value::Fixed(1),
				die: None,
			};
			assert_eq_fromkdl!(EvaluatedRoll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn basic_die() -> anyhow::Result<()> {
			let doc = "roll (Roll)\"3d4\"";
			let data = EvaluatedRoll {
				amount: Value::Fixed(3),
				die: Some(Value::Fixed(4)),
			};
			assert_eq_fromkdl!(EvaluatedRoll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn eval_amount() -> anyhow::Result<()> {
			let doc = "
				|roll {
				|    amount (Evaluator)\"get_proficiency_bonus\"
				|}
			";
			let data = EvaluatedRoll {
				amount: Value::Evaluated(GetProficiencyBonus.into()),
				die: None,
			};
			assert_eq_fromkdl!(EvaluatedRoll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn eval_die() -> anyhow::Result<()> {
			let doc = "
				|roll {
				|    amount 5
				|    die (Evaluator)\"get_proficiency_bonus\"
				|}
			";
			let data = EvaluatedRoll {
				amount: Value::Fixed(5),
				die: Some(Value::Evaluated(GetProficiencyBonus.into())),
			};
			assert_eq_fromkdl!(EvaluatedRoll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
