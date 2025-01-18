use super::{
	character::{Character, RestEntry},
	roll::{EvaluatedRollSet, RollSet},
	Rest,
};
use crate::{
	kdl_ext::NodeContext,
	system::{dnd5e::Value, mutator::ReferencePath},
	utility::selector::{self, IdPath, ValueOptions},
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::{path::PathBuf, str::FromStr};

/* Use cases:
- use x times then destroyed
- recharges per rest, regaining xdy+b charges (e.g. 2d10+1). can be a fixed b charges, or just the roll xdy
- different actions cost different charges (e.g. casting spells using charges)

will need a way to link Charges and LimitedUses, even though they are in separate blocks.
if an item has equipment like so:
equipment {
	// must generate a resource with the relative path `charges` (e.g. `/Inventory/<uuid>/charges`)
	charges {
		capacity 10
		reset {
			rest (Rest)"Long"
			roll (Roll)"2d6+1"
		}
	}
	mutator "add_feature" name="Do Thing" {
		limited_uses {
			resource "../charges" // `../` b/c this is inside the named feature
			cost 2
		}
	}
	mutator "spellcasting" "add_prepared" ability="Intelligence" {
		spell "github://flux-tabletop:dnd5e-basic-rules@dnd5e/spells/enlargeReduce.kdl"
		limited_uses {
			// NOTE: this will mean all existing resources need to be prefixed with `/`, so they are treated as absolute paths instead of relative ones
			resource "charges" // no path data, relative to parent which is the item
			cost 5
		}
	}
}
then we need some way for limited uses in various mutators to reference the charges resource of the item (as an instance in the inventory)
atm the "resource" property ONLY refers to features
we could leverage the data-path, which equipment passes down to mutators and therefore the limited_uses object. In that situation,
resources with an absolute path would be unaffected, but relative paths would be based on the data-path provided by the mutator group chain.
*/

#[derive(Clone, PartialEq, Debug)]
pub struct Resource {
	pub display_path: IdPath,
	pub capacity: Value<i32>,
	pub reset: Option<ResourceReset>,
	pub uses_count: selector::Value<Character, u32>,
}

impl Default for Resource {
	fn default() -> Self {
		Self {
			display_path: IdPath::default(),
			capacity: Value::Fixed(0),
			reset: None,
			uses_count: selector::Value::Options(ValueOptions { id: "uses".into(), ..Default::default() }),
		}
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct ResourceReset {
	pub rest: Value<String>,
	pub rate: Option<EvaluatedRollSet>,
}

impl Resource {
	pub fn apply_to(&self, stats: &mut Character, path_to_parent: &ReferencePath) {
		stats.resources_mut().register(self);

		let Some(uses_path) = self.get_uses_path() else {
			return;
		};
		let Some(reset) = &self.reset else { return };
		let Some(rest) = reset.get_rest(stats) else { return };
		let restore_amount = reset.get_rate(stats);
		stats.rest_resets_mut().add(rest, RestEntry {
			source: path_to_parent.display.clone(),
			effects: vec![super::character::RestEffect::RestoreResourceUses { data_path: uses_path, amount: restore_amount }],
		});
	}

	pub fn set_data_path(&self, path_to_parent: &ReferencePath) {
		self.display_path.set_path(&path_to_parent);
		self.uses_count.set_data_path(path_to_parent);
	}

	pub fn get_display_path(&self) -> PathBuf {
		self.display_path.display().unwrap_or_default()
	}

	pub fn get_capacity(&self, character: &Character) -> i32 {
		self.capacity.evaluate(character)
	}

	pub fn get_uses_path(&self) -> Option<PathBuf> {
		self.uses_count.get_data_path()
	}

	pub fn get_uses_consumed(&self, character: &Character) -> u32 {
		character.get_selector_value(&self.uses_count).unwrap_or_default()
	}

	pub fn get_reset_rest(&self, character: &Character) -> Option<Rest> {
		let Some(reset) = &self.reset else {
			return None;
		};
		reset.get_rest(character)
	}

	pub fn get_reset_rate(&self, character: &Character) -> Option<RollSet> {
		let Some(reset) = &self.reset else { return None };
		reset.get_rate(character)
	}
}

impl ResourceReset {
	pub fn get_rest(&self, character: &Character) -> Option<Rest> {
		let rest_str = self.rest.evaluate(character);
		Rest::from_str(&rest_str).ok()
	}

	pub fn get_rate(&self, character: &Character) -> Option<RollSet> {
		let Some(rate_eval) = &self.rate else { return None };
		Some(rate_eval.evaluate(character))
	}
}

impl FromKdl<NodeContext> for Resource {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let capacity = match node.next_i64_opt()? {
			None => node.query_req_t("scope() > capacity")?,
			Some(capacity) => Value::Fixed(capacity as i32),
		};
		let reset = node.query_opt_t("scope() > reset")?;

		Ok(Self { capacity, reset, ..Default::default() })
	}
}

impl AsKdl for Resource {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match &self.capacity {
			Value::Fixed(capacity) => node.entry(*capacity as i64),
			capacity => node.child(("capacity", capacity)),
		}
		node.child(("reset", &self.reset));
		node
	}
}

impl FromKdl<NodeContext> for ResourceReset {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let rest = match node.next_str_opt()? {
			Some(rest_str) => Value::Fixed(rest_str.to_owned()),
			None => node.query_req_t("scope() > rest")?,
		};
		let rate = node.query_opt_t("scope() > rate")?;
		Ok(Self { rest, rate })
	}
}

impl AsKdl for ResourceReset {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match &self.rest {
			Value::Fixed(rest_str) if self.rate.is_none() => {
				node.entry(rest_str.as_str());
			}
			rest_eval => {
				node.child(("rest", rest_eval));
			}
		}
		node.child(("rate", &self.rate));
		node
	}
}
