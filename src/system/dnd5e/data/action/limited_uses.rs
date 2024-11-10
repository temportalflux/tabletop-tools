use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::{
			data::{character::Character, Resource, ResourceReset, Rest},
			Value,
		},
		mutator::ReferencePath,
	},
	utility::selector::IdPath,
	GeneralError,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub enum LimitedUses {
	/// This is the most common format.
	/// Usages define a max quantity and a rest that they reset on.
	/// They can be referred to via `Consumer` by the path to the owner (often feature/action).
	Usage(Resource),
	Consumer {
		/// The path to the `LimitedUses::Usage` which this usage consumes from.
		resource: IdPath,
		/// The amount of uses of `resource` that this usage consumes.
		cost: u32,
	},
}

impl LimitedUses {
	pub fn set_data_path(&self, parent: &ReferencePath) {
		match self {
			Self::Usage(resource) => {
				resource.set_data_path(parent);
			}
			Self::Consumer { resource, .. } => {
				resource.set_path(parent);
			}
		}
	}

	fn get_use_data<'a>(&'a self, character: &'a Character) -> Option<&'a Resource> {
		match self {
			Self::Usage(data) => Some(data),
			Self::Consumer { resource, .. } => match resource.data() {
				Some(path) => character.resources().get(path),
				None => None,
			},
		}
	}

	pub fn get_uses_path(&self, character: &Character) -> Option<PathBuf> {
		let Some(data) = self.get_use_data(character) else {
			return None;
		};
		data.get_uses_path()
	}

	pub fn get_uses_consumed(&self, character: &Character) -> u32 {
		let Some(data) = self.get_use_data(character) else {
			return 0;
		};
		data.get_uses_consumed(character)
	}

	pub fn get_max_uses(&self, character: &Character) -> i32 {
		let Some(data) = self.get_use_data(character) else {
			log::warn!("no resource found {self:?}");
			return 0;
		};
		data.get_capacity(character)
	}

	pub fn get_reset_rest(&self, character: &Character) -> Option<Rest> {
		let Some(data) = self.get_use_data(character) else {
			return None;
		};
		data.get_reset_rest(character)
	}

	pub fn as_consumer(&self, character: &Character) -> Option<(u32, PathBuf)> {
		let Self::Consumer { cost, resource } = self else {
			return None;
		};
		let Some(resource) = resource.data() else {
			return None;
		};
		// convert the internal path name to the path displayed to users
		// e.g. replacing inventory item ids with the name of the item
		let Some(resource) = character.resources().get(resource) else {
			return None;
		};
		Some((*cost, resource.get_display_path()))
	}
}

impl FromKdl<NodeContext> for LimitedUses {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		if let Some(capacity) = node.query_opt_t::<Value<i32>>("scope() > max_uses")? {
			let reset_on = node.query_opt_t::<Value<String>>("scope() > reset_on")?;
			return Ok(Self::Usage(Resource {
				capacity,
				reset: reset_on.map(|rest| ResourceReset { rest, rate: None }),
				..Default::default()
			}));
		}

		if let Some(mut node_resource) = node.query_opt("scope() > resource")? {
			let resource = {
				let path_str = node_resource.next_str_req()?;
				let path_str = match path_str.ends_with("/uses") {
					false => path_str.to_owned() + "/uses",
					true => path_str.to_owned(),
				};
				IdPath::from(path_str)
			};
			let cost = match node.query_opt("scope() > cost")? {
				None => 1,
				Some(mut node) => node.next_i64_req()? as u32,
			};
			return Ok(Self::Consumer { resource, cost });
		}

		return Err(GeneralError("Invalid limited uses, expected a max_uses or resource property.".into()).into());
	}
}

impl AsKdl for LimitedUses {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		match self {
			Self::Usage(use_counter) => {
				node.child(("max_uses", &use_counter.capacity));
				if let Some(reset) = &use_counter.reset {
					node.child(("reset_on", &reset.rest));
				}
				node
			}
			Self::Consumer { resource, cost } => {
				let resource_path = match resource.get_id() {
					None => String::default(),
					Some(cow_str) => match cow_str.strip_suffix("/uses") {
						None => cow_str.into_owned(),
						Some(str) => str.to_owned(),
					},
				};
				node.child(("resource", resource_path));
				if *cost > 1 {
					node.child(("cost", cost));
				}
				node
			}
		}
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
					data::Rest,
					evaluator::{GetLevelInt, GetLevelStr},
				},
				generics,
			},
		};

		static NODE_NAME: &str = "limited_uses";

		fn node_ctx() -> NodeContext {
			let mut node_reg = generics::Registry::default();
			node_reg.register_evaluator::<GetLevelInt>();
			node_reg.register_evaluator::<GetLevelStr>();
			NodeContext::registry(node_reg)
		}

		#[test]
		fn fixed_uses_permanent() -> anyhow::Result<()> {
			let doc = "
				|limited_uses {
				|    max_uses 2
				|}
			";
			let data = LimitedUses::Usage(Resource { capacity: Value::Fixed(2), ..Default::default() });
			assert_eq_fromkdl!(LimitedUses, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn fixed_uses_reset() -> anyhow::Result<()> {
			let doc = "
				|limited_uses {
				|    max_uses 2
				|    reset_on \"Short\"
				|}
			";
			let data = LimitedUses::Usage(Resource {
				capacity: Value::Fixed(2),
				reset: Some(ResourceReset { rest: Value::Fixed(Rest::Short.to_string()), rate: None }),
				..Default::default()
			});
			assert_eq_fromkdl!(LimitedUses, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn scaling_uses_reset() -> anyhow::Result<()> {
			let doc = "
				|limited_uses {
				|    max_uses (Evaluator)\"get_level\" class=\"SpecificClass\" {
				|        level 2 1
				|        level 5 2
				|        level 10 4
				|        level 14 5
				|        level 20 -1
				|    }
				|    reset_on (Evaluator)\"get_level_str\" {
				|        level 1 \"Long\"
				|        level 5 \"Short\"
				|    }
				|}
			";
			let data = LimitedUses::Usage(Resource {
				capacity: Value::Evaluated(
					GetLevelInt {
						class_name: Some("SpecificClass".into()),
						order_map: [(2, 1), (5, 2), (10, 4), (14, 5), (20, -1)].into(),
					}
					.into(),
				),
				reset: Some(ResourceReset {
					rest: Value::Evaluated(
						GetLevelStr {
							class_name: None,
							order_map: [(1, Rest::Long.to_string()), (5, Rest::Short.to_string())].into(),
						}
						.into(),
					),
					rate: None,
				}),
				..Default::default()
			});
			assert_eq_fromkdl!(LimitedUses, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn resource_simple() -> anyhow::Result<()> {
			let doc = "
				|limited_uses {
				|    resource \"Cleric/level02/Channel Divinity\"
				|}
			";
			let data = LimitedUses::Consumer { resource: "Cleric/level02/Channel Divinity".into(), cost: 1 };
			assert_eq_fromkdl!(LimitedUses, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}

		#[test]
		fn resource_with_cost() -> anyhow::Result<()> {
			let doc = "
				|limited_uses {
				|    resource \"Cleric/level02/Channel Divinity\"
				|    cost 4
				|}
			";
			let data = LimitedUses::Consumer { resource: "Cleric/level02/Channel Divinity".into(), cost: 4 };
			assert_eq_fromkdl!(LimitedUses, doc, data);
			assert_eq_askdl!(&data, doc);
			Ok(())
		}
	}
}
