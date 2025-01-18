use crate::utility::InvalidEnumStr;
use enum_map::Enum;
use enumset::EnumSetType;
use std::str::FromStr;

#[derive(Debug, EnumSetType, Enum)]
pub enum Rest {
	Short,
	Long,
}

impl std::fmt::Display for Rest {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Short => write!(f, "Short"),
			Self::Long => write!(f, "Long"),
		}
	}
}

impl FromStr for Rest {
	type Err = InvalidEnumStr<Self>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"Short" => Ok(Self::Short),
			"Long" => Ok(Self::Long),
			_ => Err(InvalidEnumStr::from(s)),
		}
	}
}

static DESC_SHORT: &str = "\
A short rest is a period of downtime, at least 1 hour long, \
during which a character does nothing more strenuous than \
eating, drinking, reading, and tending to wounds.";
static DESC_LONG: &str = "\
A long rest is a period of extended downtime, at least 8 hours long, \
during which a character sleeps for at least 6 hours and performs \
no more than 2 hours of light activity, such as reading, talking, eating, or standing watch.

If the rest is interrupted by a period of strenuous activity — at least 1 hour of \
walking, fighting, casting spells, or similar adventuring activity — \
the characters must begin the rest again to gain any benefit from it.";

impl Rest {
	pub fn description(&self) -> &'static str {
		match self {
			Self::Short => DESC_SHORT,
			Self::Long => DESC_LONG,
		}
	}
}
