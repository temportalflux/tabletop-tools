use std::sync::Arc;

use super::Record;

pub mod entry;
pub use entry::Entry;
pub mod module;
pub use module::Module;
mod query;
pub use query::*;
mod schema;
pub use schema::*;

#[derive(Clone, PartialEq)]
pub struct Database(super::Client);

impl Database {
	pub async fn open() -> Result<Self, super::Error> {
		let client = super::Client::open::<SchemaVersion>("tabletop-tools").await?;
		Ok(Self(client))
	}

	pub fn write(&self) -> Result<idb::Transaction, super::Error> {
		Ok(self.0.transaction(
			&[Entry::store_id(), Module::store_id()],
			idb::TransactionMode::ReadWrite,
		)?)
	}

	pub fn read_entries(&self) -> Result<idb::Transaction, super::Error> {
		Ok(self.0.read_only::<Entry>()?)
	}

	pub fn write_entries(&self) -> Result<idb::Transaction, super::Error> {
		Ok(self.0.read_write::<Entry>()?)
	}

	pub fn read_modules(&self) -> Result<idb::Transaction, super::Error> {
		Ok(self.0.read_only::<Module>()?)
	}

	pub fn write_modules(&self) -> Result<idb::Transaction, super::Error> {
		Ok(self.0.read_write::<Module>()?)
	}

	fn read_index<I: super::IndexType>(&self) -> Result<super::Index<I>, super::Error> {
		use super::{ObjectStoreExt, TransactionExt};
		let transaction = self.read_entries()?;
		let entries_store = transaction.object_store_of::<I::Record>()?;
		entries_store.index_of::<I>()
	}

	pub async fn get<T>(
		&self,
		key: impl Into<wasm_bindgen::JsValue>,
	) -> Result<Option<T>, super::Error>
	where
		T: Record + serde::de::DeserializeOwned,
	{
		use super::{ObjectStoreExt, TransactionExt};
		let transaction = self.0.read_only::<T>()?;
		let store = transaction.object_store_of::<T>()?;
		Ok(store.get_record(key).await?)
	}

	pub async fn get_typed_entry<T>(
		&self,
		key: crate::system::core::SourceId,
		system_depot: crate::system::Depot,
	) -> Result<Option<T>, super::Error>
	where
		T: crate::kdl_ext::KDLNode
			+ crate::kdl_ext::FromKDL
			+ crate::system::dnd5e::SystemComponent
			+ Unpin,
	{
		use crate::system::core::System;
		let Some(entry) = self.get::<Entry>(key.to_string()).await? else { return Ok(None); };
		// Parse the entry's kdl string:
		// kdl string to document
		let Ok(document) = entry.kdl.parse::<kdl::KdlDocument>() else { return Ok(None); };
		// document to node
		let Some(node) = document.nodes().get(key.node_idx) else { return Ok(None); };
		// node to value based on the expected type
		let node_reg = {
			let system_reg = system_depot
				.get(crate::system::dnd5e::DnD5e::id())
				.expect("Missing system {system:?} in depot");
			system_reg.node()
		};
		let mut ctx = crate::kdl_ext::NodeContext::new(Arc::new(entry.source_id(true)), node_reg);
		let Ok(value) = T::from_kdl(node, &mut ctx) else { return Ok(None); };
		Ok(Some(value))
	}

	pub async fn query_entries(
		&self,
		system: impl Into<String>,
		category: impl Into<String>,
		criteria: Option<Box<Criteria>>,
	) -> Result<Query, super::Error> {
		let idx_by_sys_cate = self.read_index::<entry::SystemCategory>();
		let index = entry::SystemCategory {
			system: system.into(),
			category: category.into(),
		};
		let cursor = idx_by_sys_cate?.open_cursor(Some(&index)).await?;
		Ok(Query { cursor, criteria })
	}

	pub async fn query_typed<Output>(
		self,
		system: impl Into<String>,
		system_depot: crate::system::Depot,
		criteria: Option<Box<Criteria>>,
	) -> Result<QueryDeserialize<Output>, super::Error>
	where
		Output: crate::kdl_ext::KDLNode
			+ crate::kdl_ext::FromKDL
			+ crate::system::dnd5e::SystemComponent
			+ Unpin,
	{
		let system = system.into();
		let node_reg = {
			let system_reg = system_depot
				.get(&system)
				.expect("Missing system {system:?} in depot");
			system_reg.node()
		};
		let idx_by_sys_cate = self.read_index::<entry::SystemCategory>();
		let index = entry::SystemCategory {
			system,
			category: Output::id().into(),
		};
		let cursor = idx_by_sys_cate?.open_cursor(Some(&index)).await?;
		let query_typed = QueryDeserialize::<Output> {
			db: self,
			query: Query { cursor, criteria },
			node_reg,
			marker: Default::default(),
		};
		Ok(query_typed)
	}
}

impl std::ops::Deref for Database {
	type Target = super::Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
