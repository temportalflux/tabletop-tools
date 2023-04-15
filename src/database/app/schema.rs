use crate::database::{app, Error, MissingVersion, ObjectStoreExt, Record, Schema};

/// The schema for the `tabletop-tools` client database.
/// Use with `Client::open`.
pub enum SchemaVersion {
	Version1 = 1,
}

impl TryFrom<u32> for SchemaVersion {
	type Error = MissingVersion;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(Self::Version1),
			_ => Err(MissingVersion(value)),
		}
	}
}

impl Schema for SchemaVersion {
	fn latest() -> u32 {
		Self::Version1 as u32
	}

	fn apply(&self, database: &idb::Database) -> Result<(), Error> {
		match self {
			Self::Version1 => {
				// Create modules table
				{
					use app::module::Module;
					let mut params = idb::ObjectStoreParams::new();
					params.auto_increment(true);
					params.key_path(Some(idb::KeyPath::new_single("id")));
					let _store = database.create_object_store(Module::store_id(), params)?;
				}
				// Create entries table
				{
					use app::entry::{Entry, ModuleSystem, SystemCategory};
					let mut params = idb::ObjectStoreParams::new();
					params.auto_increment(true);
					params.key_path(Some(idb::KeyPath::new_single("id")));
					let store = database.create_object_store(Entry::store_id(), params)?;
					store.create_index_of::<ModuleSystem>(None)?;
					store.create_index_of::<SystemCategory>(None)?;
				}
			}
		}
		Ok(())
	}
}
