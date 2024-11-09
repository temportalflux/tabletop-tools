use crate::system::{dnd5e::generator::item, mutator::ReferencePath};
use multimap::MultiMap;

// Tags granted by mutators that can be assigned by users to specific objects, such as items in the inventory.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct UserTags {
	tags: Vec<UserTag>,
	usages: MultiMap<String, ReferencePath>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct UserTag {
	pub tag: String,
	// how many objects this tag can be applied to
	pub max_count: Option<usize>,
	pub filter: Option<item::Filter>,
	pub source: ReferencePath,
}

impl UserTags {
	pub fn push(&mut self, tag: UserTag) {
		self.tags.push(tag);
	}

	pub fn add_tag_usage(&mut self, tag: &String, usage: &ReferencePath) {
		self.usages.insert(tag.clone(), usage.clone());
	}

	pub fn tags(&self) -> &Vec<UserTag> {
		&self.tags
	}

	pub fn usages_of(&self, tag: impl AsRef<str>) -> Option<&Vec<ReferencePath>> {
		self.usages.get_vec(tag.as_ref())
	}
}
