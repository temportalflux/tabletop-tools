use super::{Die, Roll};
use enum_map::EnumMap;
use regex::Regex;

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct RollSet(EnumMap<Die, i32>, i32);

impl From<Roll> for RollSet {
	fn from(value: Roll) -> Self {
		let mut set = Self::default();
		set.push(value);
		set
	}
}

impl FromIterator<Roll> for RollSet {
	fn from_iter<T: IntoIterator<Item = Roll>>(iter: T) -> Self {
		let mut set = Self::default();
		for roll in iter {
			set.push(roll);
		}
		set
	}
}

impl From<i32> for RollSet {
	fn from(value: i32) -> Self {
		Self::from(Roll::from(value))
	}
}

impl std::ops::Mul<i32> for RollSet {
	type Output = Self;

	fn mul(mut self, rhs: i32) -> Self::Output {
		for (_die, amt) in &mut self.0 {
			*amt *= rhs;
		}
		self.1 *= rhs;
		self
	}
}

impl std::ops::AddAssign for RollSet {
	fn add_assign(&mut self, set: Self) {
		for (die, amt) in set.0 {
			self.0[die] += amt;
		}
		self.1 += set.1;
	}
}

impl std::ops::AddAssign<Roll> for RollSet {
	fn add_assign(&mut self, roll: Roll) {
		match roll.die {
			None => self.1 += roll.amount,
			Some(die) => {
				self.0[die] += roll.amount;
			}
		}
	}
}

impl RollSet {
	pub fn multiple(roll: &Roll, amount: i32) -> Self {
		let mut set = Self::default();
		match &roll.die {
			None => set.1 += roll.amount * amount,
			Some(die) => {
				set.0[*die] += roll.amount * amount;
			}
		}
		set
	}

	pub fn push(&mut self, roll: Roll) {
		*self += roll;
	}

	pub fn remove(&mut self, roll: Roll) {
		match roll.die {
			None => self.1 = self.1.saturating_sub(roll.amount),
			Some(die) => {
				self.0[die] = self.0[die].saturating_sub(roll.amount);
			}
		}
	}

	pub fn extend(&mut self, set: RollSet) {
		*self += set;
	}

	pub fn is_empty(&self) -> bool {
		if self.1 > 0 {
			return false;
		}
		for (_, amt) in &self.0 {
			if *amt > 0 {
				return false;
			}
		}
		true
	}

	pub fn take_flat_bonus(&mut self) -> i32 {
		let out = self.1;
		self.1 = 0;
		out
	}

	pub fn die_map(&self) -> &EnumMap<Die, i32> {
		&self.0
	}

	pub fn iter_rolls(&self) -> impl Iterator<Item = Roll> + '_ {
		let iter = self.0.iter();
		let iter = iter.filter_map(|(die, amt)| if *amt == 0 { None } else { Some(Roll::from((*amt, die))) });
		let fixed = (self.1 != 0).then(|| Roll::from(self.1));
		iter.chain(fixed.into_iter())
	}

	pub fn rolls(&self) -> Vec<Roll> {
		self.iter_rolls().collect()
	}

	pub fn min(&self) -> i32 {
		let mut value = self.1;
		for (_die, amt) in &self.0 {
			if *amt > 0 {
				value += *amt as i32;
			}
		}
		value
	}

	pub fn max(&self) -> i32 {
		let mut value = self.1;
		for (die, amt) in &self.0 {
			if *amt > 0 {
				value += *amt * die.value() as i32;
			}
		}
		value
	}

	pub fn roll(&self, rand: &mut impl rand::Rng) -> i32 {
		let mut value = self.1;
		for (die, amt) in &self.0 {
			value += die.roll(rand, *amt) as i32;
		}
		value
	}

	pub fn to_minified_string(&self) -> String {
		self.to_string().replace(' ', "")
	}
}

impl std::fmt::Display for RollSet {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut first = true;
		for roll in self.iter_rolls() {
			if roll.amount == 0 {
				continue;
			}

			let roll_str = roll.to_string();
			if first {
				write!(f, "{roll_str}")?;
				first = false;
				continue;
			}

			if let Some(stripped) = roll_str.strip_prefix("-") {
				write!(f, " - {stripped}")?;
			} else {
				write!(f, " + {roll_str}")?;
			}
		}
		Ok(())
	}
}

static ROLL_SET_REGEX: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
	// https://regexr.com/84if5
	// a. (optional)'-'
	// b. 0-9 at least once
	// c. (optional) group of:
	//    i. 'd'
	//    ii. 0-9 at least once
	// Valid formats: "10", "-10", "1d4", "1d12+10", "1d8-5", "1d4+2d6-2"
	Regex::new(r"-?[0-9]+(?:d[0-9]+)?").expect("invalid roll set regex")
});
impl std::str::FromStr for RollSet {
	type Err = super::ParseRollError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut set = Self::default();
		// NOTE: Regex search is expensive. If I ever prioritize performance over data storage format,
		// it would certainly be more performant to store Roll(Set) as a mathematic combination enum rather than a roll-string.
		for regex_match in ROLL_SET_REGEX.find_iter(s) {
			let roll = Roll::from_str(regex_match.as_str())?;
			set.push(roll);
		}
		Ok(set)
	}
}

impl kdlize::AsKdl for RollSet {
	fn as_kdl(&self) -> kdlize::NodeBuilder {
		let mut node = kdlize::NodeBuilder::default();
		if !self.is_empty() {
			node.entry(self.to_minified_string());
		}
		node
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn unsigned_integer() {
		let parsed = RollSet::from_str("2");
		assert_eq!(parsed, Ok(RollSet::from(Roll::from(2))));
	}

	#[test]
	fn signed_integer_positive() {
		let parsed = RollSet::from_str("+2");
		assert_eq!(parsed, Ok(RollSet::from(Roll::from(2))));
	}

	#[test]
	fn signed_integer_negative() {
		let parsed = RollSet::from_str("-2");
		assert_eq!(parsed, Ok(RollSet::from(Roll::from(-2))));
	}

	#[test]
	fn positive_1d4() {
		let parsed = RollSet::from_str("1d4");
		assert_eq!(parsed, Ok(RollSet::from(Roll::from((1, Die::D4)))));
	}

	#[test]
	fn negative_2d6() {
		let parsed = RollSet::from_str("-2d6");
		assert_eq!(parsed, Ok(RollSet::from(Roll::from((-2, Die::D6)))));
	}

	#[test]
	fn group_2d6_minus_1d4() {
		let parsed = RollSet::from_str("2d6-1d4");
		assert_eq!(parsed, Ok(RollSet::from_iter([Roll::from((2, Die::D6)), Roll::from((-1, Die::D4)),])));
	}

	#[test]
	fn group_3d8_plus_1d12_minus_5() {
		let parsed = RollSet::from_str("3d8+1d12-5");
		assert_eq!(
			parsed,
			Ok(RollSet::from_iter([Roll::from((3, Die::D8)), Roll::from((1, Die::D12)), Roll::from(-5),]))
		);
	}
}
