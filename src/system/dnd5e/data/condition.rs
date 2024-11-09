use super::character::{Character, ObjectCacheProvider};
use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::BoxedMutator,
		mutator::{self, ReferencePath},
		Block, SourceId,
	},
};
use async_recursion::async_recursion;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

mod indirect;
pub use indirect::*;

/// A state a character may be subject to until it is removed.
/// Some conditions are automatically cleared on the next long rest, and any conditions may be manually cleared.
/// Conditions contain a set of mutators and an optional criteria that, if met, applies those mutators.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Condition {
	pub id: Option<SourceId>,
	pub name: String,
	pub description: String,
	pub mutators: Vec<BoxedMutator>,
	pub implied: Vec<Indirect<Self>>,
}

kdlize::impl_kdl_node!(Condition, "condition");

impl Condition {
	#[async_recursion(?Send)]
	pub async fn resolve_indirection(&mut self, provider: &ObjectCacheProvider) -> anyhow::Result<()> {
		let pending = self.implied.drain(..).collect::<Vec<_>>();
		let mut resolved = Vec::with_capacity(pending.len());
		for indirect in pending {
			match indirect {
				Indirect::Id(condition_id) => {
					let condition = provider.get_typed_entry::<Condition>(condition_id.unversioned(), None).await?;
					match condition {
						None => self.implied.push(Indirect::Id(condition_id)),
						Some(condition) => resolved.push(condition),
					}
				}
				Indirect::Custom(condition) => {
					resolved.push(condition);
				}
			}
		}
		for mut condition in resolved {
			condition.resolve_indirection(provider).await?;
			self.implied.push(Indirect::Custom(condition));
		}
		Ok(())
	}
}

impl mutator::Group for Condition {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		let path_to_self = parent.join(&self.name, None);
		for mutator in &self.mutators {
			mutator.set_data_path(&path_to_self);
		}
		for implied in &self.implied {
			if let Indirect::Custom(condition) = implied {
				condition.set_data_path(parent);
			}
		}
	}

	fn apply_mutators(&self, stats: &mut Character, parent: &ReferencePath) {
		let path_to_self = parent.join(&self.name, None);
		for mutator in &self.mutators {
			stats.apply(mutator, &path_to_self);
		}
		for implied in &self.implied {
			if let Indirect::Custom(condition) = implied {
				stats.apply_from(condition, &path_to_self);
			}
		}
	}
}

impl Block for Condition {
	fn to_metadata(self) -> serde_json::Value {
		serde_json::json!({
			"name": self.name.clone(),
		})
	}
}

impl FromKdl<NodeContext> for Condition {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let id = crate::kdl_ext::query_source_opt(node)?;

		let name = node.get_str_req("name")?.to_owned();
		let description = node.query_str_opt("scope() > description", 0)?.unwrap_or_default().to_owned();
		let mutators = node.query_all_t("scope() > mutator")?;

		let implied = node.query_all_t("scope() > implies")?;

		Ok(Self { id, name, description, mutators, implied })
	}
}

impl AsKdl for Condition {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(("name", self.name.clone()));
		node.child(("source", &self.id, OmitIfEmpty));
		node.child(("description", &self.description, OmitIfEmpty));
		node.children(("mutator", self.mutators.iter()));
		node.children(("implies", self.implied.iter()));
		node
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::{
			kdl_ext::{test_utils::*, NodeContext},
			system::{
				dnd5e::{
					data::character::StatOperation,
					evaluator::HasArmorEquipped,
					mutator::{Speed, StatMutator},
				},
				generics,
			},
		};

		static NODE_NAME: &str = "condition";

		fn node_ctx() -> NodeContext {
			NodeContext::registry({
				let mut reg = generics::Registry::default();
				reg.register_mutator::<Speed>();
				reg.register_evaluator::<HasArmorEquipped>();
				reg
			})
		}

		#[test]
		fn basic() -> anyhow::Result<()> {
			let doc = "
				|condition name=\"Expedient\" {
				|    description \"You are particularly quick.\"
				|}
			";
			let data = Condition {
				name: "Expedient".into(),
				description: "You are particularly quick.".into(),
				..Default::default()
			};
			assert_eq_fromkdl!(Condition, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn mutators() -> anyhow::Result<()> {
			let doc = "
				|condition name=\"Expedient\" {
				|    description \"You are particularly quick.\"
				|    mutator \"speed\" \"Walking\" (Add)15
				|}
			";
			let data = Condition {
				name: "Expedient".into(),
				description: "You are particularly quick.".into(),
				mutators: vec![
					Speed(StatMutator { stat_name: Some("Walking".into()), operation: StatOperation::AddSubtract(15) })
						.into(),
				],
				..Default::default()
			};
			assert_eq_fromkdl!(Condition, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
