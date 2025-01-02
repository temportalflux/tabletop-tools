use crate::{
	database::{entry::EntryInSystemWithType, Criteria, Database, Entry, Query},
	system::{self, Block, SourceId},
};
use futures_util::FutureExt;
use std::{
	collections::BTreeMap,
	rc::Rc,
	sync::{Arc, Mutex},
};
use yew::prelude::*;
use yew_hooks::{use_async_with_options, UseAsyncHandle, UseAsyncOptions};

#[derive(Clone)]
pub struct QueryAllArgs<T> {
	pub system: String,
	pub criteria: Option<Box<Criteria>>,
	pub adjust_listings: Option<Arc<dyn Fn(Vec<T>) -> Vec<T> + 'static>>,
	pub max_limit: Option<usize>,
}
impl<T> Default for QueryAllArgs<T> {
	fn default() -> Self {
		Self { system: Default::default(), criteria: None, adjust_listings: None, max_limit: None }
	}
}

#[derive(Clone)]
pub struct UseQueryHandle<Input, Output, E> {
	async_handle: UseAsyncHandle<Output, E>,
	run: Rc<dyn Fn(Input)>,
}

impl<Input, Output, E> PartialEq for UseQueryHandle<Input, Output, E>
where
	Output: PartialEq,
	E: PartialEq,
{
	fn eq(&self, other: &Self) -> bool {
		self.async_handle == other.async_handle
	}
}

impl<Input, Output, E> UseQueryHandle<Input, Output, E>
where
	E: Clone,
{
	pub fn status(&self) -> QueryStatus<&Output, E> {
		if self.async_handle.loading {
			return QueryStatus::Pending;
		}
		if let Some(error) = &self.async_handle.error {
			return QueryStatus::Failed(error.clone());
		}
		if let Some(data) = &self.async_handle.data {
			return QueryStatus::Success(data);
		}
		QueryStatus::Empty
	}

	pub fn update(&self, output: Output) {
		self.async_handle.update(output);
	}

	pub fn run(&self, input: Input) {
		(self.run)(input);
	}
}

#[derive(Debug, PartialEq)]
pub enum QueryStatus<T, E> {
	Empty,
	Pending,
	Success(T),
	Failed(E),
}

#[hook]
pub fn use_query<QueryBuilder, Input, Output, E>(
	input: Option<Input>, query_builder: QueryBuilder,
) -> UseQueryHandle<Input, Output, E>
where
	Input: 'static + Clone,
	QueryBuilder: 'static + Fn(Database, Input) -> futures::future::LocalBoxFuture<'hook, Result<Output, E>>,
	Output: 'static + Clone + Default,
	E: 'static + std::error::Error + Clone,
{
	let database = use_context::<Database>().unwrap();
	let query_builder = Rc::new(query_builder);
	let options = UseAsyncOptions { auto: input.is_some() };

	let args_handle = Rc::new(Mutex::new(input));
	let async_args = args_handle.clone();

	let async_handle = use_async_with_options(
		async move {
			let guard = async_args.lock().unwrap();
			let Some(input) = (*guard).as_ref() else {
				return Ok(Output::default());
			};
			Ok((*query_builder)(database.clone(), input.clone()).await?) as Result<Output, E>
		},
		options,
	);

	let run = Rc::new({
		let handle = async_handle.clone();
		move |input: Input| {
			*args_handle.lock().unwrap() = Some(input);
			handle.run();
		}
	});

	UseQueryHandle { async_handle, run }
}

#[hook]
pub fn use_query_callback<QueryBuilder, Input, Output, Arg, E>(
	query_builder: QueryBuilder, callback: Callback<(Arg, Output)>,
) -> Callback<(Arg, Input)>
where
	Arg: 'static,
	Input: 'static,
	QueryBuilder: 'static + Fn(Database, Input) -> futures::future::LocalBoxFuture<'hook, Result<Output, E>>,
	Output: 'static,
	E: 'static + std::fmt::Debug,
{
	let database = use_context::<Database>().unwrap();
	let query_builder = Rc::new(query_builder);
	Callback::from(move |(arg, input): (Arg, Input)| {
		let database = database.clone();
		let query_builder = query_builder.clone();
		let callback = callback.clone();
		wasm_bindgen_futures::spawn_local({
			let pending = async move {
				let output = (*query_builder)(database.clone(), input).await?;
				callback.emit((arg, output));
				Ok(()) as Result<(), E>
			};
			async move {
				if let Err(err) = pending.await {
					log::error!(target: "database", "query-callback failed: {err:?}");
				}
			}
		});
	})
}

pub type UseQueryAllHandle<T> = UseQueryHandle<QueryAllArgs<T>, Vec<T>, database::Error>;

#[hook]
pub fn use_query_all_typed<T>(
	_auto_fetch: bool, initial_args: Option<QueryAllArgs<T>>,
) -> UseQueryHandle<QueryAllArgs<T>, Vec<T>, database::Error>
where
	T: Block + Unpin + Clone + 'static,
	T::Error: std::fmt::Debug,
{
	let system_depot = use_context::<system::Registry>().unwrap();
	use_query(initial_args, move |database, input| {
		let system_depot = system_depot.clone();
		async move {
			let QueryAllArgs { system, criteria, adjust_listings, max_limit } = input;
			if system.is_empty() {
				return Ok(Vec::new());
			}

			let index = Some(EntryInSystemWithType::new::<T>(system));
			let query = Query::subset(&database, index).await?;
			let query = query.apply_opt(criteria.map(|c| *c), Query::filter_by);
			let query = query.parse_as::<T>(&system_depot);
			let query = query.map(|(_entry, item)| item);
			let query = query.apply_opt(max_limit, Query::take);
			let mut entries = query.collect::<Vec<_>>().await;
			if let Some(adjust_listings) = &adjust_listings {
				entries = (*adjust_listings)(entries);
			}
			Ok(entries)
		}
		.boxed_local()
	})
}

pub type UseQueryDiscreteTypedHandle<T> =
	UseQueryHandle<Vec<SourceId>, (Vec<SourceId>, BTreeMap<SourceId, T>), database::Error>;

#[hook]
pub fn use_query_typed<T>() -> UseQueryHandle<Vec<SourceId>, (Vec<SourceId>, BTreeMap<SourceId, T>), database::Error>
where
	T: Block + Unpin + Clone + 'static,
	T::Error: std::fmt::Debug,
{
	let system_depot = use_context::<system::Registry>().unwrap();
	use_query(None, move |database, args: Vec<SourceId>| {
		let system_depot = system_depot.clone();
		async move {
			if args.is_empty() {
				return Ok((args, BTreeMap::new()));
			}
			let query = Query::<Entry>::multiple(&database, &args).await?;
			let query = query.parse_as::<T>(&system_depot);
			let iter = query.map(|(entry, typed)| (entry.source_id(false), typed));
			let entries = iter.collect::<BTreeMap<_, _>>().await;
			Ok((args, entries))
		}
		.boxed_local()
	})
}

#[hook]
pub fn use_typed_fetch_callback<EntryContent>(_task_name: String, fn_item: Callback<EntryContent>) -> Callback<SourceId>
where
	EntryContent: 'static + Block + Unpin,
	Event: 'static,
	EntryContent::Error: std::fmt::Debug,
{
	use_typed_fetch_callback_tuple(_task_name, fn_item.reform(|(output, _)| output)).reform(|id| (id, ()))
}

#[hook]
pub fn use_typed_fetch_callback_tuple<Output, Arg>(
	_task_name: String, fn_item: Callback<(Output, Arg)>,
) -> Callback<(SourceId, Arg)>
where
	Output: 'static + Block + Unpin,
	Arg: 'static,
	Output::Error: std::fmt::Debug,
{
	let system_depot = use_context::<system::Registry>().unwrap();
	use_query_callback(
		move |database, input: SourceId| {
			let system_depot = system_depot.clone();
			async move {
				let query = Query::<Entry>::single(&database, &input).await?;
				let mut query = query.parse_as::<Output>(&system_depot);
				let Some((_entry, output)) = query.next().await else {
					return Err(crate::GeneralError(format!("Missing record for id {input}")).into());
				};
				Ok(output) as Result<_, anyhow::Error>
			}
			.boxed_local()
		},
		fn_item.reform(|(arg, output)| (output, arg)),
	)
	.reform(|(id, arg)| (arg, id))
}
