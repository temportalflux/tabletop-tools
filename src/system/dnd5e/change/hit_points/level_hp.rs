use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::data::character::Character, Change},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

// Sets the Hit Points granted for given class's level
#[derive(Clone, Debug, PartialEq)]
pub struct LevelHP {
	pub class_name: String,
	pub level_idx: usize,
	pub value: u32,
}

crate::impl_trait_eq!(LevelHP);
kdlize::impl_kdl_node!(LevelHP, "level_hp");

impl Change for LevelHP {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		let iter = character.persistent().classes.iter();
		let iter = iter.filter(|class| class.name == self.class_name);
		let mut iter = iter.map(|class| &class.levels);
		let Some(levels) = iter.next() else { return };
		let Some(level) = levels.get(self.level_idx) else { return };
		let Some(data_path) = level.hit_points.get_data_path() else { return };

		// Set the HP gained for that level
		character.persistent_mut().set_selected(data_path, Some(self.value.to_string()));

		// Recompile to update MaxHP
		character.persistent_mut().mark_structurally_changed();
	}
}

impl FromKdl<NodeContext> for LevelHP {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let class_name = node.next_str_req()?.to_owned();
		let level_idx = node.next_i64_req()? as usize;
		let value = node.next_i64_req()? as u32;
		Ok(Self { class_name, level_idx, value })
	}
}

impl AsKdl for LevelHP {
	fn as_kdl(&self) -> NodeBuilder {
		NodeBuilder::default()
			.with_entry(self.class_name.as_str())
			.with_entry(self.level_idx as i64)
			.with_entry(self.value as i64)
	}
}
