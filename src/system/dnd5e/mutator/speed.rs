use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::{
			data::{
				character::{Character, StatOperation},
				description,
			},
			mutator::StatMutator,
		},
		mutator::ReferencePath,
		Mutator,
	},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};

#[derive(Clone, Debug, PartialEq)]
pub struct Speed(pub StatMutator);

crate::impl_trait_eq!(Speed);
kdlize::impl_kdl_node!(Speed, "speed");

impl Mutator for Speed {
	type Target = Character;

	fn description(&self, _state: Option<&Character>) -> description::Section {
		let content = format!("Your {} speed {}.", self.0.stat_name, match &self.0.operation {
			StatOperation::MinimumValue(value) => format!("is at least {value} feet"),
			StatOperation::MinimumStat(value) => format!("is at least equivalent to your {value} speed"),
			StatOperation::Base(value) => format!("is at least {value} feet"),
			StatOperation::AddSubtract(value) if *value >= 0 => format!("increases by {value} feet"),
			StatOperation::AddSubtract(value) => format!("decreases by {value} feet"),
			StatOperation::MultiplyDivide(value) if *value >= 0 => format!("is multiplied by {value}"),
			StatOperation::MultiplyDivide(value) => format!("is dividied by {value}"),
		});
		description::Section { content: content.into(), ..Default::default() }
	}

	fn apply(&self, stats: &mut Character, parent: &ReferencePath) {
		stats.speeds_mut().insert(self.0.stat_name.clone(), self.0.operation.clone(), parent);
	}
}

impl FromKdl<NodeContext> for Speed {
	type Error = anyhow::Error;
	fn from_kdl(node: &mut crate::kdl_ext::NodeReader) -> anyhow::Result<Self> {
		Ok(Self(StatMutator::from_kdl(node)?))
	}
}

impl AsKdl for Speed {
	fn as_kdl(&self) -> NodeBuilder {
		self.0.as_kdl()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod kdl {
		use super::*;
		use crate::{kdl_ext::test_utils::*, system::dnd5e::mutator::test::test_utils};

		test_utils!(Speed);

		#[test]
		fn minimum() -> anyhow::Result<()> {
			let doc = "mutator \"speed\" \"Walking\" (Minimum)30";
			let data = Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(30) });
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}

		#[test]
		fn additive() -> anyhow::Result<()> {
			let doc = "mutator \"speed\" \"Walking\" (Add)30";
			let data = Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::AddSubtract(30) });
			assert_eq_askdl!(&data, doc);
			assert_eq_fromkdl!(Target, doc, data.into());
			Ok(())
		}
	}

	mod mutate {
		use super::*;
		use crate::system::dnd5e::data::{
			character::{Character, Persistent},
			Bundle,
		};
		use std::path::PathBuf;

		fn character(mutators: Vec<(&'static str, Speed)>) -> Character {
			Character::from(Persistent {
				bundles: mutators
					.into_iter()
					.map(|(name, mutator)| {
						Bundle { name: name.into(), mutators: vec![mutator.into()], ..Default::default() }.into()
					})
					.collect(),
				..Default::default()
			})
		}

		#[test]
		fn minimum_single() {
			let character = character(vec![(
				"TestFeature",
				Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(60) }),
			)]);
			let sense = character.speeds().get("Walking").cloned().collect::<Vec<_>>();
			let expected: Vec<(_, PathBuf)> = vec![(StatOperation::MinimumValue(60), "TestFeature".into())];
			assert_eq!(sense, expected);
		}

		#[test]
		fn minimum_multiple() {
			let character = character(vec![
				("B", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(60) })),
				("A", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(40) })),
			]);
			let sense = character.speeds().get("Walking").cloned().collect::<Vec<_>>();
			let expected: Vec<(_, PathBuf)> =
				vec![(StatOperation::MinimumValue(40), "A".into()), (StatOperation::MinimumValue(60), "B".into())];
			assert_eq!(sense, expected);
		}

		#[test]
		fn single_additive() {
			let character = character(vec![(
				"TestFeature",
				Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::AddSubtract(20) }),
			)]);
			let sense = character.speeds().get("Walking").cloned().collect::<Vec<_>>();
			let expected: Vec<(_, PathBuf)> = vec![(StatOperation::AddSubtract(20), "TestFeature".into())];
			assert_eq!(sense, expected);
		}

		#[test]
		fn minimum_gt_additive() {
			let character = character(vec![
				("A", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(60) })),
				("B", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::AddSubtract(40) })),
			]);
			let sense = character.speeds().get("Walking").cloned().collect::<Vec<_>>();
			let expected: Vec<(_, PathBuf)> =
				vec![(StatOperation::AddSubtract(40), "B".into()), (StatOperation::MinimumValue(60), "A".into())];
			assert_eq!(sense, expected);
		}

		#[test]
		fn minimum_lt_additive() {
			let character = character(vec![
				("A", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::MinimumValue(60) })),
				("B", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::AddSubtract(40) })),
				("C", Speed(StatMutator { stat_name: "Walking".into(), operation: StatOperation::AddSubtract(30) })),
			]);
			let sense = character.speeds().get("Walking").cloned().collect::<Vec<_>>();
			let expected: Vec<(_, PathBuf)> = vec![
				(StatOperation::AddSubtract(40), "B".into()),
				(StatOperation::AddSubtract(30), "C".into()),
				(StatOperation::MinimumValue(60), "A".into()),
			];
			assert_eq!(sense, expected);
		}
	}
}
