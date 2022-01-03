use std::collections::HashMap;
use lazy_static::lazy_static;

#[derive(Hash, PartialEq, Eq)]
pub enum Target {
	C64,
}

impl Target {
	pub fn from_string(string: &str) -> Self {
		match string.to_uppercase().as_str() {
			"C64" => Self::C64,
			_ => Self::C64,
		}
	}
}

pub fn char_format(chr: u8, target: &Target) -> u8 {
	let func = CHAR_FORMAT_FUNCS[target];
	(func)(chr)
}

lazy_static! {
	pub static ref CHAR_FORMAT_FUNCS: HashMap<Target, fn(u8) -> u8> = {
		let mut map = HashMap::<Target, fn(u8) -> u8>::new();
		map.insert(Target::C64, |c| c - 96 * c.is_ascii_lowercase() as u8);

		map
	};
}
