use super::Criteria;
use crate::{
	database::{Database, Entry},
	kdl_ext::NodeContext,
	system::generics,
};
use database::Cursor;
use futures_util::StreamExt;
use kdlize::FromKdl;
use std::{pin::Pin, sync::Arc, task::Poll};

pub struct Query {
	pub cursor: Cursor<Entry>,
	pub criteria: Option<Box<Criteria>>,
}

impl futures_util::stream::Stream for Query {
	type Item = Entry;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			let Poll::Ready(entry) = self.cursor.poll_next_unpin(cx) else {
				return Poll::Pending;
			};
			let Some(entry) = entry else {
				return Poll::Ready(None);
			};
			if let Some(criteria) = &self.criteria {
				if !criteria.is_relevant(&entry.metadata) {
					continue;
				}
			}
			return Poll::Ready(Some(entry));
		}
	}
}

pub struct QueryDeserialize<Output> {
	#[allow(dead_code)]
	pub db: Database,
	pub query: Query,
	pub node_reg: Arc<generics::Registry>,
	pub marker: std::marker::PhantomData<Output>,
}
impl<Output> futures_util::stream::Stream for QueryDeserialize<Output>
where
	Output: FromKdl<NodeContext> + Unpin,
	Output::Error: std::fmt::Debug,
{
	type Item = (Entry, Output);

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
		loop {
			// Get the next database entry based on the query and underlying cursor
			let Poll::Ready(entry) = self.query.poll_next_unpin(cx) else {
				return Poll::Pending;
			};
			let Some(entry) = entry else {
				return Poll::Ready(None);
			};
			let Some(value) = entry.parse_kdl::<Output>(self.node_reg.clone()) else {
				continue;
			};
			// we found a sucessful value! we can return it
			return Poll::Ready(Some((entry, value)));
		}
	}
}
impl<Output> QueryDeserialize<Output>
where
	Output: FromKdl<NodeContext> + Unpin,
	Output::Error: std::fmt::Debug,
{
	pub async fn first_n(mut self, limit: Option<usize>) -> Vec<(Entry, Output)> {
		let mut items = Vec::new();
		while let Some(item) = self.next().await {
			items.push(item);
			if let Some(limit) = &limit {
				if items.len() >= *limit {
					break;
				}
			}
		}
		items
	}

	pub async fn all(mut self) -> Vec<(Entry, Output)> {
		let mut items = Vec::new();
		while let Some(item) = self.next().await {
			items.push(item);
		}
		items
	}
}
