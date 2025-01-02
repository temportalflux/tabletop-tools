pub mod autosync;
pub mod save_to_database;
pub mod save_to_storage;

pub static USER_HOMEBREW_REPO_NAME: &str = "integro-homebrew";
pub static MODULE_TOPIC: &str = "integro-tabletop-module";
pub static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub fn get(status: &crate::auth::Status) -> Option<github::GithubClient> {
	use crate::auth::*;
	use std::str::FromStr;
	let Status::Successful { oauth_id, token } = status else {
		return None;
	};
	let Ok(oauth) = OAuthProvider::from_str(oauth_id) else {
		return None;
	};
	match oauth {
		OAuthProvider::Github => github::GithubClient::new(token, APP_USER_AGENT).ok(),
	}
}
