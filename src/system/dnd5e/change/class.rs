use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, Class, Indirect},
		Change, SourceId,
	},
	utility::NotInList,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyClass {
	// NOTE: This is only populated when the change is applied, it is not deserialized from description.
	// It is too big to be included in the serialized description, so it should be parsed from database when displayed to users.
	Add(Indirect<Class>),
	Remove(SourceId),
	Level(SourceId, usize),
}

crate::impl_trait_eq!(ApplyClass);
kdlize::impl_kdl_node!(ApplyClass, "class");

impl ApplyClass {
	pub fn add(class: Class) -> Self {
		Self::Add(Indirect::Object(class))
	}

	pub fn remove(class_id: SourceId) -> Self {
		Self::Remove(class_id.into_unversioned())
	}

	pub fn set_level(class_id: SourceId, level: usize) -> Self {
		Self::Level(class_id.into_unversioned(), level)
	}
}

impl Change for ApplyClass {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::Add(Indirect::Object(class)) => {
				let mut class = class.clone();
				class.current_level = 1;
				character.persistent_mut().add_class(class);
				character.persistent_mut().mark_structurally_changed();
			}
			Self::Add(Indirect::Id(_)) => {}
			Self::Remove(class_id) => {
				let classes = &mut character.persistent_mut().classes;
				let mut removed = false;
				classes.retain_mut(|class| match class.id.unversioned() == *class_id {
					false => true,
					true => {
						removed = true;
						false
					}
				});
				if removed {
					character.persistent_mut().mark_structurally_changed();
				}
			}
			Self::Level(class_id, level) => {
				let mut iter = character.persistent_mut().classes.iter_mut();
				let class = iter.find(|class| class.id.unversioned() == *class_id);
				let Some(class) = class else { return };
				class.current_level = *level;
				character.persistent_mut().mark_structurally_changed();
			}
		}
	}
}

impl FromKdl<NodeContext> for ApplyClass {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"add" => {
				let class_id = node.next_str_req_t()?;
				Ok(Self::Add(Indirect::Id(class_id)))
			}
			"remove" => {
				let class_id = node.next_str_req_t()?;
				Ok(Self::Remove(class_id))
			}
			"level" => {
				let class_id = node.next_str_req_t()?;
				let level = node.next_i64_req()? as usize;
				Ok(Self::Level(class_id, level))
			}
			id => Err(NotInList(id.to_owned(), vec!["add", "remove", "level"]).into()),
		}
	}
}

impl AsKdl for ApplyClass {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Add(Indirect::Id(class_id)) => {
				node.entry("add");
				node.entry(class_id.to_string());
			}
			Self::Add(Indirect::Object(class)) => {
				node.entry("add");
				node.entry(class.id.to_string());
			}
			Self::Remove(class_id) => {
				node.entry("remove");
				node.entry(class_id.to_string());
			}
			Self::Level(class_id, level) => {
				node.entry("level");
				node.entry(class_id.to_string());
				node.entry(*level as i64);
			}
		}
		node
	}
}
