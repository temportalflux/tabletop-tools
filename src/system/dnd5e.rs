use self::data::character::Character;
use super::{block, generics};
use std::sync::Arc;

pub mod change;
pub mod components;
pub mod data;
pub mod evaluator;
pub mod generator;
pub mod mutator;

pub type BoxedCriteria = crate::system::evaluator::Generic<Character, Result<(), String>>;
pub type BoxedEvaluator<V> = crate::system::evaluator::Generic<Character, V>;
pub type BoxedMutator = crate::system::mutator::Generic<Character>;
pub type Value<T> = crate::utility::Value<Character, T>;

pub fn block_registry() -> block::Registry {
	let mut registry = block::Registry::default();
	registry.register::<data::character::DefaultsBlock>();
	registry.register::<data::character::Persistent>();
	registry.register::<data::Bundle>();
	registry.register::<data::Class>();
	registry.register::<data::Subclass>();
	registry.register::<data::Condition>();
	registry.register::<data::item::Item>();
	registry.register::<data::Spell>();
	registry.register::<crate::system::generator::Generic>();
	registry
}

pub fn node_registry() -> generics::Registry {
	use change::{hit_points::*, *};
	use evaluator::*;
	use generator::*;
	use mutator::*;
	let mut registry = generics::Registry::default();

	registry.register_mutator::<AbilityScoreChange>();
	registry.register_mutator::<AddArmorClassFormula>();
	registry.register_mutator::<AddDefense>();
	registry.register_mutator::<AddLifeExpectancy>();
	registry.register_mutator::<AddToActionBudget>();
	registry.register_mutator::<AddSize>();
	registry.register_mutator::<SuggestedPersonality>();
	registry.register_mutator::<AddMaxHitPoints>();
	registry.register_mutator::<AddProficiency>();
	registry.register_mutator::<Sense>();
	registry.register_mutator::<Speed>();
	registry.register_mutator::<SetFlag>();
	registry.register_mutator::<Spellcasting>();
	registry.register_mutator::<GrantByLevel>();
	registry.register_mutator::<PickN>();
	registry.register_mutator::<AddFeature>();
	registry.register_mutator::<AddStartingEquipment>();
	registry.register_mutator::<AddBundle>();
	registry.register_mutator::<Modify>();
	registry.register_mutator::<ApplyIf>();
	registry.register_mutator::<HitDice>();
	registry.register_mutator::<AddUserTag>();
	registry.register_mutator::<ApplyWhenRest>();

	// Evaluators output i32
	registry.register_evaluator::<Math>();
	registry.register_evaluator::<GetAbilityModifier>();
	registry.register_evaluator::<GetProficiencyBonus>();
	registry.register_evaluator::<GetHitPoints>();
	registry.register_evaluator::<GetLevelInt>();
	// Evaluators output String
	registry.register_evaluator::<GetLevelStr>();
	// Evaluators output Result<(), String>
	registry.register_evaluator::<HasArmorEquipped>();
	registry.register_evaluator::<HasAttack>();
	registry.register_evaluator::<HasBundle>();
	registry.register_evaluator::<HasCondition>();
	registry.register_evaluator::<HasProficiency>();
	registry.register_evaluator::<HasLevel>();
	registry.register_evaluator::<HasSelection>();
	registry.register_evaluator::<HasSpell>();
	registry.register_evaluator::<HasStat>();

	// Order matters here! Block generators are first because they can make other generators.
	// This order instructs the priority queue to the order in which generators are processed.
	registry.register_generator::<BlockGenerator>();
	registry.register_generator::<ItemGenerator>();

	registry.register_change::<ApplyAbilityScores>();
	registry.register_change::<ApplyCondition>();
	registry.register_change::<ApplyDescription>();
	registry.register_change::<ApplyLimitedUses>();
	registry.register_change::<ApplyNotes>();
	registry.register_change::<ConsumeItemSpell>();
	registry.register_change::<ConsumeSpellSlot>();
	registry.register_change::<DeathSaves>();
	registry.register_change::<inventory::AddItem>();
	registry.register_change::<inventory::EquipItem>();
	registry.register_change::<inventory::MoveItem>();
	registry.register_change::<inventory::PurchaseItem>();
	registry.register_change::<inventory::RemoveItem>();
	registry.register_change::<HealOrDamage>();
	registry.register_change::<ApplyHitDice>();
	registry.register_change::<LevelHP>();
	registry.register_change::<PrepareSpell>();
	registry.register_change::<TempHP>();
	registry.register_change::<ToggleInspiration>();

	registry
}

pub struct DnD5e {
	blocks: block::Registry,
	generics: Arc<generics::Registry>,
}

impl DnD5e {
	pub fn new() -> Self {
		Self { blocks: block_registry(), generics: node_registry().into() }
	}
}

impl super::System for DnD5e {
	fn id() -> &'static str {
		"dnd5e"
	}

	fn get_id(&self) -> &'static str {
		Self::id()
	}

	fn blocks(&self) -> &block::Registry {
		&self.blocks
	}

	fn generics(&self) -> &Arc<generics::Registry> {
		&self.generics
	}
}
