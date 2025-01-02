use crate::system::Block;
use database::{Error, Record, Transaction};
use futures_util::future::LocalBoxFuture;

pub mod entry;
pub use entry::Entry;
pub mod module;
pub use module::Module;
mod query;
pub use query::*;
mod schema;
pub use schema::*;
mod settings;
pub use settings::*;

#[derive(Clone, PartialEq)]
pub struct Database(database::Client);

impl Database {
	pub async fn open() -> Result<Self, Error> {
		let client = database::Client::open::<SchemaVersion>("tabletop-tools").await?;
		Ok(Self(client))
	}

	pub fn write(&self) -> Result<Transaction, Error> {
		self.0.transaction(&SchemaVersion::store_ids(), idb::TransactionMode::ReadWrite)
	}

	pub fn read(&self) -> Result<Transaction, Error> {
		self.0.transaction(&SchemaVersion::store_ids(), idb::TransactionMode::ReadOnly)
	}

	pub fn read_entries(&self) -> Result<Transaction, Error> {
		self.0.read_only::<Entry>()
	}

	pub fn write_entries(&self) -> Result<Transaction, Error> {
		self.0.read_write::<Entry>()
	}

	pub fn read_modules(&self) -> Result<Transaction, Error> {
		self.0.read_only::<Module>()
	}

	pub fn write_modules(&self) -> Result<Transaction, Error> {
		self.0.read_write::<Module>()
	}

	pub async fn clear(&self) -> Result<(), Error> {
		use database::TransactionExt;
		let transaction = self.write()?;
		transaction.object_store_of::<Module>()?.clear()?.await?;
		transaction.object_store_of::<Entry>()?.clear()?.await?;
		transaction.object_store_of::<UserSettingsRecord>()?.clear()?.await?;
		transaction.commit().await?;
		Ok(())
	}

	pub async fn get<T>(&self, key: impl Into<wasm_bindgen::JsValue>) -> Result<Option<T>, Error>
	where
		T: Record + serde::de::DeserializeOwned,
	{
		self.0.get::<T>(key).await
	}

	pub async fn get_typed_entry<T>(
		&self, key: crate::system::SourceId, system_depot: crate::system::Registry, criteria: Option<Criteria>,
	) -> Result<Option<T>, FetchError>
	where
		T: Block + Unpin + 'static,
		T::Error: std::fmt::Debug,
	{
		let query = Query::<Entry>::single(&self, &key).await?;
		let query = query.apply_opt(criteria, Query::filter_by);
		let mut query = query.parse_as::<T>(&system_depot);
		let Some((_entry, typed)) = query.next().await else {
			return Ok(None);
		};
		Ok(Some(typed))
	}

	pub async fn mutate<F, Output>(&self, fn_transaction: F) -> Result<Output, Error>
	where
		F: FnOnce(&database::Transaction) -> LocalBoxFuture<'_, Result<Output, Error>>,
	{
		let transaction = self.write()?;
		let output = fn_transaction(&transaction).await?;
		transaction.commit().await?;
		Ok(output)
	}
}

impl std::ops::Deref for Database {
	type Target = database::Client;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum FetchError {
	#[error(transparent)]
	FindEntry(#[from] Error),
	#[error(transparent)]
	InvalidDocument(#[from] kdl::KdlError),
	#[error("Entry document is empty")]
	EmptyDocument,
	#[error("Entry document has too many nodes (should only be 1 per entry): {0:?}")]
	TooManyDocNodes(String),
	#[error("Failed to parse node as a {1:?}: {0:?}")]
	FailedToParse(String, &'static str),
}
