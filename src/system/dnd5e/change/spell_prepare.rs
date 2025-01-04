use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{character::Character, Spell},
		Change, SourceId,
	},
	utility::NotInList,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub enum PrepareSpell {
	Add { caster: String, spell: Spell },
	Remove { caster: String, spell_id: SourceId },
}

crate::impl_trait_eq!(PrepareSpell);
kdlize::impl_kdl_node!(PrepareSpell, "prepare_spell");

impl Change for PrepareSpell {
	type Target = Character;

	fn apply_to(&self, character: &mut Self::Target) {
		match self {
			Self::Add { caster, spell } => {
				character.persistent_mut().selected_spells.insert(caster, spell.clone());
			}
			Self::Remove { caster, spell_id } => {
				character.persistent_mut().selected_spells.remove(caster, spell_id);
			}
		}
	}
}

impl FromKdl<NodeContext> for PrepareSpell {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		match node.next_str_req()? {
			"add" => {
				let caster = node.next_str_req()?.to_owned();
				let spell = Spell::from_kdl(node)?;
				Ok(Self::Add { caster, spell })
			}
			"remove" => {
				let caster = node.next_str_req()?.to_owned();
				let spell_id = node.next_str_req_t()?;
				Ok(Self::Remove { caster, spell_id })
			}
			s => Err(NotInList(s.to_owned(), vec!["add", "remove"]).into()),
		}
	}
}

impl AsKdl for PrepareSpell {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Add { caster, spell } => {
				node.entry("add");
				node.entry(caster.as_str());
				node += spell.as_kdl();
			}
			Self::Remove { caster, spell_id } => {
				node.entry("remove");
				node.entry(caster.as_str());
				node.entry(spell_id.to_string());
			}
		}
		node
	}
}
