use crate::utility::NotInList;
use enum_map::Enum;
use enumset::EnumSetType;
use std::str::FromStr;

#[derive(Debug, EnumSetType, Enum)]
pub enum HitPoint {
	Current,
	Max,
	Temp,
}

impl FromStr for HitPoint {
	type Err = NotInList;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Current" => Ok(Self::Current),
			"Temp" => Ok(Self::Temp),
			"Max" => Ok(Self::Max),
			_ => Err(NotInList(s.into(), vec!["Current", "Temp", "Max"])),
		}
	}
}
