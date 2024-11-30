use crate::storage::MODULE_TOPIC;
use github::{GithubClient, Query, RepositoryMetadata, SearchRepositoriesParams};

pub struct ScanForModules {
	pub client: GithubClient,
	pub owners: Vec<String>,
}
impl ScanForModules {
	pub async fn run(self) -> Result<Vec<RepositoryMetadata>, github::Error> {
		// Query github for all modules with the topic MODULE_TOPIC which are owned by the provided owners (user or organization).

		let iter_owners = self.owners.into_iter();
		let query = iter_owners.fold(Query::default(), |query, owner| query.keyed("user", owner));
		let query = query.keyed("topic", MODULE_TOPIC);

		let search_params = SearchRepositoriesParams { query, page_size: 25 };
		let (_, repositories) = self.client.search_repositories(search_params).await;
		Ok(repositories)
	}
}
