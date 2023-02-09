use super::character::StatsBuilder;
use dyn_clone::{clone_trait_object, DynClone};

mod ability_score;
pub use ability_score::*;

mod description;
pub use description::*;

mod skill;
pub use skill::*;

mod language;
pub use language::*;

pub trait Modifier: DynClone {
	fn scope_id(&self) -> Option<&str> {
		None
	}
	fn apply<'c>(&self, _: &mut StatsBuilder<'c>) {}
}
clone_trait_object!(Modifier);

pub trait Container {
	fn id(&self) -> String;
	fn apply_modifiers<'c>(&self, stats: &mut StatsBuilder<'c>);
}

#[derive(Clone)]
pub enum Selector<T> {
	Specific(T),
	AnyOf { id: Option<String>, options: Vec<T> },
	Any { id: Option<String> },
}

impl<T> Selector<T> {
	pub fn id(&self) -> Option<&str> {
		match self {
			Self::Specific(_) => None,
			Self::AnyOf { id, options: _ } => id.as_ref(),
			Self::Any { id } => id.as_ref(),
		}
		.map(String::as_str)
	}
}
