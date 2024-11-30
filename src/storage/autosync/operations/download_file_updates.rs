use crate::{
	storage::autosync::{status::Status, ModuleFile, ModuleFileUpdate},
	system::ModuleId,
};
use github::{repos, ChangedFileStatus, Error, GithubClient};
use std::{collections::HashSet, path::Path};

pub struct DownloadFileUpdates {
	pub status: Status,
	pub client: GithubClient,

	pub module_id: ModuleId,
	pub version: String,
	pub files: Vec<ModuleFileUpdate>,
}
impl DownloadFileUpdates {
	pub async fn run(mut self) -> Result<(Vec<(ModuleFile, String)>, HashSet<String>), Error> {
		let ModuleId::Github { user_org, repository } = &self.module_id else {
			// ERROR: Invalid module id
			return Ok((Vec::new(), HashSet::new()));
		};
		self.status.push_stage(format!("Downloading {user_org}/{repository}"), Some(self.files.len()));
		let mut files_to_parse = Vec::with_capacity(self.files.len());
		let mut removed_file_ids = HashSet::new();
		let files = self.files.drain(..).collect::<Vec<_>>();
		for file_update in files {
			let ModuleFileUpdate { file, status } = file_update;

			self.status.increment_progress();

			if !file.path_in_repo.ends_with(".kdl") {
				continue;
			}

			let args = repos::contents::get::Args {
				owner: user_org.as_str(),
				repo: repository.as_str(),
				path: Path::new(file.path_in_repo.as_str()),
				version: self.version.as_str(),
			};
			match status {
				ChangedFileStatus::Added
				| ChangedFileStatus::Modified
				| ChangedFileStatus::Renamed
				| ChangedFileStatus::Copied
				| ChangedFileStatus::Changed => {
					let content = self.client.get_file_content(args).await?;
					files_to_parse.push((file, content));
				}
				ChangedFileStatus::Removed => {
					removed_file_ids.insert(file.file_id);
				}
				ChangedFileStatus::Unchanged => {}
			}
		}
		self.status.pop_stage();
		Ok((files_to_parse, removed_file_ids))
	}
}
