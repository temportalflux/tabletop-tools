use super::{Feature, StatsBuilder};
use crate::system::dnd5e::modifier;

#[derive(Default, Clone, PartialEq)]
pub struct Upbringing {
	pub name: String,
	pub description: String,
	pub features: Vec<Feature>,
}

impl modifier::Container for Upbringing {
	fn id(&self) -> String {
		use convert_case::Casing;
		self.name.to_case(convert_case::Case::Pascal)
	}

	fn apply_modifiers<'c>(&self, stats: &mut StatsBuilder<'c>) {
		for feat in &self.features {
			stats.apply_from(feat);
		}
	}
}