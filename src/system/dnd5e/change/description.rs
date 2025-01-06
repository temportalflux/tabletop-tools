use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::character::{Character, PersonalityKind},
		Change,
	},
	utility::NotInList,
};
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub enum ApplyDescription {
	Name(String),
	Appearance(String),
	InsertPronoun(String),
	RemovePronoun(String),
	CustomPronouns(String),
	Height(u32),
	Weight(u32),
	HeightWeight(u32, u32),
	InsertPersonality { kind: PersonalityKind, new: String },
	RemovePersonality { kind: PersonalityKind, idx: usize, old: String },
	UpdatePersonality { kind: PersonalityKind, idx: usize, old: String, new: String },
}

crate::impl_trait_eq!(ApplyDescription);
kdlize::impl_kdl_node!(ApplyDescription, "description");

impl Change for ApplyDescription {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::Name(value) => {
				character.persistent_mut().description.name = value.clone();
			}
			Self::Appearance(value) => {
				character.persistent_mut().description.appearance = value.clone();
			}
			Self::InsertPronoun(value) => {
				character.persistent_mut().description.pronouns.insert(value.clone());
			}
			Self::RemovePronoun(value) => {
				character.persistent_mut().description.pronouns.remove(value);
			}
			Self::CustomPronouns(value) => {
				character.persistent_mut().description.custom_pronouns = value.clone();
			}
			Self::Height(value) => {
				character.persistent_mut().description.height = *value;
			}
			Self::Weight(value) => {
				character.persistent_mut().description.weight = *value;
			}
			Self::HeightWeight(height, weight) => {
				character.persistent_mut().description.height = *height;
				character.persistent_mut().description.weight = *weight;
			}
			Self::InsertPersonality { kind, new } => {
				let entries = &mut character.persistent_mut().description.personality[*kind];
				entries.push(new.clone());
			}
			Self::RemovePersonality { kind, idx, old: _ } => {
				let entries = &mut character.persistent_mut().description.personality[*kind];
				entries.remove(*idx);
			}
			Self::UpdatePersonality { kind, idx, old: _, new } => {
				let entries = &mut character.persistent_mut().description.personality[*kind];
				if let Some(entry) = entries.get_mut(*idx) {
					*entry = new.clone();
				}
			}
		}
	}
}

impl FromKdl<NodeContext> for ApplyDescription {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"name" => Ok(Self::Name(node.next_str_req()?.to_owned())),
			"appearance" => Ok(Self::Appearance(node.next_str_req()?.to_owned())),
			"pronoun" => match node.next_str_req()? {
				"insert" => Ok(Self::InsertPronoun(node.next_str_req()?.to_owned())),
				"remove" => Ok(Self::RemovePronoun(node.next_str_req()?.to_owned())),
				"custom" => Ok(Self::CustomPronouns(node.next_str_req()?.to_owned())),
				s => Err(NotInList(s.to_owned(), vec!["insert", "remove", "custom"]).into()),
			},
			"size" => {
				let height = node.get_i64_opt("height")?.map(|v| v as u32);
				let weight = node.get_i64_opt("weight")?.map(|v| v as u32);
				match (height, weight) {
					(Some(value), None) => Ok(Self::Height(value)),
					(None, Some(value)) => Ok(Self::Weight(value)),
					(Some(height), Some(weight)) => Ok(Self::HeightWeight(height, weight)),
					(None, None) => Err(anyhow::Error::msg(
						"size description change requires either or both height and weight named entries",
					)),
				}
			}
			"personality" => {
				let kind = node.next_str_req_t()?;
				match node.next_str_req()? {
					"insert" => {
						let new = node.query_str_req("scope() > new", 0)?.to_owned();
						Ok(Self::InsertPersonality { kind, new })
					}
					"remove" => {
						let idx = node.next_i64_req()? as usize;
						let old = node.query_str_req("scope() > old", 0)?.to_owned();
						Ok(Self::RemovePersonality { kind, idx, old })
					}
					"update" => {
						let idx = node.next_i64_req()? as usize;
						let old = node.query_str_req("scope() > old", 0)?.to_owned();
						let new = node.query_str_req("scope() > new", 0)?.to_owned();
						Ok(Self::UpdatePersonality { kind, idx, old, new })
					}
					s => Err(NotInList(s.to_owned(), vec!["insert", "remove", "update"]).into()),
				}
			}
			s => Err(NotInList(s.to_owned(), vec!["name", "appearance", "pronoun", "size", "personality"]).into()),
		}
	}
}

impl AsKdl for ApplyDescription {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Name(value) => {
				node.entry("name");
				node.entry(value.as_str());
			}
			Self::Appearance(value) => {
				node.entry("appearance");
				node.entry(value.as_str());
			}
			Self::InsertPronoun(value) => {
				node.entry("pronoun");
				node.entry("insert");
				node.entry(value.as_str());
			}
			Self::RemovePronoun(value) => {
				node.entry("pronoun");
				node.entry("remove");
				node.entry(value.as_str());
			}
			Self::CustomPronouns(value) => {
				node.entry("pronoun");
				node.entry("custom");
				node.entry(value.as_str());
			}
			Self::Height(value) => {
				node.entry("size");
				node.entry(("height", *value as i64));
			}
			Self::Weight(value) => {
				node.entry("size");
				node.entry(("weight", *value as i64));
			}
			Self::HeightWeight(height, weight) => {
				node.entry("size");
				node.entry(("height", *height as i64));
				node.entry(("weight", *weight as i64));
			}
			Self::InsertPersonality { kind, new } => {
				node.entry("personality");
				node.entry(kind.to_string());
				node.entry("insert");
				node.child(("new", new.as_str()));
			}
			Self::RemovePersonality { kind, idx, old } => {
				node.entry("personality");
				node.entry(kind.to_string());
				node.entry("remove");
				node.entry(*idx as i64);
				node.child(("old", old.as_str()));
			}
			Self::UpdatePersonality { kind, idx, old, new } => {
				node.entry("personality");
				node.entry(kind.to_string());
				node.entry("update");
				node.entry(*idx as i64);
				node.child(("old", old.as_str()));
				node.child(("new", new.as_str()));
			}
		}
		node
	}
}
