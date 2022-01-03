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

pub fn parse_expression(expression: &str, constants: &HashMap<String, u16>, labels: &HashMap<String, u16>) -> Option<u16> {
	let (expr, op) = if expression.starts_with('<') {
		(&expression[1..], Operation::Lo8)
	} else if expression.starts_with('>') {
		(&expression[1..], Operation::Hi8)
	} else {
		(expression, Operation::None)
	};
	
	let result = if expr.contains('+') {
		let mut split = expr.split('+');
		let left = split.next().unwrap().trim();
		let right = split.next().unwrap().trim();
		let rleft = parse_value(left, constants, labels);
		let rright = parse_value(right, constants, labels);
		
		if rleft.is_some() && rright.is_some() {
			Some(rleft.unwrap() + rright.unwrap())
		} else {
			None
		}
	} else if expr.contains('-') {
		let mut split = expr.split('-');
		let left = split.next().unwrap().trim();
		let right = split.next().unwrap().trim();
		let rleft = parse_value(left, constants, labels);
		let rright = parse_value(right, constants, labels);
		
		if rleft.is_some() && rright.is_some() {
			Some(rleft.unwrap() - rright.unwrap())
		} else {
			None
		}
	} else {
		parse_value(expr, constants, labels)
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

fn parse_value(value: &str, constants: &HashMap<String, u16>, labels: &HashMap<String, u16>) -> Option<u16> {
	match parse_num(value) {
		Some(num) => Some(num),
		None => {
			let string = value.to_string();
			if constants.contains_key(&string) {
				Some(constants[&string])
			} else if labels.contains_key(&string) {
				Some(labels[&string])
			} else {
				None
			}
		},
	}
}

pub fn parse_num(num: &str) -> Option<u16> {
	if num.starts_with('$') {
		u16::from_str_radix(&num[1..], 16).ok()
	} else if num.starts_with('%') {
		u16::from_str_radix(&num[1..], 2).ok()
	} else if num.starts_with('"') || num.starts_with('\'') {
		match &num[1..=1].parse::<char>() {
			Ok(c) => Some((*c as u8) as u16),
			Err(_) => None,
		}
	} else {
		u16::from_str_radix(num, 10).ok()
	}
}
