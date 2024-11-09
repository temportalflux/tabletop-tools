use super::{entry::Entry, Database};
use crate::system::dnd5e::data::character::ObjectCacheArc;
use database::{IndexType, Record};
use futures::{stream::LocalBoxStream, Future, StreamExt};

mod criteria;
pub use criteria::*;
mod old;
pub use old::*;

/// Handles querying the database using some user-defined criteria, filtering, and mapping.
pub struct Query<Output>(LocalBoxStream<'static, Output>);

impl<Output> std::ops::Deref for Query<Output> {
	type Target = LocalBoxStream<'static, Output>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<Output> std::ops::DerefMut for Query<Output> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<Output> Query<Output>
where
	Output: 'static,
{
	pub async fn single<Key>(database: &Database, key: &Key) -> Result<Self, database::Error>
	where
		Output: Record + serde::de::DeserializeOwned,
		Key: 'static + ToString,
	{
		Self::multiple(database, vec![key]).await
	}

	pub async fn multiple<'iter, Iter, Key>(database: &Database, iter: Iter) -> Result<Self, database::Error>
	where
		Output: Record + serde::de::DeserializeOwned,
		Iter: IntoIterator<Item = &'iter Key>,
		Key: 'static + ToString,
	{
		let mut found = Vec::new();
		for key in iter {
			if let Some(value) = database.get::<Output>(key.to_string()).await? {
				found.push(value);
			}
		}
		Ok(Self(futures_util::stream::iter(found).boxed_local()))
	}

	pub async fn subset<Index>(database: &Database, index: Option<Index>) -> Result<Self, database::Error>
	where
		Index: IndexType<Record = Output>,
		Output: Record + serde::de::DeserializeOwned + Unpin,
	{
		use database::{ObjectStoreExt, TransactionExt};
		let transaction = database.read_only::<Index::Record>()?;
		let object_store = transaction.object_store_of::<Index::Record>()?;
		let index_table = object_store.index_of::<Index>()?;
		let cursor = index_table.open_cursor(index.as_ref()).await?;
		Ok(Self(cursor.boxed_local()))
	}

	pub async fn all(database: &Database) -> Result<Self, database::Error>
	where
		Output: Record + serde::de::DeserializeOwned + Unpin,
	{
		use database::{ObjectStoreExt, TransactionExt};
		let transaction = database.read_only::<Output>()?;
		let object_store = transaction.object_store_of::<Output>()?;
		let cursor = object_store.cursor_all::<Output>().await?;
		Ok(Self(cursor.boxed_local()))
	}

	pub fn filter<F, Fut>(self, predicate: F) -> Self
	where
		F: 'static + FnMut(&Output) -> Fut,
		Fut: 'static + Future<Output = bool>,
	{
		Self(self.0.filter(predicate).boxed_local())
	}

	pub fn map<F, T>(self, predicate: F) -> Query<T>
	where
		T: 'static,
		F: 'static + FnMut(Output) -> T,
	{
		Query(self.0.map(predicate).boxed_local())
	}

	pub fn take(self, n: usize) -> Self {
		Self(self.0.take(n).boxed_local())
	}

	pub fn apply_opt<Arg, Operation>(self, arg: Option<Arg>, operation: Operation) -> Self
	where
		Operation: FnOnce(Self, Arg) -> Self,
	{
		match arg {
			None => self,
			Some(arg) => operation(self, arg),
		}
	}

	pub async fn next(&mut self) -> Option<Output> {
		self.0.next().await
	}

	pub async fn collect<C: Default + Extend<Output>>(self) -> C {
		self.0.collect::<C>().await
	}

	pub fn into_inner(self) -> LocalBoxStream<'static, Output> {
		self.0
	}
}

impl Query<Entry> {
	pub fn filter_by(self, criteria: Criteria) -> Self {
		self.filter(criteria.into_predicate())
	}

	pub fn parse_as<BlockType>(self, registry: &crate::system::Registry) -> Query<(Entry, BlockType)>
	where
		BlockType: crate::system::Block,
	{
		let registry = registry.clone();
		Query(
			self.0
				.filter_map(move |entry| {
					let generics = registry.get(&entry.system).map(|reg| reg.node());
					async move {
						let block = entry.parse_kdl::<BlockType>(generics?)?;
						Some((entry, block))
					}
				})
				.boxed_local(),
		)
	}

	pub fn parse_as_cached<BlockType>(
		self, registry: &crate::system::Registry, cache: &ObjectCacheArc,
	) -> Query<(Entry, BlockType)>
	where
		BlockType: crate::system::Block + Clone + 'static,
	{
		let registry = registry.clone();
		let cache = cache.clone();
		let stream = self.0.filter_map(move |entry| {
			let registry = registry.clone();
			let cache = cache.clone();
			async move {
				let entry_id = entry.source_id(true);
				let mut mutable_cache = cache.writable_cache();
				match mutable_cache.get::<BlockType>(&entry_id) {
					Some(block) => Some((entry, block.clone())),
					None => {
						let generics = registry.get(&entry.system).map(|reg| reg.node());
						let block = entry.parse_kdl::<BlockType>(generics?)?;
						mutable_cache.insert(entry_id, block.clone());
						Some((entry, block))
					}
				}
			}
		});
		Query(stream.boxed_local())
	}
}
