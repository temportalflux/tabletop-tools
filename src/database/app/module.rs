use crate::{database::Record, system::core::ModuleId};
use serde::{Deserialize, Serialize};

mod name_system;
pub use name_system::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct Module {
	pub module_id: ModuleId,
	pub name: String,
	pub system: String,
}

impl Record for Module {
	fn store_id() -> &'static str {
		"modules"
	}
}
