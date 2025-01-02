use super::Character;
use crate::{
	kdl_ext::NodeContext,
	system::dnd5e::data::{roll::Die, DeathSave},
	utility::selector::{self, IdPath},
};
use enum_map::EnumMap;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, PartialEq, Debug)]
pub struct HitPoints {
	pub current: u32,
	pub temp: u32,
	pub saves: EnumMap<DeathSave, u8>,
	pub hit_dice_selectors: EnumMap<Die, selector::Value<Character, u32>>,
}

impl Default for HitPoints {
	fn default() -> Self {
		Self {
			current: Default::default(),
			temp: Default::default(),
			saves: Default::default(),
			hit_dice_selectors: EnumMap::from_fn(|die| {
				let id = IdPath::from(Some(format!("hit_die/{die}")));
				selector::Value::Options(selector::ValueOptions { id, ..Default::default() })
			}),
		}
	}
}

impl FromKdl<NodeContext> for HitPoints {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let current = node.query_i64_req("scope() > current", 0)? as u32;
		let temp = node.query_i64_req("scope() > temp", 0)? as u32;

		let mut saves = EnumMap::<DeathSave, u8>::default();
		if let Some(node) = node.query_opt("scope() > saves")? {
			for (kind, amount) in &mut saves {
				*amount = node.get_i64_opt(&kind.to_string())?.unwrap_or(0) as u8;
			}
		}

		Ok(Self { current, temp, saves, ..Default::default() })
	}
}

impl AsKdl for HitPoints {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.child(("current", &self.current));
		node.child(("temp", &self.temp));
		if self.saves != Default::default() {
			node.child(("saves", {
				let mut node = NodeBuilder::default();
				for (kind, amount) in self.saves {
					if amount <= 0 {
						continue;
					}
					node.entry((kind.to_string(), amount as i64));
				}
				node
			}));
		}
		node
	}
}

impl HitPoints {
	pub fn set_temp_hp(&mut self, value: u32) {
		self.temp = value;
	}

	pub fn plus_hp(mut self, amount: i32, max: u32) -> Self {
		let mut amt_abs = amount.abs() as u32;
		let had_hp = self.current > 0;
		match amount.signum() {
			1 => {
				self.current = self.current.saturating_add(amt_abs).min(max);
			}
			-1 if self.temp >= amt_abs => {
				self.temp = self.temp.saturating_sub(amt_abs);
			}
			-1 if self.temp < amt_abs => {
				amt_abs -= self.temp;
				self.temp = 0;
				self.current = self.current.saturating_sub(amt_abs);
			}
			_ => {}
		}
		if !had_hp && self.current != 0 {
			self.saves = EnumMap::default();
		}
		self
	}
}

impl std::ops::Add<(i32, u32)> for HitPoints {
	type Output = Self;

	fn add(self, (amount, max): (i32, u32)) -> Self::Output {
		self.plus_hp(amount, max)
	}
}

impl std::ops::AddAssign<(i32, u32)> for HitPoints {
	fn add_assign(&mut self, rhs: (i32, u32)) {
		*self = self.clone() + rhs;
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::kdl_ext::test_utils::*;

	static NODE_NAME: &str = "hit_points";

	#[test]
	fn kdl() -> anyhow::Result<()> {
		let doc = "
			|hit_points {
			|    current 30
			|    temp 5
			|    failure_saves 1
			|    success_saves 2
			|}
		";
		let data = HitPoints {
			current: 30,
			temp: 5,
			saves: enum_map::enum_map! { DeathSave::Failure => 1, DeathSave::Success => 2},
			..Default::default()
		};
		assert_eq_fromkdl!(HitPoints, doc, data);
		assert_eq_askdl!(&data, doc);
		Ok(())
	}
}
