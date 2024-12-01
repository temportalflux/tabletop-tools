use github::{GithubClient, RepositoryMetadata};

pub struct FindRepository {
	pub client: GithubClient,
	pub owner: String,
	pub repository: String,
}
impl FindRepository {
	pub async fn run(&mut self) -> Result<Option<RepositoryMetadata>, github::Error> {
		self.client.find_repository(&self.owner, &self.repository).await
	}
}
