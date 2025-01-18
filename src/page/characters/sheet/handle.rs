use crate::{
	database::{entry::EntryInSystemWithType, Database, FetchError, Query},
	system::{
		self,
		dnd5e::data::character::{Character, DefaultsBlock, ObjectCacheArc, ObjectCacheProvider, Persistent},
		Block, SourceId,
	},
	task,
};
use kdlize::{ext::DocumentExt2, NodeId};
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
			CharacterState::Loaded { character, file_id, version } => {
				(Some(character.clone()), file_id.clone(), version.clone())
			}
			CharacterState::Unloaded => (None, None, None),
		})),
		pending_mutations: Rc::new(Mutex::new(Vec::new())),
		pending_changes: Rc::new(Mutex::new(Vec::new())),
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
	state_backup: Rc<Mutex<(Option<Character>, /*file_id*/ Option<String>, /*version*/ Option<String>)>>,
	pending_mutations: Rc<Mutex<Vec<FnMutator>>>,
	pending_changes: Rc<Mutex<Vec<system::change::Generic<Character>>>>,
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
		matches!(*self.state, CharacterState::Loaded { .. })
	}

	pub fn unload(&self) {
		*self.state_backup.lock().unwrap() = (None, None, None);
		self.state.set(CharacterState::Unloaded);
	}

	fn load_with(&self, id: SourceId) {
		wasm_bindgen_futures::spawn_local({
			let handle = self.clone();
			let initialize_character = async move {
				let Some(system) = &id.system else {
					return Err(CharacterInitializationError::NoSystem.into());
				};
				let id_str = id.unversioned().to_string();
				log::info!(target: "character", "Initializing from {:?}", id_str);

				// Query the database for the character and its storage file-id
				let (file_id, version, persistent) = {
					let query = Query::<crate::database::Entry>::single(&handle.object_cache.database, &id_str);
					let query = query.await.map_err(FetchError::from)?;
					let mut query = query.parse_as::<Persistent>(&handle.object_cache.system_depot);
					match query.next().await {
						Some((entry, typed)) => (entry.file_id, entry.version, typed),
						None if !id.has_path() => (None, None, Persistent::new(id.clone())),
						None => return Err(CharacterInitializationError::CharacterMissing(id_str).into()),
					}
				};

				{
					let mut state_guard = handle.state_backup.lock().unwrap();
					state_guard.1 = file_id;
					state_guard.2 = version;
				}

				// Query the database for any defaults needed to initialize a character
				let default_blocks = {
					let index = EntryInSystemWithType::new::<DefaultsBlock>(system);
					let query =
						Query::<crate::database::Entry>::subset(&handle.object_cache.database, Some(index)).await;
					let query = query.map_err(|err| CharacterInitializationError::DefaultsError(format!("{err:?}")))?;
					let query = query.parse_as_cached::<DefaultsBlock>(
						&handle.object_cache.system_depot,
						&handle.object_cache.object_cache,
					);
					let query = query.map(|(_, block)| block);
					query.collect::<Vec<_>>().await
				};

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
	Loaded {
		character: Character,
		file_id: Option<String>,
		version: Option<String>,
	},
}
impl CharacterState {
	fn value(&self) -> &Character {
		match self {
			Self::Loaded { character, .. } => character,
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
		let mut state_guard = self.state_backup.lock().unwrap();
		state_guard.0 = Some(character.clone());
		self.state.set(CharacterState::Loaded {
			character,
			file_id: state_guard.1.clone(),
			version: state_guard.2.clone(),
		});
	}

	fn trigger_recompile(&self, requires_recompile: bool) {
		let handle = self.clone();
		self.task_dispatch.spawn("Recompile Character", None, async move {
			let has_pending_changes = handle.has_pending_changes();
			handle.process_pending_mutations(requires_recompile).await;

			let pending_data_locked = task::Signal::new(true);
			if let Some(character) = handle.pending_data.lock().unwrap().take() {
				handle.load_supplemental_objects(&character, &pending_data_locked);

				// Save the character to the database to retain changelist data between webpage reloads
				if has_pending_changes {
					let persistent = character.persistent().clone();
					let document = persistent.export_as_kdl().to_string_unescaped();
					let metadata = persistent.to_metadata();
					let (file_id, version) = {
						let state_guard = handle.state_backup.lock().unwrap();
						(state_guard.1.clone(), state_guard.2.clone())
					};
					let request = crate::storage::save_to_database::SaveToDatabase {
						database: handle.object_cache.database.clone(),
						id: character.id().clone(),
						category: character.persistent().get_id().into(),
						metadata,
						document,
						file_id,
						version: version.unwrap_or_default(),
					};
					request.execute().await?;
				}

				handle.set_loaded(character);
			} else {
				log::error!(target: "character", "Missing pending character during recompile");
			}
			pending_data_locked.unset();

			Ok(()) as anyhow::Result<()>
		});
	}

	fn has_pending_changes(&self) -> bool {
		let has_mutations = { !self.pending_mutations.lock().unwrap().is_empty() };
		let has_changes = { !self.pending_changes.lock().unwrap().is_empty() };
		has_mutations || has_changes
	}

	async fn process_pending_mutations(&self, requires_recompile: bool) {
		let Some(character) = &mut *self.pending_data.lock().unwrap() else { return };

		if requires_recompile {
			character.persistent_mut().mark_structurally_changed();
		}

		'recompile_and_mutate: loop {
			// Recompile the character if it was requested or a change has resulted in a structural addition
			// (e.g. added a bundle, equipped an item, added a feature, etc)
			if character.persistent().has_structurally_changed() {
				let cached_spells = character.spellcasting_mut().take_cached_spells();
				character.clear_derived();
				if let Err(err) = character.recompile(&self.object_cache).await {
					log::warn!("Encountered error updating cached character objects: {err:?}");
				}
				character.spellcasting_mut().insert_cached_spells(cached_spells);
			}

			let mutations = {
				let mut pending = self.pending_mutations.lock().unwrap();
				pending.drain(..).collect::<Vec<_>>()
			};
			let changes = {
				let mut pending = self.pending_changes.lock().unwrap();
				pending.drain(..).collect::<Vec<_>>()
			};
			if mutations.is_empty() && changes.is_empty() {
				break 'recompile_and_mutate;
			}

			for mutation in mutations {
				match mutation(character.persistent_mut()) {
					MutatorImpact::None => {}
					MutatorImpact::Recompile => {
						character.persistent_mut().mark_structurally_changed();
					}
				}
			}
			for change in changes {
				change.apply_to(character);
				character.persistent_mut().changes.push(change);
			}
		}
	}

	fn load_supplemental_objects(&self, character: &Character, pending_data_locked: &task::Signal) {
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
		let pending_data_locked = pending_data_locked.clone();
		let _signal = self.task_dispatch.spawn("Load spells", None, async move {
			pending_data_locked.wait_false().await;
			while let Ok((group, entry, spell)) = recv_req.recv().await {
				let mut pending_guard = handle.pending_data.lock().unwrap();
				let mut using_loaded_character = false;
				if pending_guard.is_none() {
					using_loaded_character = true;
					*pending_guard = handle.state_backup.lock().unwrap().0.clone();
				}
				let spell_id = spell.id.unversioned();
				let Some(character) = &mut *pending_guard else {
					log::error!(target: "character",
						"missing character when receiving loaded supplemental objects. Aborting load of {}",
						spell_id
					);
					continue;
				};

				match group {
					SpellGroup::Prepared => {
						if let Some(entry) = character.spellcasting().prepared_spells().get(&spell_id) {
							if entry.spell.is_some() {
								continue;
							}
						}
						character.spellcasting_mut().insert_resolved_prepared_spell(spell);
					}
					SpellGroup::Ritual => {
						if character.spellcasting().get_ritual(&spell_id).is_some() {
							continue;
						}
						character.spellcasting_mut().insert_resolved_ritual_spell(spell_id, entry.metadata, spell);
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

	fn trigger_process_pending_mutations(&self) {
		if self.pending_mutations.lock().unwrap().is_empty() && self.pending_changes.lock().unwrap().is_empty() {
			return;
		}
		// If there is already an operation in progress, we cannot process mutations right now.
		let mut pending_guard = self.pending_data.lock().unwrap();
		if pending_guard.is_some() {
			log::warn!("there is already data pending");
			return;
		}
		// If there is no data yet loaded, we cannot process mutations right now.
		let character = match self.state_backup.lock().unwrap().0.as_ref() {
			Some(character) => character.clone(),
			None => {
				log::error!("character not loaded");
				return;
			}
		};

		*pending_guard = Some(character);
		drop(pending_guard);

		self.trigger_recompile(false);
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

	pub fn add_change(&self, change: impl system::Change<Target = Character> + 'static + Send + Sync) {
		{
			let mut pending_changes = self.pending_changes.lock().unwrap();
			pending_changes.push(system::change::Generic::from(change));
		}
		self.trigger_process_pending_mutations();
	}

	pub fn dispatch_change<I, F, O>(&self, generator: F) -> Callback<I>
	where
		I: 'static,
		F: Fn(I) -> Option<O> + 'static,
		O: system::Change<Target = Character> + 'static + Send + Sync,
	{
		let handle = self.clone();
		let generator = std::rc::Rc::new(generator);
		Callback::from(move |input: I| {
			if let Some(change) = (*generator)(input) {
				handle.add_change(change);
			}
		})
	}

	pub fn save_to_storage(&self, storage: github::GithubClient, navigator: yew_router::prelude::Navigator) {
		// Takes the changelist from persistent data and returns it and the copy of persistent data,
		// updating the loaded character state in the process.
		// The changelist order returned in from oldest to newest change.
		let (character, file_id, _version) = {
			let state_guard = self.state_backup.lock().unwrap();
			state_guard.clone()
		};
		let Some(mut character) = character else { return };
		let Some(file_id) = file_id else {
			log::error!(target: "character", "cannot save, missing storage file id");
			return;
		};
		let changelist = character.persistent_mut().take_changelist();

		let state = self.clone();
		let database = self.object_cache.database.clone();
		self.task_dispatch.spawn("Save Character", None, async move {
			use kdlize::ext::DocumentExt2;

			let id = character.id().unversioned();
			let is_new = !id.has_path();
			let persistent = character.persistent().clone();
			let category = persistent.get_id().to_owned();

			let commit_message = format!("Save {}", persistent.description.name);
			// Convert the character's changelist into a document message,
			// reversing the order of changes such that the newest is at the top,
			// and the oldest is at the bottom of the message.
			let commit_body = {
				let mut node = kdlize::NodeBuilder::default();
				let iter_changes = changelist.into_iter().rev();
				node.children(("change", iter_changes, kdlize::OmitIfEmpty));
				let document = (!node.is_empty()).then(|| node.into_document());
				document.as_ref().map(kdlize::ext::DocumentExt2::to_string_unescaped)
			};

			let document = persistent.export_as_kdl().to_string_unescaped();
			let persistent_metadata = persistent.to_metadata();

			let request = crate::storage::save_to_storage::SaveToStorage {
				storage,
				id,
				file_id: Some(file_id),
				commit_message,
				commit_body,
				document: document.clone(),
			};
			let response = request.execute().await?;

			let route = crate::page::characters::Route::sheet(&response.id);

			let request = crate::storage::save_to_database::SaveToDatabase {
				database,
				id: response.id,
				category,
				metadata: persistent_metadata,
				document,
				file_id: Some(response.file_id.clone()),
				version: response.version.clone(),
			};
			request.execute().await?;

			{
				let mut state_guard = state.state_backup.lock().unwrap();
				state_guard.1 = Some(response.file_id);
				state_guard.2 = Some(response.version);
			}
			state.set_loaded(character);

			if is_new {
				navigator.push(&route);
			}

			Ok(()) as Result<(), anyhow::Error>
		});
	}
}
