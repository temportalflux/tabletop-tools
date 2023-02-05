use super::{
	modifier::{
		AddAbilityScore, AddLanguage, AddLifeExpectancy, AddMaxHeight, AddSkill, Container,
		Selector,
	},
	roll::{Die, Roll, RollSet},
	Ability, Action, Feature, ProficiencyLevel, Skill,
};
use enum_map::EnumMap;
use enumset::EnumSet;
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
};

mod description;
pub use description::*;
mod lineage;
pub use lineage::*;
mod upbringing;
pub use upbringing::*;

#[derive(Clone)]
pub struct Character {
	#[allow(dead_code)]
	description: Description,
	ability_scores: EnumMap<Ability, i32>,
	lineages: [Option<Lineage>; 2],
	upbringing: Option<Upbringing>,
	background: Option<Background>,
	classes: Vec<Class>,
	selected_values: HashMap<PathBuf, String>,
}

#[derive(Clone)]
pub struct Background;
#[derive(Clone)]
pub struct Class;

#[derive(Clone, Default, PartialEq, Debug)]
pub struct CompiledStats {
	pub missing_selections: Vec<PathBuf>,
	pub ability_scores: EnumMap<Ability, i32>,
	pub skills: EnumMap<Skill, ProficiencyLevel>,
	pub languages: Vec<String>,
	pub life_expectancy: i32,
	pub max_height: (i32, RollSet),
}

impl Character {
	pub fn with_culture(mut self, culture: Culture) -> Self {
		let [a, b] = culture.lineages;
		self.lineages = [Some(a), Some(b)];
		self.upbringing = Some(culture.upbringing);
		self
	}

	pub fn compile_stats(&self) -> CompiledStats {
		let mut stats = CompiledStats::default();
		stats.ability_scores = self.ability_scores;

		let scope = PathBuf::new();
		for lineage in &self.lineages {
			if let Some(lineage) = lineage {
				lineage.apply_modifiers(self, &mut stats, scope.join(&lineage.id()));
			}
		}
		if let Some(upbringing) = &self.upbringing {
			upbringing.apply_modifiers(self, &mut stats, scope.join(&upbringing.id()));
		}

		stats
	}

	pub fn get_selection(&self, stats: &mut CompiledStats, scope: &Path) -> Option<&str> {
		let selection = self.selected_values.get(scope).map(String::as_str);
		if selection.is_none() {
			stats.missing_selections.push(scope.to_owned());
		}
		selection
	}
}

#[derive(Clone, PartialEq)]
pub struct Culture {
	pub lineages: [Lineage; 2],
	pub upbringing: Upbringing,
}

pub fn changeling_culture() -> Culture {
	let life_expectancy = Feature {
		name: "Age".into(),
		description: "Your life expectancy increases by 50 years.".into(),
		modifiers: vec![Box::new(AddLifeExpectancy(50))],
		..Default::default()
	};
	let size = Feature {
		name: "Size".into(),
		description: "Your height increases by 30 + 1d4 inches.".into(),
		modifiers: vec![
			Box::new(AddMaxHeight::Value(30)),
			Box::new(AddMaxHeight::Roll(Roll {
				amount: 1,
				die: Die::D4,
			})),
		],
		..Default::default()
	};
	Culture {
    lineages: [
			Lineage {
				name: "Changeling I".to_owned(),
				description: "One of your birth parents is a changeling. You can change your appearance at will.".to_owned(),
				features: vec![
					life_expectancy.clone(),
					size.clone(),
					Feature {
						name: "Shapechanger".to_owned(),
						description: "As an action, you can change your appearance. You determine the specifics of the changes, \
						including your coloration, hair length, and sex. You can also adjust your height and weight, \
						but not so much that your size changes. While shapechanged, none of your game statistics change. \
						You can't duplicate the appearance of a creature you've never seen, and you must adopt a form that has \
						the same basic arrangement of limbs that you have. Your voice, clothing, and equipment aren't changed by this trait. \
						You stay in the new form until you use an action to revert to your true form or until you die.".into(),
						action: Some(Action::Full),
						..Default::default()
					},
				],
			},
			Lineage {
				name: "Changeling II".to_owned(),
				description: "One of your birth parents is a changeling. You can perfectly mimic another person's voice.".to_owned(),
				features: vec![
					life_expectancy.clone(),
					size.clone(),
					Feature {
						name: "Voice Change".to_owned(),
						description: "As an action, you can change your voice. You can't duplicate the voice of a creature you've never heard. \
						Your appearance remains the same. You keep your mimicked voice until you use an action to revert to your true voice.".into(),
						action: Some(Action::Full),
						..Default::default()
					},
				],
			},
		],
    upbringing: Upbringing {
			name: "Incognito".into(),
			description: "You were brought up by those who were not what they seemed.".into(),
			features: vec![
				Feature {
					name: "Ability Score Increase".into(),
					description: "Your Charisma score increases by 2. In addition, one ability score of your choice increases by 1.".into(),
					modifiers: vec![
						Box::new(AddAbilityScore {
							ability: Selector::Specific(Ability::Charisma),
							value: 2,
						}),
						Box::new(AddAbilityScore {
							ability: Selector::AnyOf {
								id: None,
								options: EnumSet::only(Ability::Charisma).complement().into_iter().collect(),
							},
							value: 1,
						}),
					],
					..Default::default()
				},
				Feature {
					name: "Good with People".into(),
					description: "You gain proficiency with two of the following skills of your choice: Deception, Insight, Intimidation, and Persuasion.".into(),
					modifiers: vec![
						Box::new(AddSkill {
							skill: Selector::AnyOf {
								id: None,
								options: vec![
									Skill::Deception, Skill::Insight, Skill::Intimidation, Skill::Persuasion,
								],
							},
							proficiency: ProficiencyLevel::Full,
						}),
					],
					..Default::default()
				},
				Feature {
					name: "Languages".into(),
					description: "You can speak, read, and write Common and two other languages of your choice.".into(),
					modifiers: vec![
						Box::new(AddLanguage(Selector::Specific("Common".into()))),
						Box::new(AddLanguage(Selector::Any { id: Some("langA".into()) })),
						Box::new(AddLanguage(Selector::Any { id: Some("langB".into()) })),
					],
					..Default::default()
				},
			],
		},
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use enum_map::enum_map;

	#[test]
	fn test_changeling() {
		let character = Character {
			description: Description {
				name: "changeling".into(),
				pronouns: "".into(),
			},
			ability_scores: enum_map! {
				Ability::Strength => 10,
				Ability::Dexterity => 10,
				Ability::Constitution => 10,
				Ability::Intelligence => 10,
				Ability::Wisdom => 10,
				Ability::Charisma => 10,
			},
			lineages: [None, None],
			upbringing: None,
			background: None,
			classes: Vec::new(),
			selected_values: HashMap::from([
				(
					PathBuf::from("culture\\Incognito\\AbilityScoreIncrease"),
					"con".into(),
				),
				(
					PathBuf::from("culture\\Incognito\\GoodWithPeople"),
					"Insight".into(),
				),
				(
					PathBuf::from("culture\\Incognito\\Languages\\langA"),
					"Draconic".into(),
				),
				(
					PathBuf::from("culture\\Incognito\\Languages\\langB"),
					"Undercommon".into(),
				),
			]),
		}
		.with_culture(changeling_culture());
		assert_eq!(
			character.compile_stats(),
			CompiledStats {
				ability_scores: enum_map! {
					Ability::Strength => 10,
					Ability::Dexterity => 10,
					Ability::Constitution => 11,
					Ability::Intelligence => 10,
					Ability::Wisdom => 10,
					Ability::Charisma => 12,
				},
				skills: enum_map! {
					Skill::Acrobatics => ProficiencyLevel::None,
					Skill::AnimalHandling => ProficiencyLevel::None,
					Skill::Arcana => ProficiencyLevel::None,
					Skill::Athletics => ProficiencyLevel::None,
					Skill::Deception => ProficiencyLevel::None,
					Skill::History => ProficiencyLevel::None,
					Skill::Insight => ProficiencyLevel::Full,
					Skill::Intimidation => ProficiencyLevel::None,
					Skill::Investigation => ProficiencyLevel::None,
					Skill::Medicine => ProficiencyLevel::None,
					Skill::Nature => ProficiencyLevel::None,
					Skill::Perception => ProficiencyLevel::None,
					Skill::Performance => ProficiencyLevel::None,
					Skill::Persuasion => ProficiencyLevel::None,
					Skill::Religion => ProficiencyLevel::None,
					Skill::SleightOfHand => ProficiencyLevel::None,
					Skill::Stealth => ProficiencyLevel::None,
					Skill::Survival => ProficiencyLevel::None,
				},
				languages: vec!["Common".into(), "Draconic".into(), "Undercommon".into()],
				life_expectancy: 100,
				max_height: (
					60,
					RollSet(enum_map! {
						Die::D4 => 2,
						Die::D6 => 0,
						Die::D8 => 0,
						Die::D10 => 0,
						Die::D12 => 0,
						Die::D20 => 0,
					})
				),
				missing_selections: vec![],
			}
		);
	}
}
