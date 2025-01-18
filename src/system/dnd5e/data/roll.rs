use crate::{kdl_ext::NodeContext, GeneralError};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::str::FromStr;

mod bonus_list;
pub use bonus_list::*;
mod die;
pub use die::*;
mod evaluated;
pub use evaluated::*;
mod modifier;
pub use modifier::*;
mod set;
pub use set::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Roll {
	amount: i32,
	die: Option<Die>,
}

impl From<i32> for Roll {
	fn from(amount: i32) -> Self {
		Self { amount, die: None }
	}
}

impl From<(i32, Die)> for Roll {
	fn from((amount, die): (i32, Die)) -> Self {
		Self { amount: amount, die: Some(die) }
	}
}

impl Roll {
	pub fn min(&self) -> i32 {
		self.amount
	}

	pub fn max(&self) -> i32 {
		self.amount * self.die.clone().map(Die::value).unwrap_or(1) as i32
	}

	pub fn abs(&self) -> Self {
		Self { amount: self.amount.abs(), die: self.die }
	}

	pub fn as_nonzero_string(&self) -> Option<String> {
		(self.amount != 0).then(|| self.to_string())
	}

	pub fn roll(&self, rand: &mut impl rand::Rng) -> i32 {
		match self.die {
			None => self.amount,
			Some(die) => die.roll(rand, self.amount),
		}
	}
}

impl std::fmt::Display for Roll {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Self::fmt_die_amount(f, self.die, self.amount)
	}
}

impl Roll {
	pub fn fmt_die_amount<Integer: std::fmt::Display>(
		f: &mut std::fmt::Formatter<'_>, die: Option<Die>, amount: Integer,
	) -> std::fmt::Result {
		match die {
			None => write!(f, "{}", amount),
			Some(die) => write!(f, "{}d{}", amount, die.value()),
		}
	}

	pub fn parse_die_amount<Integer>(s: &str) -> Result<(Option<Die>, Integer), ParseRollError>
	where
		Integer: std::str::FromStr,
		ParseRollError: From<Integer::Err>,
	{
		static EXPECTED: &'static str = "{int}d{int}";
		if !s.contains('d') {
			let amount = s.parse::<Integer>()?;
			return Ok((None, amount));
		}
		let mut parts = s.split('d');
		let amount_str = parts
			.next()
			.ok_or(GeneralError(format!("Roll string {s:?} missing amount, expected format {EXPECTED:?}.")))?;
		let die_str = parts
			.next()
			.ok_or(GeneralError(format!("Roll string {s:?} missing die type, expected format {EXPECTED:?}.")))?;
		if parts.next().is_some() {
			return Err(GeneralError(format!("Too many parts in {s:?} for Roll, expected {EXPECTED:?}")).into());
		}
		let amount = amount_str.parse::<Integer>()?;
		let die = Die::try_from(die_str.parse::<u32>()?)?;
		Ok((Some(die), amount))
	}
}

impl FromStr for Roll {
	type Err = ParseRollError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (die, amount) = Self::parse_die_amount(s)?;
		Ok(Self { amount, die })
	}
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ParseRollError {
	#[error(transparent)]
	ParseInt(#[from] std::num::ParseIntError),
	#[error(transparent)]
	Format(#[from] GeneralError),
}

impl FromKdl<NodeContext> for Roll {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		Ok(node.next_str_req_t::<Self>()?)
	}
}

impl AsKdl for Roll {
	fn as_kdl(&self) -> NodeBuilder {
		let node = NodeBuilder::default();
		if self.die.is_none() {
			node.with_entry(self.to_string())
		} else {
			node.with_entry_typed(self.to_string(), "Roll")
		}
	}
}

impl Roll {
	pub fn from_kdl_value(kdl: &kdl::KdlValue) -> anyhow::Result<Self> {
		if let Some(amt) = kdl.as_i64() {
			return Ok(Self::from(amt as i32));
		}
		if let Some(str) = kdl.as_string() {
			return Ok(Self::from_str(str)?);
		}
		Err(crate::kdl_ext::InvalidValueType(kdl.clone(), "i64 or string").into())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::kdl_ext::test_utils::*;

		static NODE_NAME: &str = "roll";

		#[test]
		fn fixed() -> anyhow::Result<()> {
			let doc = "roll \"4\"";
			let data = Roll::from(4);
			assert_eq_fromkdl!(Roll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn single_die() -> anyhow::Result<()> {
			let doc = "roll (Roll)\"1d6\"";
			let data = Roll::from((1, Die::D6));
			assert_eq_fromkdl!(Roll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn multi_die() -> anyhow::Result<()> {
			let doc = "roll (Roll)\"8d10\"";
			let data = Roll::from((8, Die::D10));
			assert_eq_fromkdl!(Roll, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
