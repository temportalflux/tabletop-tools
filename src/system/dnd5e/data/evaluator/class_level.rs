use crate::{
	system::dnd5e::{data::character::Character, Value},
	utility::{Dependencies, Evaluator},
};
use std::{collections::BTreeMap, fmt::Debug, iter::Product};

#[derive(Clone, PartialEq, Default, Debug)]
pub struct GetLevel<T> {
	class_name: Option<String>,
	marker: std::marker::PhantomData<T>,
}
impl<T, S> From<Option<S>> for GetLevel<T>
where
	S: ToString,
	T: Default,
{
	fn from(value: Option<S>) -> Self {
		Self {
			class_name: value.map(|s| s.to_string()),
			marker: std::marker::PhantomData::default(),
		}
	}
}

impl<T> crate::utility::TraitEq for GetLevel<T>
where
	T: 'static + PartialEq,
{
	fn equals_trait(&self, other: &dyn crate::utility::TraitEq) -> bool {
		crate::utility::downcast_trait_eq(self, other)
	}
}

impl<T> Evaluator for GetLevel<T>
where
	T: 'static + Copy + Debug + Send + Sync + PartialEq,
	usize: num_traits::AsPrimitive<T>,
{
	type Context = Character;
	type Item = T;

	fn evaluate(&self, state: &Self::Context) -> Self::Item {
		use num_traits::AsPrimitive;
		let value = state
			.level(self.class_name.as_ref().map(String::as_str))
			.as_();
		value
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct GetAbilityModifier(pub crate::system::dnd5e::data::Ability);

impl crate::utility::TraitEq for GetAbilityModifier {
	fn equals_trait(&self, other: &dyn crate::utility::TraitEq) -> bool {
		crate::utility::downcast_trait_eq(self, other)
	}
}

impl Evaluator for GetAbilityModifier {
	type Context = Character;
	type Item = i32;

	fn dependencies(&self) -> Dependencies {
		["add_ability_score"].into()
	}

	fn evaluate(&self, state: &Self::Context) -> Self::Item {
		let value = state.ability_modifier(self.0, None);
		value
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct MulValues<T>(pub Vec<Value<T>>);

impl<T> crate::utility::TraitEq for MulValues<T>
where
	T: 'static + PartialEq,
{
	fn equals_trait(&self, other: &dyn crate::utility::TraitEq) -> bool {
		crate::utility::downcast_trait_eq(self, other)
	}
}

impl<T> Evaluator for MulValues<T>
where
	T: 'static + Product + Clone + Send + Sync + Debug + PartialEq,
{
	type Context = Character;
	type Item = T;

	fn dependencies(&self) -> Dependencies {
		self.0.iter().fold(Dependencies::default(), |deps, value| {
			deps.join(value.dependencies())
		})
	}

	fn evaluate(&self, state: &Self::Context) -> Self::Item {
		self.0.iter().map(|value| value.evaluate(state)).product()
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct ByLevel<T> {
	pub class_name: Option<String>,
	pub map: BTreeMap<usize, T>,
}

impl<T, const N: usize> From<[(usize, T); N]> for ByLevel<T> {
	fn from(value: [(usize, T); N]) -> Self {
		Self {
			class_name: None,
			map: BTreeMap::from(value),
		}
	}
}

impl<T> crate::utility::TraitEq for ByLevel<T>
where
	T: 'static + PartialEq,
{
	fn equals_trait(&self, other: &dyn crate::utility::TraitEq) -> bool {
		crate::utility::downcast_trait_eq(self, other)
	}
}

impl<T> Evaluator for ByLevel<T>
where
	T: 'static + Clone + Default + Send + Sync + Debug + PartialEq,
{
	type Context = Character;
	type Item = T;

	fn evaluate(&self, state: &Self::Context) -> Self::Item {
		let character_level = GetLevel::from(self.class_name.clone()).evaluate(state);
		for (level, value) in self.map.iter().rev() {
			if *level <= character_level {
				return value.clone();
			}
		}
		T::default()
	}
}
