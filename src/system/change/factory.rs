use super::Change;
use crate::{
	kdl_ext::{NodeContext, NodeReader},
	utility::BoxAny,
};
use kdlize::FromKdl;
use std::any::TypeId;

/// A factory which parses a block (root-level kdl node) into some concrete type, and exposes methods for calling
/// specific functions on that type (e.g. reserializing into text).
pub struct Factory {
	type_name: &'static str,
	target_type_info: (TypeId, &'static str),
	fn_from_kdl: Box<dyn Fn(&mut NodeReader<'_>) -> anyhow::Result<BoxAny> + 'static + Send + Sync>,
}
impl Factory {
	pub(super) fn new<T>() -> Self
	where
		T: Change + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<T::Error>,
	{
		Self {
			type_name: std::any::type_name::<T>(),
			target_type_info: (TypeId::of::<T::Target>(), std::any::type_name::<T::Target>()),
			fn_from_kdl: Box::new(|node| {
				let impl_type = T::from_kdl(node)?;
				Ok(Box::new(impl_type))
			}),
		}
	}

	pub fn from_kdl<'doc, T>(&self, node: &mut NodeReader<'doc>) -> anyhow::Result<Box<T>>
	where
		T: 'static,
	{
		if TypeId::of::<T>() != self.target_type_info.0 {
			return Err(crate::utility::IncompatibleTypes(
				"target",
				self.type_name,
				self.target_type_info.1,
				std::any::type_name::<T>(),
			)
			.into());
		}
		let any = (self.fn_from_kdl)(node)?;
		let downcasted_t = any.downcast::<T>().expect("failed to unpack boxed change");
		Ok(downcasted_t)
	}
}
