use lazy_static::lazy_static;
use regex::Regex;

use crate::instructions::AddressMode;
use crate::utility::*;

const REGEX_PAT_IMMEDIATE: &'static str = r"^#(.+)$";
const REGEX_PAT_ABS_X: &'static str = r"^(.+),[Xx]$";
const REGEX_PAT_ABS_Y: &'static str = r"^(.+),[Yy]$";
const REGEX_PAT_INDIRECT: &'static str = r"^\((.+)\)$";
const REGEX_PAT_INDIRECT_X: &'static str = r"^\((.+),[Xx]\)$";
const REGEX_PAT_INDIRECT_Y: &'static str = r"^\((.+)\),[Yy]$";

lazy_static! {
	pub static ref REGEX_ASSIGN: Regex = Regex::new(r"^([\w\*]+)\s*=\s*(.+)$").unwrap();
	pub static ref REGEX_INSTR: Regex = Regex::new(r"^(\w{3})(?:\s+(.+))?$").unwrap();
	pub static ref REGEX_LABEL: Regex = Regex::new(r"^(\w+):$").unwrap();
	pub static ref REGEX_PSEUDO: Regex = Regex::new(r"^\.(\w+)(?:\s+(.+))?$").unwrap();

	pub static ref ADDR_REGEXES: Vec<(Regex, fn(u16) -> (AddressMode, Vec<u8>))> = vec![

		(Regex::new(REGEX_PAT_IMMEDIATE).unwrap(), |op| (AddressMode::Immediate, vec![op as u8])),
		(Regex::new(REGEX_PAT_INDIRECT).unwrap(), |op| (AddressMode::Indirect, vec![lo8(op), hi8(op)])),
		(Regex::new(REGEX_PAT_INDIRECT_X).unwrap(), |op| (AddressMode::IndirectX, vec![op as u8])),
		(Regex::new(REGEX_PAT_INDIRECT_Y).unwrap(), |op| (AddressMode::IndirectY, vec![op as u8])),
		(Regex::new(REGEX_PAT_ABS_X).unwrap(), |op| {
			if op > u8::MAX as u16 {
				(AddressMode::AbsoluteX, vec![lo8(op), hi8(op)])
			} else {
				(AddressMode::ZeropageX, vec![op as u8])
			}
		}),

		(Regex::new(REGEX_PAT_ABS_Y).unwrap(), |op| {
			if op > u8::MAX as u16 {
				(AddressMode::AbsoluteY, vec![lo8(op), hi8(op)])
			} else {
				(AddressMode::ZeropageY, vec![op as u8])
			}
		}),
	];
}
