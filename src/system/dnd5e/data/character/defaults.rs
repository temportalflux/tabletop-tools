use super::Character;
use crate::{
	kdl_ext::FromKDL,
	system::{
		core::SourceId,
		dnd5e::{BoxedMutator, DnD5e, SystemComponent},
	},
	utility::MutatorGroup,
};
use std::path::Path;

/// Contains mutators and features which are applied to every character using the module it is present in.
#[derive(Clone, PartialEq, Debug)]
pub struct DefaultsBlock {
	pub source_id: Option<SourceId>,
	pub mutators: Vec<BoxedMutator>,
}

crate::impl_kdl_node!(DefaultsBlock, "defaults");

impl SystemComponent for DefaultsBlock {
	type System = DnD5e;

	fn add_component(mut self, source_id: SourceId, system: &mut Self::System) {
		self.source_id = Some(source_id.clone());
		system.default_blocks.insert(source_id, self);
	}
}

impl MutatorGroup for DefaultsBlock {
	type Target = Character;

	fn set_data_path(&self, parent: &Path) {
		for mutator in &self.mutators {
			mutator.set_data_path(parent);
		}
	}

	fn apply_mutators(&self, stats: &mut Character, parent: &Path) {
		for mutator in &self.mutators {
			stats.apply(mutator, parent);
		}
	}
}

impl FromKDL for DefaultsBlock {
	fn from_kdl(
		node: &kdl::KdlNode,
		ctx: &mut crate::kdl_ext::NodeContext,
	) -> anyhow::Result<Self> {
		let mut mutators = Vec::new();
		for entry_node in node.query_all("scope() > mutator")? {
			mutators.push(ctx.parse_mutator(entry_node)?);
		}
		Ok(Self {
			source_id: None,
			mutators,
		})
	}
}