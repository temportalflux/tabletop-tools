use super::{Change, Factory};
use crate::kdl_ext::NodeContext;
use kdlize::FromKdl;
use std::{collections::HashMap, sync::Arc};

/// A registry of all of the root-level nodes (aka blocks) which could be parsed from kdl.
#[derive(Default)]
pub struct Registry(HashMap<&'static str, Arc<Factory>>);
impl Registry {
	pub fn register<T>(&mut self)
	where
		T: Change + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<T::Error>,
	{
		assert!(!self.0.contains_key(T::id()));
		self.0.insert(T::id(), Factory::new::<T>().into());
	}

	pub fn get_factory(&self, id: &str) -> Option<&Arc<Factory>> {
		self.0.get(id)
	}
}
