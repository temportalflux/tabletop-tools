use kdlize::{AsKdl, NodeId};

mod factory;
pub use factory::*;
mod registry;
pub use registry::*;

/// A change made to a character's persistent data. This is an exhaustive list of all the possible mutations
/// that can be made, and systems should never modify a character's persistent data directly.
///
/// While modifying data directly is not app-breaking, have an exhaustive serializable list of changes
/// allows for features like user-displayed changelogs and diffs.
pub trait Change: NodeId + AsKdl + std::fmt::Debug {
	type Target;

	fn apply_to(&self, target: &mut Self::Target);
}
