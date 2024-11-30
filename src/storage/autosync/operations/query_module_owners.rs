use crate::storage::autosync::status::Status;
use github::{GithubClient, Query};

// Query github for the logged in user and all organizations they have access to.
pub struct QueryModuleOwners {
	pub status: Status,
	pub client: GithubClient,
	pub user: Option<String>,
	pub found_homebrew: bool,
}
impl QueryModuleOwners {
	pub async fn run(&mut self) -> Result<Vec<String>, github::Error> {
		self.status.push_stage("Finding module owners", None);
		let search_params = github::SearchRepositoriesParams {
			query: Query::default()
				.keyed("user", "@me")
				.value(crate::storage::USER_HOMEBREW_REPO_NAME)
				.keyed("in", "name"),
			page_size: 1,
		};
		let (user, repositories) = self.client.search_repositories(search_params).await;
		self.user = Some(user.clone());
		self.found_homebrew = !repositories.is_empty();

		let mut owners = self.client.find_orgs().await;
		owners.push(user);

		self.status.pop_stage();
		Ok(owners)
	}
}
