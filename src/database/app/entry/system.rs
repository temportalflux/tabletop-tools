use crate::database::{app::Entry, Error, IndexType, QueryExt};

pub struct System {
	pub system: String,
}

impl IndexType for System {
	type Record = Entry;

	fn name() -> &'static str {
		"system"
	}

	fn keys() -> &'static [&'static str] {
		&["system"]
	}

	fn as_query(&self) -> Result<idb::Query, Error> {
		idb::Query::from_items([&self.system])
	}
}