mod ability_score;
pub use ability_score::*;

mod action_budget;
pub use action_budget::*;

mod add_bundle;
pub use add_bundle::*;

mod apply_if;
pub use apply_if::*;

mod armor_class;
pub use armor_class::*;

mod condition;
pub use condition::*;

mod description;
pub use description::*;

mod defense;
pub use defense::*;

mod feature;
pub use feature::*;

mod flag;
pub use flag::*;

mod hit_dice;
pub use hit_dice::*;

mod hit_points;
pub use hit_points::*;

mod level;
pub use level::*;

mod modify;
pub use modify::*;

mod proficient;
pub use proficient::*;

mod pick_n;
pub use pick_n::*;

mod rest;
pub use rest::*;

mod sense;
pub use sense::*;

mod speed;
pub use speed::*;

mod spellcasting;
pub use spellcasting::*;

mod starting_equipment;

pub use starting_equipment::*;

mod stat;
pub use stat::*;

mod user_tag;
pub use user_tag::*;

#[cfg(test)]
pub(crate) mod test {
	macro_rules! test_utils {
		($mut_ty:ty) => {
			test_utils!($mut_ty, crate::system::generics::Registry::default_with_mut::<$mut_ty>());
		};
		($mut_ty:ty, $node_reg:expr) => {
			static NODE_NAME: &str = "mutator";
			type Target = crate::system::mutator::Generic<<$mut_ty as crate::system::Mutator>::Target>;

			fn node_ctx() -> crate::kdl_ext::NodeContext {
				crate::kdl_ext::NodeContext::registry($node_reg)
			}

			fn from_kdl<'doc>(mut node: crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Target> {
				Target::from_kdl(&mut node)
			}

			fn as_kdl<E: crate::system::Mutator>(data: &E) -> crate::kdl_ext::NodeBuilder {
				let mut node = crate::kdl_ext::NodeBuilder::default();
				node.entry(data.get_id());
				node += data.as_kdl();
				node
			}
		};
	}
	pub(crate) use test_utils;
}
