use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;

use crate::regexes;
use crate::utility::*;
use crate::Pass;

#[derive(Hash, PartialEq, Eq)]
pub enum AddressMode {
	Implied,
	Immediate,
	Absolute,
	AbsoluteX,
	AbsoluteY,
	Zeropage,
	ZeropageX,
	ZeropageY,
	Relative,
	Indirect,
	IndirectX,
	IndirectY,
}

fn addr_default(op_str: &str, mnemonic_map: &HashMap<AddressMode, u8>, constants: &HashMap<String, u16>, labels: &HashMap<String, u16>, program_counter: usize, line_num: usize, pass: &Pass) -> (AddressMode, Vec<u8>) {
	if op_str.is_empty() || op_str == "a" {
		return (AddressMode::Implied, vec![]);
	}

	let op = match parse_expression(op_str, constants, labels) {
		Some(v) => v,
		None => {
			if *pass != Pass::Main {
				0xffff
			} else {
				panic!("Line {}: Undefined symbol \"{}\"", line_num, op_str);
			}
		},
	};

	if mnemonic_map.contains_key(&AddressMode::Relative) {
		let diff = !(program_counter as isize - op as isize) - 1;
		(AddressMode::Relative, vec![diff as u8])
	} else {
		if op < u8::MAX as u16 {
			(AddressMode::Zeropage, vec![op as u8])
		} else {
			(AddressMode::Absolute, vec![lo8(op), hi8(op)])
		}
	}
}

pub fn get_instruction_bytes(mnemonic: &str, operand: &str, constants: &HashMap<String, u16>, labels: &HashMap<String, u16>, program_counter: usize, line_num: usize, pass: &Pass) -> Vec<u8> {
	let mut addr_mode = AddressMode::Implied;
	let mut operand_vec = Vec::<u8>::new();
	let mut matched = false;

	for regex in regexes::ADDR_REGEXES.iter() {
		if let Some(matches) = regex.0.captures(operand) {
			let op = match parse_expression(&matches[1], constants, labels) {
				Some(o) => o,
				None => {
					if *pass == Pass::Label {
						u16::MAX
					} else {
						panic!("Line {}: Undefined symbol \"{}\"", line_num, &matches[1]);
					}
				},
			};

			let result = (regex.1)(op);

			addr_mode = result.0;
			operand_vec = result.1;
			matched = true;
			break;
		}
	}

	let mnemonic_map = OPCODES.get(mnemonic).unwrap();

	if !matched {
		let result = addr_default(operand, &mnemonic_map, constants, labels, program_counter, line_num, pass);
		addr_mode = result.0;
		operand_vec = result.1;
	}

	let opcode = *mnemonic_map.get(&addr_mode).unwrap();

	let mut ret = vec![opcode];
	ret.extend(operand_vec);
	ret
}

lazy_static! {
	pub static ref OPCODES: HashMap<&'static str, HashMap<AddressMode, u8>> = {
		type A = AddressMode;

		hashmap!{
			"adc" => hashmap!{
				A::Immediate => 0x69,
				A::Zeropage => 0x65,
				A::ZeropageX => 0x75,
				A::Absolute => 0x6d,
				A::AbsoluteX => 0x7d,
				A::AbsoluteY => 0x79,
				A::IndirectX => 0x61,
				A::IndirectY => 0x71,
			},

			"and" => hashmap!{
				A::Immediate => 0x29,
				A::Zeropage => 0x25,
				A::ZeropageX => 0x35,
				A::Absolute => 0x2d,
				A::AbsoluteX => 0x3d,
				A::AbsoluteY => 0x39,
				A::IndirectX => 0x21,
				A::IndirectY => 0x31,
			},

			"asl" => hashmap!{
				A::Implied => 0x0a,
				A::Zeropage => 0x06,
				A::ZeropageX => 0x16,
				A::Absolute => 0x0e,
				A::AbsoluteX => 0x1e,
			},

			"bcc" => hashmap!{
				A::Relative => 0x90,
			},

			"bcs" => hashmap!{
				A::Relative => 0xb0,
			},
			
			"beq" => hashmap!{
				A::Relative => 0xf0,
			},

			"bit" => hashmap!{
				A::Zeropage => 0x24,
				A::Absolute => 0x2c,
			},

			"bmi" => hashmap!{
				A::Relative => 0x30,
			},

			"bne" => hashmap!{
				A::Relative => 0xd0,
			},

			"bpl" => hashmap!{
				A::Relative => 0x10,
			},

			"brk" => hashmap!{
				A::Implied => 0x00,
			},

			"bvc" => hashmap!{
				A::Relative => 0x50,
			},

			"bvs" => hashmap!{
				A::Relative => 0x70,
			},

			"clc" => hashmap!{
				A::Implied => 0x18,
			},

			"cld" => hashmap!{
				A::Implied => 0xd8,
			},

			"cli" => hashmap!{
				A::Implied => 0x58,
			},

			"clv" => hashmap!{
				A::Implied => 0xb8,
			},

			"cmp" => hashmap!{
				A::Immediate => 0xc9,
				A::Zeropage => 0xc5,
				A::ZeropageX => 0xd5,
				A::Absolute => 0xcd,
				A::AbsoluteX => 0xdd,
				A::AbsoluteY => 0xd9,
				A::IndirectX => 0xc1,
				A::IndirectY => 0xd1,
			},

			"cpx" => hashmap!{
				A::Immediate => 0xe0,
				A::Zeropage => 0xe4,
				A::Absolute => 0xec,
			},

			"cpy" => hashmap!{
				A::Immediate => 0xc0,
				A::Zeropage => 0xc4,
				A::Absolute => 0xcc,
			},

			"dec" => hashmap!{
				A::Zeropage => 0xc6,
				A::ZeropageX => 0xd6,
				A::Absolute => 0xce,
				A::AbsoluteX => 0xde,
			},

			"dex" => hashmap!{
				A::Implied => 0xca,
			},

			"dey" => hashmap!{
				A::Implied => 0x88,
			},

			"eor" => hashmap!{
				A::Immediate => 0x49,
				A::Zeropage => 0x45,
				A::ZeropageX => 0x55,
				A::Absolute => 0x4d,
				A::AbsoluteX => 0x5d,
				A::AbsoluteY => 0x59,
				A::IndirectX => 0x41,
				A::IndirectY => 0x51,
			},

			"inc" => hashmap!{
				A::Zeropage => 0xe6,
				A::ZeropageX => 0xf6,
				A::Absolute => 0xee,
				A::AbsoluteX => 0xfe,
			},

			"inx" => hashmap!{
				A::Implied => 0xe8,
			},

			"iny" => hashmap!{
				A::Implied => 0xc8,
			},

			"jmp" => hashmap!{
				A::Absolute => 0x4c,
				A::Indirect => 0x6c,
			},

			"jsr" => hashmap!{
				A::Absolute => 0x20,
			},

			"lda" => hashmap!{
				A::Immediate => 0xa9,
				A::Zeropage => 0xa5,
				A::ZeropageX => 0xb5,
				A::Absolute => 0xad,
				A::AbsoluteX => 0xbd,
				A::AbsoluteY => 0xb9,
				A::IndirectX => 0xa1,
				A::IndirectY => 0xb1,
			},

			"ldx" => hashmap!{
				A::Immediate => 0xa2,
				A::Zeropage => 0xa6,
				A::ZeropageY => 0xb6,
				A::Absolute => 0xae,
				A::AbsoluteY => 0xbe,
			},

			"ldy" => hashmap!{
				A::Immediate => 0xa0,
				A::Zeropage => 0xa4,
				A::ZeropageX => 0xb4,
				A::Absolute => 0xac,
				A::AbsoluteX => 0xbc,
			},

			"lsr" => hashmap!{
				A::Implied => 0x4a,
				A::Zeropage => 0x46,
				A::ZeropageX => 0x56,
				A::Absolute => 0x4e,
				A::AbsoluteX => 0x5e,
			},

			"nop" => hashmap!{
				A::Implied => 0xea,
			},

			"ora" => hashmap!{
				A::Immediate => 0x09,
				A::Zeropage => 0x05,
				A::ZeropageX => 0x15,
				A::Absolute => 0x0d,
				A::AbsoluteX => 0x1d,
				A::AbsoluteY => 0x19,
				A::IndirectX => 0x01,
				A::IndirectY => 0x11,
			},

			"pha" => hashmap!{
				A::Implied => 0x48,
			},

			"php" => hashmap!{
				A::Implied => 0x08,
			},

			"pla" => hashmap!{
				A::Implied => 0x68,
			},

			"plp" => hashmap!{
				A::Implied => 0x28,
			},

			"rol" => hashmap!{
				A::Implied => 0x2a,
				A::Zeropage => 0x26,
				A::ZeropageX => 0x36,
				A::Absolute => 0x2e,
				A::AbsoluteX => 0x3e,
			},

			"ror" => hashmap!{
				A::Implied => 0x6a,
				A::Zeropage => 0x66,
				A::ZeropageX => 0x76,
				A::Absolute => 0x6e,
				A::AbsoluteX => 0x7e,
			},

			"rti" => hashmap!{
				A::Implied => 0x40,
			},

			"rts" => hashmap!{
				A::Implied => 0x60,
			},

			"sbc" => hashmap!{
				A::Immediate => 0xe9,
				A::Zeropage => 0xe5,
				A::ZeropageX => 0xf5,
				A::Absolute => 0xed,
				A::AbsoluteX => 0xfd,
				A::AbsoluteY => 0xf9,
				A::IndirectX => 0xe1,
				A::IndirectY => 0xf1,
			},

			"sec" => hashmap!{
				A::Implied => 0x38,
			},

			"sed" => hashmap!{
				A::Implied => 0xf8,
			},

			"sei" => hashmap!{
				A::Implied => 0x78,
			},

			"sta" => hashmap!{
				A::Zeropage => 0x85,
				A::ZeropageX => 0x95,
				A::Absolute => 0x8d,
				A::AbsoluteX => 0x9d,
				A::AbsoluteY => 0x99,
				A::IndirectX => 0x81,
				A::IndirectY => 0x91,
			},

			"stx" => hashmap!{
				A::Zeropage => 0x86,
				A::ZeropageY => 0x96,
				A::Absolute => 0x8e,
			},

			"sty" => hashmap!{
				A::Zeropage => 0x84,
				A::ZeropageY => 0x94,
				A::Absolute => 0x8c,
			},

			"tax" => hashmap!{
				A::Implied => 0xaa,
			},

			"tay" => hashmap!{
				A::Implied => 0xa8,
			},

			"tsx" => hashmap!{
				A::Implied => 0xba,
			},

			"txa" => hashmap!{
				A::Implied => 0x8a,
			},

			"txs" => hashmap!{
				A::Implied => 0x9a,
			},

			"tya" => hashmap!{
				A::Implied => 0x98,
			},
		}
	};
}
