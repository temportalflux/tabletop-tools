use super::System;
use std::{collections::HashMap, sync::Arc};

mod builder;
pub use builder::*;
mod entry;
pub use entry::*;

#[derive(Clone)]
pub struct Registry(Arc<HashMap<&'static str, Entry>>);

impl PartialEq for Registry {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Registry {
	pub fn builder() -> Builder {
		Builder::new()
	}

	pub(in super::registry) fn new(systems: HashMap<&'static str, Entry>) -> Self {
		Self(Arc::new(systems))
	}

	pub fn iter_ids(&self) -> impl Iterator<Item = &str> + '_ {
		self.0.keys().map(|str| *str)
	}

	pub fn get_sys<T: System>(&self) -> Option<&Entry> {
		self.get(T::id())
	}

	pub fn get(&self, system_id: &str) -> Option<&Entry> {
		self.0.get(system_id)
	}

	pub fn make_node_context(&self, id: Arc<super::SourceId>) -> Option<crate::kdl_ext::NodeContext> {
		let system = id.system.as_ref()?;
		let entry = self.get(system)?;
		Some(crate::kdl_ext::NodeContext::new(id, entry.node()))
	}
}
