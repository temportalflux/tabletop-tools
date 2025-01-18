use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use enum_map::EnumMap;
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct HealOrDamage {
	pub delta: i32,
	pub current: u32,
	pub temp: u32,
	pub clear_saves: bool,
}

crate::impl_trait_eq!(HealOrDamage);
kdlize::impl_kdl_node!(HealOrDamage, "heal_or_damage");

impl Change for HealOrDamage {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let hit_points = &mut character.persistent_mut().hit_points_mut();
		hit_points.current = self.current;
		hit_points.temp = self.temp;
		if self.clear_saves {
			hit_points.saves = EnumMap::default();
		}
	}
}

impl FromKdl<NodeContext> for HealOrDamage {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let delta = node.next_i64_req()? as i32;
		let current = node.next_i64_req()? as u32;
		let temp = node.next_i64_req()? as u32;
		let clear_saves = node.get_bool_req("clear_saves")?;
		Ok(Self { delta, current, temp, clear_saves })
	}
}

impl AsKdl for HealOrDamage {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.delta as i64);
		node.entry(self.current as i64);
		node.entry(self.temp as i64);
		node.entry(("clear_saves", self.clear_saves));
		node
	}
}
