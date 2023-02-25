use crate::{system::dnd5e::data::character::Character, utility::Mutator};

#[derive(Clone)]
pub struct AddMaxSpeed(pub String, pub i32);

impl Mutator for AddMaxSpeed {
	type Target = Character;

	fn node_id(&self) -> &'static str {
		"add_max_speed"
	}

	fn apply<'c>(&self, stats: &mut Character) {
		let source = stats.source_path();
		stats.speeds_mut().push_max(self.0.clone(), self.1, source);
	}
}

#[derive(Clone)]
pub struct AddMaxSense(pub String, pub i32);

impl Mutator for AddMaxSense {
	type Target = Character;

	fn node_id(&self) -> &'static str {
		"add_max_sense"
	}

	fn apply<'c>(&self, stats: &mut Character) {
		let source = stats.source_path();
		stats.senses_mut().push_max(self.0.clone(), self.1, source);
	}
}
