use super::DerivedBuilder;
use crate::system::dnd5e::{mutator, BoxedFeature};

#[derive(Default, Clone, PartialEq)]
pub struct Upbringing {
	pub name: String,
	pub description: String,
	pub features: Vec<BoxedFeature>,
}

impl mutator::Container for Upbringing {
	fn id(&self) -> Option<String> {
		use convert_case::Casing;
		Some(self.name.to_case(convert_case::Case::Pascal))
	}

	fn apply_mutators<'c>(&self, stats: &mut DerivedBuilder<'c>) {
		for feat in &self.features {
			stats.add_feature(feat);
		}
	}
}
