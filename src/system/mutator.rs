use crate::{
	system::dnd5e::data::description,
	utility::{AsTraitEq, Dependencies, PathExt, TraitEq},
};
use kdlize::{AsKdl, NodeId};
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

mod factory;
pub use factory::*;
mod generic;
pub use generic::*;

pub type ArcMutator<T> = Arc<dyn Mutator<Target = T> + 'static + Send + Sync>;

pub trait Mutator: std::fmt::Debug + TraitEq + AsTraitEq<dyn TraitEq> + NodeId + AsKdl {
	type Target;

	fn dependencies(&self) -> Dependencies {
		Dependencies::default()
	}

	fn set_data_path(&self, _parent: &ReferencePath) {}

	fn description(&self, _state: Option<&Self::Target>) -> description::Section {
		description::Section::default()
	}

	fn on_insert(&self, _: &mut Self::Target, _parent: &ReferencePath) {}

	fn apply(&self, _: &mut Self::Target, _parent: &ReferencePath) {}
}

pub trait Group {
	type Target;

	fn set_data_path(&self, parent: &ReferencePath);

	fn apply_mutators(&self, target: &mut Self::Target, parent: &ReferencePath);
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct ReferencePath {
	pub data: PathBuf,
	pub display: PathBuf,
}
impl ReferencePath {
	pub fn new() -> Self {
		Self { data: PathBuf::new(), display: PathBuf::new() }
	}
	
	pub fn new_path<Subpath>(data: Subpath, display: Option<Subpath>) -> Self 
	where
		Subpath: AsRef<Path>,
		{
			Self {
				data: data.as_ref().to_owned(),
				display: match display {
					None => data.as_ref().to_owned(),
					Some(path) => path.as_ref().to_owned(),
				},
			}
	}

	pub fn join<Subpath>(&self, data: Subpath, display: Option<Subpath>) -> Self
	where
		Subpath: AsRef<Path>,
	{
		Self {
			data: self.data.join(data.as_ref()),
			display: match display {
				None => self.display.join(data.as_ref()),
				Some(path) => self.display.join(path.as_ref()),
			},
		}
	}

	pub fn normalized(&self) -> Self {
		let data = PathBuf::from(self.data.normalize().to_str().unwrap().replace("\\", "/"));
		let display = PathBuf::from(self.display.normalize().to_str().unwrap().replace("\\", "/"));
		Self { data, display }
	}
}

pub struct GroupIter<'owner, TargetType>(Vec<Box<&'owner dyn Group<Target = TargetType>>>);
impl<'owner, TargetType> Default for GroupIter<'owner, TargetType> {
	fn default() -> Self {
		Self(Vec::new())
	}
}
impl<'owner, TargetType> GroupIter<'owner, TargetType> {
	pub fn with<ItemType>(mut self, item: &'owner ItemType) -> Self
	where
		ItemType: 'owner + Group<Target = TargetType> + Sized,
	{
		let boxed = Box::<&'owner dyn Group<Target = TargetType>>::new(item);
		self.0.push(boxed);
		self
	}

	pub fn chain<IterType, ItemType>(mut self, iter: IterType) -> Self
	where
		IterType: Iterator<Item = &'owner ItemType>,
		ItemType: 'owner + Group<Target = TargetType> + Sized,
	{
		let iter = iter.map(|group| Box::<&'owner dyn Group<Target = TargetType>>::new(group));
		self.0.extend(iter);
		self
	}

	pub fn into_iter(self) -> impl Iterator<Item = Box<&'owner dyn Group<Target = TargetType>>> + 'owner {
		self.0.into_iter()
	}
}
