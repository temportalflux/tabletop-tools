use crate::storage::{
	autosync::{status::Status, ModuleFile},
	MODULE_TOPIC,
};
use github::{repos, Error, GithubClient, Query, SearchRepositoriesParams};
use std::{collections::VecDeque, path::PathBuf};

pub struct ScanRepository {
	pub status: Status,
	pub client: GithubClient,

	pub owner: String,
	pub name: String,
	pub tree_id: Option<String>,
}
impl ScanRepository {
	pub async fn run(self) -> Result<Vec<ModuleFile>, Error> {
		let repo_id = format!("{}/{}", self.owner, self.name);
		let mut tree_count = 1;
		self.status.push_stage(format!("Scanning {repo_id}"), Some(tree_count));

		let tree_id = match self.tree_id {
			Some(id) => id,
			None => {
				let search_params = SearchRepositoriesParams {
					query: Query::default().keyed("repo", &repo_id).keyed("topic", MODULE_TOPIC),
					page_size: 1,
				};
				let (_, repositories) = self.client.search_repositories(search_params).await;

				let Some(metadata) = repositories.into_iter().next() else {
					return Err(Error::InvalidResponse("Empty repository metadata".to_string().into()));
				};
				metadata.tree_id
			}
		};

		let mut tree_ids = VecDeque::from([(PathBuf::new(), tree_id)]);
		let mut files = Vec::new();
		while let Some((tree_path, tree_id)) = tree_ids.pop_front() {
			let args =
				repos::tree::Args { owner: self.owner.as_str(), repo: self.name.as_str(), tree_id: tree_id.as_str() };
			self.status.increment_progress();
			for entry in self.client.get_tree(args).await? {
				let full_path = tree_path.join(&entry.path);
				// if the entry is a directory, put it in the queue to be scanned
				if entry.is_tree {
					tree_ids.push_back((full_path, entry.id));
					tree_count += 1;
					self.status.set_progress_max(tree_count);
				} else {
					// only record content files (kdl extension)
					if !entry.path.ends_with(".kdl") {
						continue;
					}
					// extract the system the content is for (which is the top-most parent).
					// if this path has no parent, then it isn't in a system and can be ignored.
					match full_path.parent() {
						None => continue,
						Some(path) if path == std::path::Path::new("") => continue,
						_ => {}
					}
					let path_in_repo = full_path.display().to_string().replace("\\", "/");
					files.push(ModuleFile { path_in_repo, file_id: entry.id });
				}
			}
		}

		self.status.pop_stage();
		Ok(files)
	}
}
