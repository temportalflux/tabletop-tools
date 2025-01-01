use crate::{
	database::{entry::EntryInSystemWithType, Database, FetchError, Query},
	system::{
		self,
		dnd5e::data::character::{Character, DefaultsBlock, ObjectCacheArc, ObjectCacheProvider, Persistent},
		SourceId,
	},
	task,
};
use std::{collections::BTreeSet, rc::Rc, sync::Mutex};
use yew::prelude::*;

#[hook]
pub fn use_character(id: SourceId) -> CharacterHandle {
	let database = use_context::<Database>().unwrap();
	let system_depot = use_context::<system::Registry>().unwrap();
	let object_cache = use_context::<ObjectCacheArc>().unwrap();
	let task_dispatch = use_context::<task::Dispatch>().unwrap();

	let state = use_state(|| CharacterState::default());
	let handle = CharacterHandle {
		object_cache: ObjectCacheProvider::new(&database, &system_depot, &object_cache),
		task_dispatch,
		state: state.clone(),
		pending_data: Rc::new(Mutex::new(None)),
		state_backup: Rc::new(Mutex::new(match &*state {
			CharacterState::Loaded(character) => Some(character.clone()),
			CharacterState::Unloaded => None,
		})),
		pending_mutations: Rc::new(Mutex::new(Vec::new())),
	};

	// Character Initialization
	use_effect_with(
		(handle.clone(), handle.is_loaded(), handle.is_processing()),
		|(handle, is_loaded, is_processing)| {
			if !is_processing && !is_loaded {
				handle.load_with(id);
			}
		},
	);

	handle
}

#[derive(thiserror::Error, Debug)]
enum CharacterInitializationError {
	#[error("Character has no game system associated with it.")]
	NoSystem,
	#[error("Character at key {0:?} is not in the database.")]
	CharacterMissing(String),
	#[error(transparent)]
	EntryError(#[from] FetchError),
	#[error("Defaults block query failed: {0}")]
	DefaultsError(String),
}

#[derive(Clone)]
pub struct CharacterHandle {
	object_cache: ObjectCacheProvider,
	task_dispatch: task::Dispatch,
	// The data referenced when creating the UI/webpage, and which causes consumers of the character to re-render.
	state: UseStateHandle<CharacterState>,
	// Data that is being mutated by recompilation, mutations, and background object loading (spells).
	// This will get propagated to the authoritiative state.
	pending_data: Rc<Mutex<Option<Character>>>,
	// A copy of the authoritiative state, because the state handle only updates the
	// shared-data after a couple of frames (not immediately in the same stack that an update is dispatched).
	state_backup: Rc<Mutex<Option<Character>>>,
	pending_mutations: Rc<Mutex<Vec<FnMutator>>>,
}
impl PartialEq for CharacterHandle {
	fn eq(&self, other: &Self) -> bool {
		self.state == other.state
	}
}
impl std::ops::Deref for CharacterHandle {
	type Target = Character;
	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}
impl AsRef<Character> for CharacterHandle {
	fn as_ref(&self) -> &Character {
		self.state.value()
	}
}

impl CharacterHandle {
	pub fn is_loaded(&self) -> bool {
		matches!(*self.state, CharacterState::Loaded(_))
	}

	pub fn unload(&self) {
		self.state.set(CharacterState::Unloaded);
	}

	fn load_with(&self, id: SourceId) {
		wasm_bindgen_futures::spawn_local({
			let handle = self.clone();
			let initialize_character = async move {
				let Some(system) = &id.system else {
					return Err(CharacterInitializationError::NoSystem);
				};
				let id_str = id.to_string();
				log::info!(target: "character", "Initializing from {:?}", id_str);

				let entry = handle.object_cache.get_typed_entry::<Persistent>(id.clone(), None).await?;
				let persistent = match entry {
					Some(known) => known,
					None if !id.has_path() => Persistent { id: id.clone(), ..Default::default() },
					None => {
						return Err(CharacterInitializationError::CharacterMissing(id_str));
					}
				};

				let index = EntryInSystemWithType::new::<DefaultsBlock>(system);
				let query = Query::<crate::database::Entry>::subset(&handle.object_cache.database, Some(index)).await;
				let query = query.map_err(|err| CharacterInitializationError::DefaultsError(format!("{err:?}")))?;
				let query = query.parse_as_cached::<DefaultsBlock>(
					&handle.object_cache.system_depot,
					&handle.object_cache.object_cache,
				);
				let query = query.map(|(_, block)| block);
				let default_blocks = query.collect::<Vec<_>>().await;

				*handle.pending_data.lock().unwrap() = Some(Character::new(persistent, default_blocks));
				handle.trigger_recompile(true);

				Ok(()) as Result<(), CharacterInitializationError>
			};
			async move {
				if let Err(err) = initialize_character.await {
					log::error!(target: "character", "Failed to initialize character: {err:?}");
				}
			}
		});
	}
}

pub enum MutatorImpact {
	None,
	Recompile,
}

#[derive(Clone, PartialEq, Default, Debug)]
enum CharacterState {
	#[default]
	Unloaded,
	Loaded(Character),
}
impl CharacterState {
	fn value(&self) -> &Character {
		match self {
			Self::Loaded(character) => character,
			Self::Unloaded => panic!("character not loaded"),
		}
	}
}

type FnMutator = Box<dyn FnOnce(&mut Persistent) -> MutatorImpact + 'static>;
impl CharacterHandle {
	fn is_processing(&self) -> bool {
		self.pending_data.lock().unwrap().is_some()
	}

	fn set_loaded(&self, character: Character) {
		*self.state_backup.lock().unwrap() = Some(character.clone());
		self.state.set(CharacterState::Loaded(character));
	}

	fn trigger_recompile(&self, requires_recompile: bool) {
		let handle = self.clone();
		self.task_dispatch.spawn("Recompile Character", None, async move {
			handle.process_pending_mutations(requires_recompile).await;

			if let Some(character) = handle.pending_data.lock().unwrap().take() {
				handle.load_supplemental_objects(&character);
				handle.set_loaded(character);
			} else {
				log::error!(target: "character", "Missing pending character during recompile");
			}

			Ok(()) as anyhow::Result<()>
		});
	}

	async fn process_pending_mutations(&self, mut requires_recompile: bool) {
		'recompile_and_mutate: loop {
			if requires_recompile {
				if let Some(character) = &mut *self.pending_data.lock().unwrap() {
					character.clear_derived();
					if let Err(err) = character.recompile(&self.object_cache).await {
						log::warn!("Encountered error updating cached character objects: {err:?}");
					}
				}
				requires_recompile = false;
			}

			let pending = {
				let mut pending = self.pending_mutations.lock().unwrap();
				pending.drain(..).collect::<Vec<_>>()
			};
			if !pending.is_empty() {
				let mut character_guard = self.pending_data.lock().unwrap();
				if let Some(character) = &mut *character_guard {
					for mutator in pending {
						match mutator(character.persistent_mut()) {
							MutatorImpact::None => {}
							MutatorImpact::Recompile => {
								requires_recompile = true;
							}
						}
					}
				}
			} else {
				break 'recompile_and_mutate;
			}
		}
	}

	fn load_supplemental_objects(&self, character: &Character) {
		use crate::system::{dnd5e::data::Spell, System};

		let (send_req, recv_req) = async_channel::unbounded();

		#[derive(Clone, Copy)]
		enum SpellGroup {
			Prepared,
			Ritual,
		}

		self.task_dispatch.spawn("Query Prepared Spells", None, {
			let mut prepared_spell_ids = BTreeSet::new();
			for (id, spell_entry) in character.spellcasting().prepared_spells() {
				if spell_entry.spell.is_none() {
					prepared_spell_ids.insert(id.clone());
				}
			}

			let recv_group = SpellGroup::Prepared;
			let channel = send_req.clone();
			let provider = self.object_cache.clone();
			async move {
				if prepared_spell_ids.is_empty() {
					return Ok(());
				}
				let query = Query::<crate::database::Entry>::multiple(&provider.database, &prepared_spell_ids).await?;
				let mut query = query.parse_as_cached::<Spell>(&provider.system_depot, &provider.object_cache);
				while let Some((entry, spell)) = query.next().await {
					let _ = channel.try_send((recv_group, entry, spell));
				}
				Ok(()) as Result<(), database::Error>
			}
		});

		self.task_dispatch.spawn("Query Ritual Spells", None, {
			let provider = self.object_cache.clone();
			let channel = send_req.clone();
			let criteria = character.spellcasting().ritual_cache().query_criteria.clone();
			async move {
				let Some(criteria) = criteria else { return Ok(()) };
				let index = EntryInSystemWithType::new::<Spell>(system::dnd5e::DnD5e::id());
				let query = Query::subset(&provider.database, Some(index)).await?;
				let query = query.filter_by(criteria);
				let mut query = query.parse_as_cached::<Spell>(&provider.system_depot, &provider.object_cache);
				while let Some((entry, spell)) = query.next().await {
					let _ = channel.try_send((SpellGroup::Ritual, entry, spell));
				}
				Ok(()) as Result<(), database::Error>
			}
		});

		drop(send_req);

		let handle = self.clone();
		let _signal = self.task_dispatch.spawn("Load spells", None, async move {
			while let Ok((group, entry, spell)) = recv_req.recv().await {
				let mut pending_guard = handle.pending_data.lock().unwrap();
				let mut using_loaded_character = false;
				if pending_guard.is_none() {
					using_loaded_character = true;
					if let Some(state) = &*handle.state_backup.lock().unwrap() {
						*pending_guard = Some(state.clone());
					}
				}
				let Some(character) = &mut *pending_guard else {
					log::error!(target: "character",
						"missing character when receiving loaded supplemental objects. Aborting load of {}",
						spell.id.unversioned()
					);
					continue;
				};

				match group {
					SpellGroup::Prepared => {
						character.spellcasting_mut().insert_resolved_prepared_spell(spell);
					}
					SpellGroup::Ritual => {
						character.spellcasting_mut().insert_resolved_ritual_spell(entry, spell);
					}
				}

				if using_loaded_character {
					if let Some(character) = pending_guard.take() {
						handle.set_loaded(character);
					}
				}

				handle.trigger_process_pending_mutations();
			}
			Ok(()) as anyhow::Result<()>
		});
	}

	pub fn dispatch<F>(&self, mutator: F)
	where
		F: FnOnce(&mut Persistent) -> MutatorImpact + 'static,
	{
		{
			let mut pending_mutations = self.pending_mutations.lock().unwrap();
			pending_mutations.push(Box::new(mutator));
		}
		self.trigger_process_pending_mutations();
	}

	fn trigger_process_pending_mutations(&self) {
		if self.pending_mutations.lock().unwrap().is_empty() {
			return;
		}
		// If there is already an operation in progress, we cannot process mutations right now.
		let mut pending_guard = self.pending_data.lock().unwrap();
		if pending_guard.is_some() {
			return;
		}
		// If there is no data yet loaded, we cannot process mutations right now.
		let character = match self.state_backup.lock().unwrap().as_ref() {
			Some(character) => character.clone(),
			None => return,
		};

		*pending_guard = Some(character);
		drop(pending_guard);

		self.trigger_recompile(false);
	}

	pub fn new_dispatch<I, F>(&self, mutator: F) -> Callback<I>
	where
		I: 'static,
		F: Fn(I, &mut Persistent) -> MutatorImpact + 'static,
	{
		let handle = self.clone();
		let mutator = std::rc::Rc::new(mutator);
		Callback::from(move |input: I| {
			let mutator = mutator.clone();
			handle.dispatch(move |persistent| (*mutator)(input, persistent));
		})
	}
}
