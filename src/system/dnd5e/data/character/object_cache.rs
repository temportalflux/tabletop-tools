use crate::{
	database::{Criteria, Entry, FetchError, Query},
	system::{Block, SourceId},
};
use std::{
	any::Any,
	collections::HashMap,
	sync::{Arc, RwLock},
};

type ObjectCacheData = HashMap<SourceId, Box<dyn Any>>;
#[derive(Default, Clone)]
pub struct ObjectCacheArc(Arc<RwLock<ObjectCacheData>>);
impl PartialEq for ObjectCacheArc {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

#[derive(Clone)]
pub struct ObjectCacheProvider {
	// TODO: Decouple database+system_depot from dnd data - there can be an abstraction/trait
	// which specifies the minimal API for getting a cached object from a database of content
	pub database: crate::database::Database,
	pub system_depot: crate::system::Registry,
	// TODO: Will need at some point to have a timer to clear stale entries
	// (though thats equivalent to reloading the webpage in its current form, because the character is reloaded from scratch).
	pub object_cache: ObjectCacheArc,
}

impl ObjectCacheProvider {
	pub fn new(
		database: &crate::database::Database, system_depot: &crate::system::Registry, object_cache: &ObjectCacheArc,
	) -> Self {
		Self { database: database.clone(), system_depot: system_depot.clone(), object_cache: object_cache.clone() }
	}

	pub async fn get_typed_entry<T>(
		&self, key: crate::system::SourceId, criteria: Option<Criteria>,
	) -> Result<Option<T>, FetchError>
	where
		T: Block + Unpin + 'static,
	{
		let query = Query::<Entry>::single(&self.database, &key).await?;
		let query = query.apply_opt(criteria, Query::filter_by);
		let mut query = query.parse_as::<T>(&self.system_depot);
		let Some((_entry, typed)) = query.next().await else {
			return Ok(None);
		};
		Ok(Some(typed))
	}

	pub fn writable_cache(&self) -> MutableObjectCache<'_> {
		self.object_cache.writable_cache()
	}
}

impl ObjectCacheArc {
	pub fn writable_cache(&self) -> MutableObjectCache<'_> {
		MutableObjectCache(self.0.write().unwrap())
	}
}

pub struct MutableObjectCache<'provider>(std::sync::RwLockWriteGuard<'provider, HashMap<SourceId, Box<dyn Any>>>);

impl<'provider> MutableObjectCache<'provider> {
	pub fn get<T>(&self, id: &SourceId) -> Option<&T>
	where
		T: 'static,
	{
		let Some(object) = self.0.get(id) else { return None };
		object.downcast_ref()
	}

	pub fn insert<T>(&mut self, id: SourceId, object: T)
	where
		T: 'static,
	{
		self.0.insert(id, Box::new(object));
	}
}
