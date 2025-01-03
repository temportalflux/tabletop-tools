use crate::system::{ModuleId, SourceId};

pub struct SaveToStorage {
	pub storage: github::GithubClient,
	pub id: SourceId,
	pub file_id: Option<String>,
	pub commit_message: String,
	pub commit_body: Option<String>,
	pub document: String,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Github(#[from] github::Error),
	#[error("Source {0} is not supported by Github")]
	InvalidStorage(SourceId),
}

pub struct Response {
	pub id: SourceId,
	pub file_id: String,
	pub version: String,
}

impl SaveToStorage {
	pub async fn execute(mut self) -> Result<Response, Error> {
		let is_new = !self.id.has_path();
		self.id.path = match is_new {
			false => self.id.path.clone(),
			true => {
				let id = uuid::Uuid::new_v4();
				let mut buffer = uuid::Uuid::encode_buffer();
				let filename = id.as_hyphenated().encode_lower(&mut buffer);
				std::path::Path::new("character").join(format!("{filename}.kdl"))
			}
		};

		let SourceId { module: Some(ModuleId::Github { user_org, repository }), .. } = &self.id else {
			log::debug!("non-github source id");
			return Err(Error::InvalidStorage(self.id));
		};

		let path_in_repo = self.id.storage_path();
		let message = {
			// Commit messages in github can be separated into header and body by `\n\n`
			let mut message = self.commit_message;
			if let Some(body) = self.commit_body {
				message += &format!("\n\n{body}");
			}
			message
		};
		let repo_org = user_org.clone();
		let repo_name = repository.clone();

		let args = github::repos::contents::update::Args {
			repo_org: &repo_org,
			repo_name: &repo_name,
			path_in_repo: &path_in_repo,
			commit_message: &message,
			content: &self.document,
			file_id: self.file_id.as_ref().map(String::as_str),
			branch: None,
		};
		let response = self.storage.create_or_update_file(args).await?;

		Ok(Response { id: self.id, file_id: response.file_id, version: response.version })
	}
}
