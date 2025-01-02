use std::sync::Arc;
use yew::prelude::*;

pub mod dnd5e;
mod source;
pub use source::*;

pub mod block;
pub use block::Block;
pub mod change;
pub use change::Change;

mod registry;
pub use registry::*;

pub mod mutator;
pub use mutator::Mutator;
pub mod evaluator;
pub use evaluator::Evaluator;
pub mod generator;
pub use generator::Generator;

pub mod generics;

pub trait System {
	fn id() -> &'static str
	where
		Self: Sized;
	fn get_id(&self) -> &'static str;
	fn blocks(&self) -> &block::Registry;
	fn generics(&self) -> &Arc<generics::Registry>;
}

pub fn system_registry() -> Registry {
	let mut builder = crate::system::Registry::builder();
	builder.insert(dnd5e::DnD5e::new());
	builder.build()
}

#[function_component]
pub fn Provider(props: &html::ChildrenProps) -> Html {
	let depot = use_state(|| system_registry());
	html! {
		<ContextProvider<crate::system::Registry> context={(*depot).clone()}>
			{props.children.clone()}
		</ContextProvider<crate::system::Registry>>
	}
}
