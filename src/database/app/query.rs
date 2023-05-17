use super::Entry;
use crate::{
	database::Cursor,
	kdl_ext::{FromKDL, NodeContext},
	system::core::NodeRegistry,
};
use futures_util::StreamExt;
use std::{pin::Pin, sync::Arc, task::Poll};

pub enum Criteria {
	/// Passes if the value being evaluated is equal to an expected value.
	Exact(serde_json::Value),
	/// Passes if the value being evaluated does not pass the provided criteria.
	Not(Box<Criteria>),
	/// Passes if the value being evaluated:
	/// 1. Is a string
	/// 2. Contains the provided substring
	ContainsSubstring(String),
	/// Passes if the value being evaluated:
	/// 1. Is an object/map
	/// 2. Contains the provided key
	/// 3. The value at the provided key passes the provided criteria
	ContainsProperty(String, Box<Criteria>),
	/// Passes if the value being evaluated:
	/// 1. Is an array
	/// 2. Contains a value equivalent to the provided value
	ContainsValue(Box<Criteria>),
	/// Passes if the value being evaluated passes all of the provided criteria.
	All(Vec<Box<Criteria>>),
	/// Passes if the value being evaluated passes any of the provided criteria.
	Any(Vec<Box<Criteria>>),
}

impl Criteria {
	pub fn is_relevant(&self, value: &serde_json::Value) -> bool {
		match self {
			Self::Exact(expected) => value == expected,
			Self::Not(criteria) => !criteria.is_relevant(value),
			Self::ContainsSubstring(substring) => {
				let serde_json::Value::String(value) = value else { return false; };
				value.to_lowercase().contains(&substring.to_lowercase())
			}
			Self::ContainsProperty(key, criteria) => {
				let serde_json::Value::Object(map) = value else { return false; };
				let Some(value) = map.get(key) else { return false; };
				criteria.is_relevant(value)
			}
			Self::ContainsValue(criteria) => {
				let serde_json::Value::Array(value_list) = value else { return false; };
				for value in value_list {
					if criteria.is_relevant(value) {
						return true;
					}
				}
				false
			}
			Self::All(criteria) => {
				for criteria in criteria {
					if !criteria.is_relevant(value) {
						return false;
					}
				}
				true
			}
			Self::Any(criteria) => {
				for criteria in criteria {
					if criteria.is_relevant(value) {
						return true;
					}
				}
				false
			}
		}
	}
}

// TODO: Tests for criteria against specific json values

pub struct Query {
	pub cursor: Cursor<Entry>,
	pub criteria: Box<Criteria>,
}

impl futures_util::stream::Stream for Query {
	type Item = Entry;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		loop {
			let Poll::Ready(entry) = self.cursor.poll_next_unpin(cx) else { return Poll::Pending };
			let Some(entry) = entry else { return Poll::Ready(None); };
			if self.criteria.is_relevant(&entry.metadata) {
				return Poll::Ready(Some(entry));
			}
		}
	}
}

pub struct QueryDeserialize<Output> {
	pub query: Query,
	pub node_reg: Arc<NodeRegistry>,
	pub marker: std::marker::PhantomData<Output>,
}
impl<Output> futures_util::stream::Stream for QueryDeserialize<Output>
where
	Output: FromKDL + Unpin,
{
	type Item = Output;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		loop {
			// Get the next database entry based on the query and underlying cursor
			let Poll::Ready(entry) = self.query.poll_next_unpin(cx) else { return Poll::Pending };
			let Some(entry) = entry else { return Poll::Ready(None); };
			// Parse the entry's kdl string:
			// kdl string to document
			let Ok(document) = entry.kdl.parse::<kdl::KdlDocument>() else { continue; };
			// document to first (and hopefully only) node
			let Some(node) = document.nodes().get(0) else { continue; };
			// node to value based on the expected type
			let mut ctx = NodeContext::new(Arc::new(entry.source_id()), self.node_reg.clone());
			let Ok(value) = Output::from_kdl(node, &mut ctx) else { continue; };
			// we found a sucessful value! we can return it
			return Poll::Ready(Some(value));
		}
	}
}
