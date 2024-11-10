use crate::{
	database::{entry::EntryVariantInSystemWithType, Query},
	kdl_ext::NodeContext,
	system::{dnd5e::data::item::Item, generator::SystemObjectList, Block, SourceId},
	utility::PinFutureLifetimeNoSend,
};
use kdlize::{AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};

mod filter;
pub use filter::Filter;
mod variant;
pub use variant::Variant;

#[derive(Clone, PartialEq, Debug)]
pub struct ItemGenerator {
	pub(in super::super) id: SourceId,
	pub(in super::super) short_id: String,
	// the filter applied to search for the item objects
	pub(in super::super) filter: Filter,
	pub(in super::super) variants: Vec<Variant>,
}

kdlize::impl_kdl_node!(ItemGenerator, "item");
crate::impl_trait_eq!(ItemGenerator);

impl ItemGenerator {
	pub fn id(&self) -> &SourceId {
		&self.id
	}
}

impl crate::system::Generator for ItemGenerator {
	fn source_id(&self) -> &SourceId {
		&self.id
	}

	fn short_id(&self) -> &String {
		&self.short_id
	}

	fn execute<'this>(
		&'this self, args: crate::system::generator::Args<'this>,
	) -> PinFutureLifetimeNoSend<'this, anyhow::Result<SystemObjectList>> {
		Box::pin(async move {
			let mut output = SystemObjectList::new(self, args.system.node());

			// Finds all entries in the system under the Item category,
			// skipping any that don't match the filter provided via criteria.
			let index = EntryVariantInSystemWithType::new::<Item>(args.system.id(), false);
			let query = Query::<crate::database::Entry>::subset(args.database, Some(index)).await?;
			let query = query.filter_by(self.filter.as_criteria());
			let mut query = query.parse_as::<Item>(args.system_registry);
			while let Some((entry, item)) = query.next().await {
				//log::debug!(target: "item-gen", "creating variants of {}", item.id.unversioned());
				// Each item needs each variant applied to it
				for variant in &self.variants {
					// applying a variant means:
					// 1. cloning the original entry & item
					let mut entry = entry.clone();
					let mut item = item.clone();
					item.id.variant = Some(output.variant_id(variant.name.clone()));
					// 2. appling the extensions
					variant.apply_to(&mut item)?;
					// 3. reserializing into the entry (both content and metadata)
					entry.kdl = {
						let mut doc = kdl::KdlDocument::new();
						doc.nodes_mut().push(item.as_kdl().build("item"));
						let doc = doc.to_string();
						let doc = doc.replace("\\r", "");
						let doc = doc.replace("\\n", "\n");
						let doc = doc.replace("\\t", "\t");
						let doc = doc.replace("    ", "\t");
						doc
					};
					entry.metadata = item.to_metadata();
					// 4. overwriting original object fields
					entry.file_id = None;
					entry.generator_id = Some(self.source_id().to_string());
					// 5. profit
					output.insert(variant.name.clone(), entry);
					//log::debug!(target: "item-gen", "made variant {}", variant.name);
				}
			}

			Ok(output) as anyhow::Result<SystemObjectList>
		})
	}
}

impl FromKdl<NodeContext> for ItemGenerator {
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let id = crate::kdl_ext::query_source_req(node)?;
		let short_id = node.next_str_req()?.to_owned();
		let filter = node.query_req_t("scope() > filter")?;
		let variants = node.query_all_t("scope() > variant")?;
		Ok(Self { id, short_id, filter, variants })
	}
}

impl AsKdl for ItemGenerator {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();
		node.entry(self.short_id.as_str());
		node.child(("source", &self.id, OmitIfEmpty));
		node.child(("filter", &self.filter));
		node.children(("variant", self.variants.iter(), OmitIfEmpty));
		node
	}
}

#[cfg(test)]
mod test {
	use super::*;

	mod generator {
		use super::*;
		use crate::{
			kdl_ext::{test_utils::*, NodeContext},
			system::{
				dnd5e::{
					data::{description, item::armor, DamageType, Rarity},
					mutator::{AddDefense, Defense},
				},
				generator, generics, ModuleId, SourceId,
			},
			utility::selector,
		};

		static NODE_NAME: &str = "generator";

		fn node_ctx() -> NodeContext {
			NodeContext::registry({
				let mut node_reg = generics::Registry::default();
				node_reg.register_mutator::<AddDefense>();
				node_reg.register_generator::<ItemGenerator>();
				node_reg
			})
		}

		#[test]
		fn item_simple() -> anyhow::Result<()> {
			let doc = "
				|generator \"item\" \"test-gen\" {
				|    source \"local://homebrew@dnd5e/items/generator.kdl\"
				|    filter {
				|        tag \"Arrow\"
				|    }
				|    variant \"vari1\" {
				|        extend \"name\" \"{name} +1\"
				|        extend \"rarity\" \"Rare\"
				|        extend \"description\" {
				|            section \"Does +1 extra damage (probably).\"
				|        }
				|    }
				|}
			";
			let data = ItemGenerator {
				id: SourceId {
					module: Some(ModuleId::Local { name: "homebrew".into() }),
					system: Some("dnd5e".into()),
					path: "items/generator.kdl".into(),
					..Default::default()
				},
				short_id: "test-gen".into(),
				filter: Filter { tags: ["Arrow".into()].into(), ..Default::default() },
				variants: vec![Variant {
					name: "vari1".into(),
					extensions: vec![
						variant::Extension::Name("{name} +1".into()),
						variant::Extension::Rarity(Some(Rarity::Rare)),
						variant::Extension::Description(vec![description::Section {
							content: description::SectionContent::Body("Does +1 extra damage (probably).".into()),
							..Default::default()
						}]),
					],
				}],
			};
			let generator = generator::Generic::from(data);
			assert_eq_fromkdl!(generator::Generic, doc, generator);
			assert_eq_askdl!(&generator, doc);
			Ok(())
		}

		#[test]
		fn item_equipment() -> anyhow::Result<()> {
			let doc = "
				|generator \"item\" \"test-gen\" {
				|    source \"local://homebrew@dnd5e/items/generator.kdl\"
				|    filter {
				|        rarity \"None\" \"Common\"
				|        armor \"Medium\" \"Heavy\"
				|    }
				|    variant \"vari1\" {
				|        extend \"name\" \"{name} Armor of Magic Resistance\"
				|        extend \"rarity\" \"Legendary\"
				|        extend \"description\" {
				|            section \"You have resistance to magic damage while you wear this armor.\"
				|        }
				|        extend \"equipment\" {
				|            attunement
				|            armor {
				|                formula base_bonus=5
				|            }
				|            mutator \"add_defense\" \"Resistance\" (DamageType)\"Specific\" \"Psychic\"
				|        }
				|    }
				|}
			";
			let data = ItemGenerator {
				id: SourceId {
					module: Some(ModuleId::Local { name: "homebrew".into() }),
					system: Some("dnd5e".into()),
					path: "items/generator.kdl".into(),
					..Default::default()
				},
				short_id: "test-gen".into(),
				filter: Filter {
					armor: Some([armor::Kind::Medium, armor::Kind::Heavy].into()),
					rarity: [None, Some(Rarity::Common)].into(),
					..Default::default()
				},
				variants: vec![Variant {
					name: "vari1".into(),
					extensions: vec![
						variant::Extension::Name("{name} Armor of Magic Resistance".into()),
						variant::Extension::Rarity(Some(Rarity::Legendary)),
						variant::Extension::Description(vec![description::Section {
							content: description::SectionContent::Body(
								"You have resistance to magic damage while you wear this armor.".into(),
							),
							..Default::default()
						}]),
						variant::Extension::Equipment {
							attunement: Some(variant::AttunementExtension { mutators: Vec::new() }),
							weapon: None,
							armor: Some(variant::ArmorExtension {
								formula: Some(variant::ArmorFormulaExtension { base_bonus: Some(5) }),
							}),
							mutators: [AddDefense {
								defense: Defense::Resistance,
								damage_type: Some(selector::Value::Specific(DamageType::Psychic)),
								context: None,
							}
							.into()]
							.into(),
						},
					],
				}],
			};
			let generator = generator::Generic::from(data);
			assert_eq_fromkdl!(generator::Generic, doc, generator);
			assert_eq_askdl!(&generator, doc);
			Ok(())
		}
	}
}
