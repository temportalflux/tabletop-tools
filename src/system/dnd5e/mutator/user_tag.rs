use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::{
			data::{
				character::{Character, UserTag},
				description,
			},
			generator::item,
		},
		mutator::ReferencePath,
		Mutator,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct AddUserTag {
	tag: String,
	max: Option<usize>,
	filter: Option<item::Filter>,
}

crate::impl_trait_eq!(AddUserTag);
kdlize::impl_kdl_node!(AddUserTag, "add_user_tag");

impl Mutator for AddUserTag {
	type Target = Character;

	fn description(&self, _state: Option<&Character>) -> description::Section {
		let mut body = format!("Add tag \"{}\" assignable to items.", self.tag);
		if let Some(max) = self.max {
			body += &format!(" It can be applied to at most {max} items.");
		}
		description::Section { content: body.into(), ..Default::default() }
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		stats.user_tags_mut().push(UserTag {
			tag: self.tag.clone(),
			max_count: self.max,
			filter: self.filter.clone(),
			source: parent.clone(),
			..Default::default()
		});
	}
}

impl FromKdl<NodeContext> for AddUserTag {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let tag = node.next_str_req()?.to_owned();
		let max = node.get_i64_opt("max")?.map(|v| v as usize);
		let filter = node.query_opt_t("scope() > filter")?;
		Ok(Self { tag, max, filter })
	}
}

impl AsKdl for AddUserTag {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.tag.clone());
		node.entry(("max", self.max.map(|v| v as i64)));
		node.child(("filter", self.filter.as_ref()));
		node
	}
}
