use super::{Character, ObjectCacheProvider};
use crate::system::{
	dnd5e::data::Condition,
	mutator::{self, ReferencePath},
	SourceId,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub enum IdOrIndex {
	Id(Arc<SourceId>),
	Index(usize),
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Conditions {
	by_id: BTreeMap<SourceId, Condition>,
	custom: Vec<Condition>,
}

impl Conditions {
	pub async fn resolve_indirection(&mut self, provider: &ObjectCacheProvider) -> anyhow::Result<()> {
		for condition in self.iter_mut() {
			condition.resolve_indirection(provider).await?;
		}
		Ok(())
	}

	pub fn insert(&mut self, condition: Condition) {
		match &condition.id {
			Some(id) => {
				self.by_id.insert(id.unversioned(), condition);
			}
			None => {
				self.custom.push(condition);
				self.custom.sort_by(|a, b| a.name.cmp(&b.name));
			}
		}
	}

	pub fn remove_by_id(&mut self, id: &SourceId) {
		self.by_id.remove(&*id);
	}

	pub fn remove_custom(&mut self, idx: usize) {
		self.custom.remove(idx);
	}

	pub fn iter(&self) -> impl Iterator<Item = &Condition> {
		self.by_id.values().chain(self.custom.iter())
	}

	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Condition> {
		self.by_id.values_mut().chain(self.custom.iter_mut())
	}

	pub fn iter_keyed(&self) -> impl Iterator<Item = (IdOrIndex, &Condition)> {
		let ids = self.by_id.iter().map(|(id, value)| (IdOrIndex::Id(Arc::new(id.clone())), value));
		let indices = self.custom.iter().enumerate().map(|(idx, value)| (IdOrIndex::Index(idx), value));
		ids.chain(indices)
	}

	pub fn contains_id(&self, id: &SourceId) -> bool {
		self.by_id.contains_key(id)
	}
}

impl mutator::Group for Conditions {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		for condition in self.iter() {
			condition.set_data_path(parent);
		}
	}

	fn apply_mutators(&self, target: &mut Self::Target, parent: &ReferencePath) {
		for condition in self.iter() {
			target.apply_from(condition, parent);
		}
	}
}
