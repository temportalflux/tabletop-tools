use super::NodeExt;
use crate::{
	system::core::{NodeRegistry, SourceId},
	utility::{GenericEvaluator, GenericMutator},
};
use std::sync::Arc;

#[derive(Default)]
pub struct NodeContext {
	root_id: Arc<SourceId>,
	index_cursor: usize,
	node_registry: Arc<NodeRegistry>,
}

impl NodeContext {
	pub fn new(id: Arc<SourceId>, registry: Arc<NodeRegistry>) -> Self {
		Self {
			root_id: id,
			node_registry: registry,
			index_cursor: 0,
		}
	}

	#[cfg(test)]
	pub fn registry(registry: NodeRegistry) -> Self {
		Self {
			node_registry: Arc::new(registry),
			..Default::default()
		}
	}

	pub fn id(&self) -> &SourceId {
		&*self.root_id
	}

	pub fn peak_idx(&self) -> usize {
		self.index_cursor
	}

	pub fn consume_idx(&mut self) -> usize {
		let consumed = self.index_cursor;
		self.index_cursor += 1;
		consumed
	}

	pub fn next_node(&self) -> Self {
		Self {
			root_id: self.root_id.clone(),
			index_cursor: 0,
			node_registry: self.node_registry.clone(),
		}
	}

	pub fn node_reg(&self) -> &Arc<NodeRegistry> {
		&self.node_registry
	}

	pub fn parse_mutator<T>(&self, node: &kdl::KdlNode) -> anyhow::Result<GenericMutator<T>>
	where
		T: 'static,
	{
		let mut ctx = self.next_node();
		let id = node.get_str_req(ctx.consume_idx())?;
		let factory = self.node_registry.get_mutator_factory(id)?;
		factory.from_kdl::<T>(node, &mut ctx)
	}

	pub fn parse_evaluator<C, V>(
		&self,
		node: &kdl::KdlNode,
	) -> anyhow::Result<GenericEvaluator<C, V>>
	where
		C: 'static,
		V: 'static,
	{
		let mut ctx = self.next_node();
		let id = node.get_str_req(ctx.consume_idx())?;
		let factory = self.node_registry.get_evaluator_factory(id)?;
		factory.from_kdl::<C, V>(node, &mut ctx)
	}
}

pub trait KDLNode {
	fn id() -> &'static str
	where
		Self: Sized;

	fn get_id(&self) -> &'static str;
}

#[macro_export]
macro_rules! impl_kdl_node {
	($target:ty, $id:expr) => {
		impl crate::kdl_ext::KDLNode for $target {
			fn id() -> &'static str {
				$id
			}

			fn get_id(&self) -> &'static str {
				$id
			}
		}
	};
}

pub trait FromKDL {
	fn from_kdl(node: &kdl::KdlNode, ctx: &mut NodeContext) -> anyhow::Result<Self>
	where
		Self: Sized;
}

macro_rules! impl_fromkdl {
	($target:ty, $method:ident, $map:expr) => {
		impl FromKDL for $target {
			fn from_kdl(node: &kdl::KdlNode, ctx: &mut NodeContext) -> anyhow::Result<Self> {
				Ok(node.$method(ctx.consume_idx()).map($map)?)
			}
		}
	};
}
impl_fromkdl!(bool, get_bool_req, |v| v);
impl_fromkdl!(u8, get_i64_req, |v| v as u8);
impl_fromkdl!(i8, get_i64_req, |v| v as i8);
impl_fromkdl!(u16, get_i64_req, |v| v as u16);
impl_fromkdl!(i16, get_i64_req, |v| v as i16);
impl_fromkdl!(u32, get_i64_req, |v| v as u32);
impl_fromkdl!(i32, get_i64_req, |v| v as i32);
impl_fromkdl!(u64, get_i64_req, |v| v as u64);
impl_fromkdl!(i64, get_i64_req, |v| v);
impl_fromkdl!(u128, get_i64_req, |v| v as u128);
impl_fromkdl!(i128, get_i64_req, |v| v as i128);
impl_fromkdl!(usize, get_i64_req, |v| v as usize);
impl_fromkdl!(isize, get_i64_req, |v| v as isize);
impl_fromkdl!(f32, get_f64_req, |v| v as f32);
impl_fromkdl!(f64, get_f64_req, |v| v);
impl FromKDL for String {
	fn from_kdl(node: &kdl::KdlNode, ctx: &mut NodeContext) -> anyhow::Result<Self> {
		Ok(node.get_str_req(ctx.consume_idx())?.to_string())
	}
}

impl<T> FromKDL for Option<T>
where
	T: FromKDL,
{
	fn from_kdl(node: &kdl::KdlNode, ctx: &mut NodeContext) -> anyhow::Result<Self> {
		// Instead of consuming the next-idx, just peek to see if there is a value there or not.
		match node.get(ctx.peak_idx()) {
			Some(_) => T::from_kdl(node, ctx).map(|v| Some(v)),
			None => Ok(None),
		}
	}
}