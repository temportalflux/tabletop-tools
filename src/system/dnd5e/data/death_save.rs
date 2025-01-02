use crate::utility::NotInList;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Debug, enum_map::Enum)]
pub enum DeathSave {
	Success,
	Failure,
}

impl DeathSave {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Failure => "failure",
			Self::Success => "success",
		}
	}
}

impl ToString for DeathSave {
	fn to_string(&self) -> String {
		self.as_str().to_owned()
	}
}

impl FromStr for DeathSave {
	type Err = NotInList;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"failure" => Ok(Self::Failure),
			"success" => Ok(Self::Success),
			_ => Err(NotInList(s.to_owned(), vec!["failure", "success"])),
		}
	}
}
