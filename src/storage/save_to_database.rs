use crate::{
	database::Database,
	system::{ModuleId, SourceId},
};

pub struct SaveToDatabase {
	pub database: Database,
	pub id: SourceId,
	pub category: String,
	pub metadata: serde_json::Value,
	pub document: String,
	pub file_id: Option<String>,
	pub version: String,
}

impl SaveToDatabase {
	pub async fn execute(self) -> Result<(), anyhow::Error> {
		use crate::database::{Entry, Module};
		use database::{ObjectStoreExt, TransactionExt};

		let entry = Entry {
			id: self.id.to_string(),
			module: self.id.module.as_ref().map(ModuleId::to_string).unwrap(),
			system: self.id.system.clone().unwrap(),
			category: self.category,
			version: Some(self.version.clone()),
			metadata: self.metadata,
			kdl: self.document,
			file_id: self.file_id,
			generator_id: None,
			generated: 0,
		};

		let transaction = self.database.write()?;
		let module_store = transaction.object_store_of::<Module>()?;
		let entry_store = transaction.object_store_of::<Entry>()?;

		let module_req = module_store.get_record::<Module>(entry.module.clone());
		let mut module = module_req.await?.unwrap();
		module.version = self.version;
		module_store.put_record(&module).await?;

		entry_store.put_record(&entry).await?;

		transaction.commit().await?;

		Ok(())
	}
}
