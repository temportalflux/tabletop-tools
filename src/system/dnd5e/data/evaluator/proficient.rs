use crate::system::dnd5e::{
	data::{character::Character, WeaponProficiency},
	evaluator::Evaluator,
};

#[derive(Clone, PartialEq)]
pub enum IsProficientWith {
	Weapon(WeaponProficiency),
}

impl Evaluator for IsProficientWith {
	type Item = bool;

	fn evaluate(&self, state: &Character) -> Self::Item {
		match self {
			Self::Weapon(proficiency) => state
				.other_proficiencies()
				.weapons
				.contains_key(proficiency),
		}
	}
}