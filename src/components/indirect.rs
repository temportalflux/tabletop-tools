use crate::{
	components::{database::use_typed_fetch_callback, Spinner},
	system::{dnd5e::data::Indirect, Block},
};
use std::rc::Rc;
use yew::prelude::*;
use yew_hooks::*;

#[derive(Clone, PartialEq, Properties)]
pub struct IndirectFetchProps<T: PartialEq> {
	pub indirect: Indirect<T>,
	pub to_inner: Callback<Rc<T>, Html>,
}

#[function_component]
pub fn IndirectFetch<T>(props: &IndirectFetchProps<T>) -> Html
where
	T: 'static + Clone + PartialEq + Block + Unpin,
	T::Error: std::fmt::Debug,
{
	let IndirectFetchProps { indirect, to_inner } = props;

	let found_record = use_state(|| None::<Rc<T>>);
	let fetch_record = use_typed_fetch_callback(
		"Fetch Indirect Object".into(),
		Callback::from({
			let found_record = found_record.clone();
			move |object: T| {
				found_record.set(Some(Rc::new(object)));
			}
		}),
	);
	let is_first_mount = use_is_first_mount();

	let object = match indirect {
		Indirect::Id(id) => {
			if is_first_mount {
				fetch_record.emit(id.unversioned());
			}
			found_record.as_ref().cloned()
		}
		Indirect::Object(object) => Some(Rc::new(object.clone())),
	};

	let Some(object) = object else {
		return html!(<Spinner />);
	};

	to_inner.emit(object)
}
