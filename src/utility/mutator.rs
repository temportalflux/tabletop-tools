use super::{AsTraitEq, Dependencies, TraitEq};
use std::{fmt::Debug, sync::Arc};

pub trait Mutator: Debug + TraitEq + AsTraitEq<dyn TraitEq> {
	type Target;

	fn get_node_name(&self) -> &'static str;

	fn dependencies(&self) -> Dependencies {
		Dependencies::default()
	}

	fn data_id(&self) -> Option<&str> {
		None
	}

	fn apply<'c>(&self, _: &mut Self::Target) {}
}

#[derive(Clone)]
pub struct ArcMutator<T>(Arc<dyn Mutator<Target = T> + 'static + Send + Sync>);
impl<T> PartialEq for ArcMutator<T>
where
	T: 'static,
{
	fn eq(&self, other: &Self) -> bool {
		self.0.equals_trait((*other.0).as_trait_eq())
	}
}
impl<T> std::ops::Deref for ArcMutator<T> {
	type Target = Arc<dyn Mutator<Target = T> + 'static + Send + Sync>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<M, T> From<M> for ArcMutator<T>
where
	M: Mutator<Target = T> + 'static + Send + Sync,
{
	fn from(value: M) -> Self {
		Self(Arc::new(value))
	}
}
impl<T> std::fmt::Debug for ArcMutator<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

pub trait MutatorGroup {
	type Target;

	fn display_id(&self) -> bool {
		true
	}

	fn id(&self) -> Option<String> {
		None
	}

	fn apply_mutators<'c>(&self, target: &mut Self::Target);
}
