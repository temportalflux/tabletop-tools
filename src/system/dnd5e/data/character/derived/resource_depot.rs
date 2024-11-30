use crate::system::dnd5e::data::Resource;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

#[derive(Clone, PartialEq, Debug, Default)]
pub struct ResourceDepot {
	uses: HashMap<PathBuf, Resource>,
}

impl ResourceDepot {
	pub fn register(&mut self, resource: &Resource) {
		//log::debug!(target: "resource", "register {resource:?} @ {:?}", resource.get_uses_path());
		let Some(path) = resource.get_uses_path() else { return };
		self.uses.insert(path, resource.clone());
	}

	pub fn get(&self, key: impl AsRef<Path>) -> Option<&Resource> {
		self.uses.get(key.as_ref())
	}
}
