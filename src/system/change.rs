use crate::utility::{AsTraitEq, TraitEq};

mod factory;
pub use factory::*;
mod registry;
pub use registry::*;

/// A change made to a character's persistent data. This is an exhaustive list of all the possible mutations
/// that can be made, and systems should never modify a character's persistent data directly.
///
/// While modifying data directly is not app-breaking, have an exhaustive serializable list of changes
/// allows for features like user-displayed changelogs and diffs.
pub trait Change: kdlize::NodeId + kdlize::AsKdl + std::fmt::Debug + TraitEq + AsTraitEq<dyn TraitEq> {
	type Target;

	fn apply_to(&self, target: &mut Self::Target);
}

pub type ArcChange<T> = std::sync::Arc<dyn Change<Target = T> + 'static + Send + Sync>;

#[derive(Clone)]
pub struct Generic<T>(ArcChange<T>);

impl<C, T> From<C> for Generic<T>
where
	C: Change<Target = T> + 'static + Send + Sync,
{
	fn from(value: C) -> Self {
		Self(std::sync::Arc::new(value))
	}
}

impl<T> Generic<T> {
	pub(super) fn new(inner: ArcChange<T>) -> Self {
		Self(inner)
	}
}

impl<T> PartialEq for Generic<T>
where
	T: 'static,
{
	fn eq(&self, other: &Self) -> bool {
		self.0.equals_trait((*other.0).as_trait_eq())
	}
}

impl<T> std::ops::Deref for Generic<T> {
	type Target = ArcChange<T>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<T> std::ops::DerefMut for Generic<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<T> std::fmt::Debug for Generic<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<T> kdlize::AsKdl for Generic<T> {
	fn as_kdl(&self) -> crate::kdl_ext::NodeBuilder {
		let mut node = crate::kdl_ext::NodeBuilder::default();
		node.entry(self.0.get_id());
		node += self.0.as_kdl();
		node
	}
}

impl<T> kdlize::FromKdl<crate::kdl_ext::NodeContext> for Generic<T>
where
	T: 'static,
{
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let id = node.next_str_req()?;
		let node_reg = node.context().node_reg().clone();
		let factory = node_reg.get_change_factory(id)?;
		factory.from_kdl::<T>(node)
	}
}
