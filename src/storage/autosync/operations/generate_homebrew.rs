use crate::storage::{autosync::status::Status, USER_HOMEBREW_REPO_NAME};
use github::{repos, GithubClient};

// Create the homebrew repo on the github client viewer (the user that is logged in).
// https://docs.github.com/en/rest/repos/repos?apiVersion=2022-11-28#create-a-repository-for-the-authenticated-user
// https://docs.github.com/en/rest/repos/contents?apiVersion=2022-11-28#create-or-update-file-contents
pub struct GenerateHomebrew {
	pub status: Status,
	pub client: GithubClient,
}
impl GenerateHomebrew {
	pub async fn run(self) -> Result<(), github::Error> {
		self.status.push_stage("Initializing homebrew module", None);

		use crate::storage::MODULE_TOPIC;
		let create_repo = repos::create::Args { org: None, name: USER_HOMEBREW_REPO_NAME, private: true };
		let owner = self.client.create_repo(create_repo).await?;

		let set_topics = repos::set_topics::Args {
			owner: owner.as_str(),
			repo: USER_HOMEBREW_REPO_NAME,
			topics: vec![MODULE_TOPIC.to_owned()],
		};
		self.client.set_repo_topics(set_topics).await?;

		self.status.pop_stage();
		Ok(())
	}
}
