use std::collections::HashMap;

#[inline(always)]
pub fn lo8(n: u16) -> u8 {
	(n & 0xff) as u8
}

#[inline(always)]
pub fn hi8(n: u16) -> u8 {
	(n >> 8) as u8
}

enum Operation {
	None,
	Lo8,
	Hi8,
}

pub fn parse_value(value: &str, constants: &HashMap<String, u16>, labels: &HashMap<String, u16>) -> Option<u16> {
	let (val, op) = if value.starts_with('<') {
		(&value[1..], Operation::Lo8)
	} else if value.starts_with('>') {
		(&value[1..], Operation::Hi8)
	} else {
		(value, Operation::None)
	};

	let result = match parse_num(val) {
		Ok(num) => Some(num),
		Err(_) => {
			let string = val.to_string();
			if constants.contains_key(&string) {
				Some(constants[&string])
			} else if labels.contains_key(&string) {
				Some(labels[&string])
			} else {
				None
			}
		},
	};

	match op {
		Operation::None => result,
		Operation::Lo8 => {
			match result {
				Some(r) => Some(lo8(r) as u16),
				None => None,
			}
		},
		Operation::Hi8 => {
			match result {
				Some(r) => Some(hi8(r) as u16),
				None => None,
			}
		},
	}
}

pub fn parse_num(num: &str) -> Result<u16, std::num::ParseIntError> {
	if num.starts_with('$') {
		u16::from_str_radix(&num[1..], 16)
	} else if num.starts_with('%') {
		u16::from_str_radix(&num[1..], 2)
	} else {
		u16::from_str_radix(num, 10)
	}
}
