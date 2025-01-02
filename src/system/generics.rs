use super::{change, evaluator, generator, mutator, Change, Evaluator, Generator, Mutator};
use crate::kdl_ext::NodeContext;
use kdlize::{FromKdl, NodeId};
use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
#[error("No {0} registration found for {1:?}.")]
pub struct MissingRegistration(&'static str, String);

pub struct Registry {
	mutators: HashMap<&'static str, mutator::Factory>,
	evaluators: HashMap<&'static str, evaluator::Factory>,
	generators: HashMap<&'static str, generator::Factory>,
	generator_order: Vec<&'static str>,
	changes: HashMap<&'static str, change::Factory>,
}

impl Default for Registry {
	fn default() -> Self {
		Self {
			mutators: HashMap::new(),
			evaluators: HashMap::new(),
			generators: HashMap::new(),
			generator_order: Vec::new(),
			changes: HashMap::new(),
		}
	}
}

impl Registry {
	pub fn register_mutator<T>(&mut self)
	where
		T: Mutator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<T::Error>,
	{
		assert!(!self.mutators.contains_key(T::id()));
		self.mutators.insert(T::id(), mutator::Factory::new::<T>());
	}

	pub fn register_evaluator<E>(&mut self)
	where
		E: Evaluator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<E::Error>,
	{
		assert!(!self.evaluators.contains_key(E::id()));
		self.evaluators.insert(E::id(), evaluator::Factory::new::<E>());
	}

	pub fn register_generator<G>(&mut self)
	where
		G: Generator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<<G as FromKdl<NodeContext>>::Error>,
	{
		assert!(!self.generators.contains_key(G::id()));
		self.generator_order.push(G::id());
		self.generators.insert(G::id(), generator::Factory::new::<G>());
	}

	pub fn register_change<T>(&mut self)
	where
		T: Change + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<T::Error>,
	{
		assert!(!self.changes.contains_key(T::id()));
		self.changes.insert(T::id(), change::Factory::new::<T>());
	}

	pub fn get_mutator_factory(&self, id: &str) -> anyhow::Result<&mutator::Factory> {
		self.mutators.get(id).ok_or(MissingRegistration("mutator", id.to_owned()).into())
	}

	pub fn get_evaluator_factory(&self, id: &str) -> anyhow::Result<&evaluator::Factory> {
		self.evaluators.get(id).ok_or(MissingRegistration("evaluator", id.to_owned()).into())
	}

	pub fn get_generator_factory(&self, id: &str) -> anyhow::Result<&generator::Factory> {
		self.generators.get(id).ok_or(MissingRegistration("generator", id.to_owned()).into())
	}

	pub fn get_generator_order(&self) -> &[&'static str] {
		&self.generator_order
	}

	pub fn get_change_factory(&self, id: &str) -> anyhow::Result<&change::Factory> {
		self.changes.get(id).ok_or(MissingRegistration("change", id.to_owned()).into())
	}
}

#[cfg(test)]
impl Registry {
	pub fn default_with_eval<E>() -> Self
	where
		E: Evaluator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<E::Error>,
	{
		let mut node_reg = Self::default();
		node_reg.register_evaluator::<E>();
		node_reg
	}

	pub fn default_with_mut<M>() -> Self
	where
		M: Mutator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		anyhow::Error: From<M::Error>,
	{
		let mut node_reg = Self::default();
		node_reg.register_mutator::<M>();
		node_reg
	}

	pub fn parse_kdl_evaluator<C, T>(self, doc: &str) -> anyhow::Result<evaluator::Generic<C, T>>
	where
		C: 'static,
		T: 'static,
	{
		let document = doc.parse::<kdl::KdlDocument>()?;
		let node = document.query("scope() > evaluator")?.expect("missing evaluator node");
		let ctx = crate::kdl_ext::NodeContext::registry(self);
		let mut node = crate::kdl_ext::NodeReader::new_child(node, ctx);
		evaluator::Generic::<C, T>::from_kdl(&mut node)
	}

	pub fn parse_kdl_mutator<T>(self, doc: &str) -> anyhow::Result<mutator::Generic<T>>
	where
		T: 'static,
	{
		let document = doc.parse::<kdl::KdlDocument>()?;
		let node = document.query("scope() > mutator")?.expect("missing mutator node");
		let ctx = crate::kdl_ext::NodeContext::registry(self);
		let mut node = crate::kdl_ext::NodeReader::new_child(node, ctx);
		mutator::Generic::<T>::from_kdl(&mut node)
	}

	pub fn defaulteval_parse_kdl<E>(doc: &str) -> anyhow::Result<evaluator::Generic<E::Context, E::Item>>
	where
		E: Evaluator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		E::Error: std::error::Error + Send + Sync,
		E::Item: 'static,
	{
		Self::default_with_eval::<E>().parse_kdl_evaluator::<E::Context, E::Item>(doc)
	}

	pub fn defaultmut_parse_kdl<M>(doc: &str) -> anyhow::Result<mutator::Generic<M::Target>>
	where
		M: Mutator + NodeId + FromKdl<NodeContext> + 'static + Send + Sync,
		M::Error: std::error::Error + Send + Sync,
		M::Target: 'static,
	{
		Self::default_with_mut::<M>().parse_kdl_mutator(doc)
	}
}
