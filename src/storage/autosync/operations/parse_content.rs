use crate::{
	data::UserSettings,
	database::UserSettingsRecord,
	kdl_ext::{NodeContext, NodeReader},
	storage::autosync::{status::Status, ModuleFile},
	system::{self, generics, ModuleId, SourceId},
};
use anyhow::Context;
use itertools::Itertools;
use kdlize::{FromKdl, NodeId};
use std::{
	path::{Path, PathBuf},
	sync::Arc,
};

// One node per document is required for 2 reasons:
// 1. Updating the content of a file is much more complicated if there are more than 1 entry per document,
//    because other content has to either be reparsed or fetched from database for any one entry to be written.
// 2. Variants need a way to document that they are not the original object they are based on,
//    without creating a whole new id. The marker for this replaced the old `node_idx` field in the source id.
#[derive(thiserror::Error, Debug)]
#[error("Too many entries in document {0}, only one node per document is permitted.")]
pub struct TooManyEntries(pub String);

#[derive(thiserror::Error, Debug)]
#[error("No such system {0} registered in the system depot. Please check content against application setup.")]
pub struct MissingSystem(pub String);

pub struct ParseFiles {
	pub status: Status,
	pub system_depot: system::Registry,

	pub module_id: ModuleId,
	pub version: String,
	pub files: Vec<(ModuleFile, String)>,
}

#[derive(Default)]
pub struct RecordsToUpdate {
	pub entries: Vec<crate::database::Entry>,
	pub user_settings: Vec<UserSettingsRecord>,
}

impl ParseFiles {
	pub async fn run(mut self) -> Result<RecordsToUpdate, anyhow::Error> {
		let ModuleId::Github { user_org, repository } = &self.module_id else {
			// ERROR: Invalid module id
			return Ok(RecordsToUpdate::default());
		};
		self.status.push_stage(format!("Parsing {user_org}/{repository}"), Some(self.files.len()));

		let files = self.files.drain(..).collect::<Vec<_>>();
		let mut updates =
			RecordsToUpdate { entries: Vec::with_capacity(files.len()), user_settings: Vec::with_capacity(1) };
		for (ModuleFile { path_in_repo, file_id }, content) in files {
			let document =
				content.parse::<kdl::KdlDocument>().with_context(|| format!("Failed to parse content: {content:?}"))?;

			let mut system = None;
			if let Some(first_component) = Path::new(&path_in_repo).components().next() {
				let first_component_str = first_component.as_os_str().to_str().unwrap();
				if self.system_depot.iter_ids().contains(&first_component_str) {
					system = Some(first_component_str.to_owned());
				}
			}

			let mut source_id = SourceId {
				module: Some(self.module_id.clone()),
				system: system.clone(),
				path: PathBuf::from(&path_in_repo),
				// TODO: Should this include version?
				..Default::default()
			};
			if document.nodes().len() > 1 {
				return Err(TooManyEntries(source_id.to_string()).into());
			}

			let Some(node) = document.nodes().first() else {
				self.status.increment_progress();
				continue;
			};

			let node_id = node.name().value().to_owned();

			// All content in a system is interpreted as an Entry record; meaning it is some type of content that is
			// dynamically registered by whatever system it is organized under.
			// This content is registered on a per-system basis as multiple systems rarely have identical content format.
			if let Some(system) = &system {
				let Some(system_reg) = self.system_depot.get(&system) else {
					return Err(MissingSystem(system.clone()).into());
				};
				if let Some(relative_to_system) = path_in_repo.strip_prefix(&format!("{system}/")) {
					source_id.path = PathBuf::from(relative_to_system);
				}

				let metadata = system_reg.parse_metadata(node, &source_id)?;

				updates.entries.push(crate::database::Entry {
					id: source_id.to_string(),
					module: self.module_id.to_string(),
					system: system.clone(),
					category: node_id,
					version: Some(self.version.clone()),
					metadata,
					kdl: node.to_string(),
					file_id: Some(file_id.clone()),
					generator_id: None,
					generated: 0,
				});

				self.status.increment_progress();
				continue;
			}

			// All other content must have specific record types b/c it is outside any particular system (e.g. User Settings)

			if node_id == UserSettings::id() {
				// Construct the node reader using an empty generics registry since this content is unrelated any system.
				// If user settings need generics like mutators, evaluators, and generators, the app will need to have
				// a dedicated generics registry separate from any system.
				let empty_registry = Arc::new(generics::Registry::default());
				let ctx = NodeContext::new(Arc::new(source_id.clone()), empty_registry);
				let mut node_reader = NodeReader::new_root(node, ctx);

				// Parse the user settings from the kdl node
				let user_settings = UserSettings::from_kdl(&mut node_reader)?;

				updates.user_settings.push(UserSettingsRecord {
					id: source_id.to_string(),
					file_id: Some(file_id.clone()),
					version: self.version.clone(),
					user_settings,
				});

				self.status.increment_progress();
				continue;
			}

			// Any content not parsed is silently ignored
			self.status.increment_progress();
		}
		self.status.pop_stage();
		Ok(updates)
	}
}
