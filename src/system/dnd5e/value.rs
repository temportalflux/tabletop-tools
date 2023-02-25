use super::{data::character::Character, evaluator::BoxedEvaluator};
use std::rc::Rc;

#[derive(Clone)]
pub enum Value<T> {
	Fixed(T),
	Evaluated(BoxedEvaluator<T>),
}

impl<T> Default for Value<T>
where
	T: Default,
{
	fn default() -> Self {
		Self::Fixed(T::default())
	}
}

impl<T> PartialEq for Value<T>
where
	T: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Fixed(a), Self::Fixed(b)) => a == b,
			(Self::Evaluated(a), Self::Evaluated(b)) => Rc::ptr_eq(a, b),
			_ => false,
		}
	}
}

impl<T> std::fmt::Debug for Value<T>
where
	T: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Fixed(value) => write!(f, "Value::Fixed({value:?})"),
			Self::Evaluated(_eval) => write!(f, "Value::Evaluated(?)"),
		}
	}
}

impl<T> Value<T> {
	pub fn evaluate(&self, state: &Character) -> T
	where
		T: Clone,
	{
		match self {
			Self::Fixed(value) => value.clone(),
			Self::Evaluated(evaluator) => evaluator.evaluate(state),
		}
	}
}