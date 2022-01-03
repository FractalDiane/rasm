extern crate lazy_static;
extern crate regex;
extern crate maplit;

mod instructions;
mod regexes;
mod target;
mod utility;

use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, SeekFrom};
use std::collections::HashMap;
use std::io::Seek;

use crate::utility::*;
use crate::target::*;
use crate::instructions::get_instruction_bytes;

use maplit::hashmap;

#[derive(PartialEq)]
pub enum Pass {
	Constant,
	Label,
	Main,
}

pub struct AssemblyState {
	pub target: Target,
	pub pass: Pass,

	pub line_num: usize,
	pub program_counter: usize,

	pub current_block: isize,
	pub max_block: isize,

	pub labels: HashMap<isize, HashMap<String, u16>>,
	pub constants: HashMap<String, u16>,
}

pub struct ErrorMsg {
	msg: String,
	line_num: usize,
}

impl ErrorMsg {
	pub fn new(msg: String, line_num: usize) -> Self {
		Self{msg, line_num}
	}
}

#[macro_export]
macro_rules! rasm_error {
	($line:expr, $fmt:literal, $($arg:tt)*) => {
		std::panic::panic_any(ErrorMsg::new(
			format!($fmt, $($arg)*),
			$line,
		));
	};
}

fn assemble(file: &File, assembly_state: &mut AssemblyState) -> (Vec<u8>, u16) {
	let mut load_addr = 0x0801u16;
	let mut code = vec![0, 0];
	assembly_state.line_num = 1;
	assembly_state.current_block = -1;
	assembly_state.max_block = -1;

	for ln in io::BufReader::new(file).lines() {
		let line = ln.unwrap();
		let pretrimmed = line.trim();
		let trimmed = {
			if let Some(idx) = pretrimmed.find(';') {
				&pretrimmed[0..idx].trim_end()
			} else {
				pretrimmed
			}
		};

		if let Some(matches) = regexes::REGEX_ASSIGN.captures(trimmed) {
			let name = &matches[1];
			let value_str = &matches[2];
			let value = match parse_expression(value_str, assembly_state) {
				Some(val) => val,
				None => {
					rasm_error!(assembly_state.line_num, "Unidentified label \"{}\"", &matches[2]);
				},
			};
			
			if name == "*" {
				load_addr = value;
				assembly_state.program_counter = value as usize;
			} else if assembly_state.pass == Pass::Constant {
				assembly_state.constants.insert(name.into(), value);
			}
		} else if let Some(matches) = regexes::REGEX_LABEL.captures(trimmed) {
			if assembly_state.pass == Pass::Label {
				let current_labels = assembly_state.labels.get_mut(&assembly_state.current_block).unwrap();
				current_labels.insert(matches[1].into(), assembly_state.program_counter as u16);
			}
		} else if let Some(matches) = regexes::REGEX_PSEUDO.captures(trimmed) {
			if assembly_state.pass != Pass::Constant {
				match &matches[1] {
					"block" => {
						assembly_state.max_block += 1;
						assembly_state.current_block = assembly_state.max_block;
						if assembly_state.pass == Pass::Label {
							assembly_state.labels.insert(assembly_state.current_block, hashmap!{});
						}
					},
					"bend" => {
						assembly_state.current_block = -1;
					},
					"byte" => {
						let bytes = matches[2].split(',').map(|b| {
							let byte = parse_expression(b.trim(), assembly_state);
							match byte {
								Some(b) => b as u8,
								None => {
									if assembly_state.pass == Pass::Label {
										u8::MAX
									} else {
										rasm_error!(assembly_state.line_num, "Undefined symbol \"{}\"", &matches[2]);
									}
								},
							}
						}).collect::<Vec<u8>>();
						
						assembly_state.program_counter += bytes.len();
						if assembly_state.pass == Pass::Main {
							code.extend(bytes);
						}
					},
					"word" => {
						let words = matches[2].split(',').map(|w| {
							let word = parse_expression(w.trim(), assembly_state);
							match word {
								Some(w) => w,
								None => {
									if assembly_state.pass == Pass::Label {
										u16::MAX
									} else {
										rasm_error!(assembly_state.line_num, "Undefined symbol \"{}\"", &matches[2]);
									}
								}
							}
						}).collect::<Vec<u16>>();

						assembly_state.program_counter += 2 * words.len();
						if assembly_state.pass == Pass::Main {
							code.extend(words.iter().fold(vec![], |mut vec, w| { vec.extend(vec![lo8(*w), hi8(*w)]); vec }));
						}
					},
					"addrstring" => {
						let bytes = match parse_expression(&matches[2], assembly_state) {
							Some(v) => {
								let mut vec = vec![];
								let string = v.to_string();
								vec.extend(string.chars().map(|c| c as u8));
								while vec.len() < 5 {
									vec.insert(0, b'0');
								}

								vec
							},
							None => {
								if assembly_state.pass == Pass::Label {
									vec![b'0'; 5]
								} else {
									rasm_error!(assembly_state.line_num, "Undefined label \"{}\"", &matches[2]);
								}
							},
						};

						assembly_state.program_counter += bytes.len();
						if assembly_state.pass == Pass::Main {
							code.extend(bytes);
						}
					},
					"string" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| char_format(c as u8, &assembly_state.target)).collect::<Vec<u8>>();
						assembly_state.program_counter += vec.len();
						if assembly_state.pass == Pass::Main {
							code.extend(vec);
						}
					},
					"cstring" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| char_format(c as u8, &assembly_state.target)).collect::<Vec<u8>>();
						assembly_state.program_counter += vec.len() + 1;
						if assembly_state.pass == Pass::Main {
							code.extend(vec);
							code.push(0);
						}
					},
					"cbmstring" => {
						let text = matches[2].trim_matches('"');
						let mut chars = text.chars().map(|c| char_format(c as u8, &assembly_state.target)).collect::<Vec<u8>>();
						*chars.last_mut().unwrap() |= 0x80;
						assembly_state.program_counter += chars.len();
						if assembly_state.pass == Pass::Main {
							code.extend(chars);
						}
					},
					_ => {
						rasm_error!(assembly_state.line_num, "Invalid pseudo-op \"{}\"", &matches[1]);
					},
				}
			}
		} else if let Some(matches) = regexes::REGEX_INSTR.captures(trimmed) {
			if assembly_state.pass != Pass::Constant {
				let mnemonic = &matches[1].to_lowercase();
				let operand = &matches.get(2).map_or("", |m| m.as_str());

				let bytes = get_instruction_bytes(mnemonic, operand, assembly_state);
				assembly_state.program_counter += bytes.len();
				if assembly_state.pass == Pass::Main {
					code.extend(bytes);
				}
			}
		} else if !trimmed.is_empty() {
			rasm_error!(assembly_state.line_num, "Invalid syntax \"{}\"", trimmed);
		}

		assembly_state.line_num += 1;
	}

	(code, load_addr)
}

fn main() {
	std::panic::set_hook(Box::new(|info| {
		if let Some(error) = info.payload().downcast_ref::<ErrorMsg>() {
			let line_str = if error.line_num > 0 {
				format!("Line {}:", error.line_num)
			} else {
				String::new()
			};

			eprintln!("\x1b[0;91mERROR:\x1b[0m {} {}", line_str, error.msg);
		} else {
			eprintln!("\x1b[0;91mERROR:\x1b[0m (Couldn't parse error)");
		}
	}));

	let mut target = Target::C64;

	let mut input_file = String::new();
	let mut output_file = String::new();
	let mut input_file_given = false;

	let mut args = env::args().peekable();
	args.next().unwrap();
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"-o" => {
				output_file = args.peek().unwrap_or_else(
					|| rasm_error!(0, "{}", "No valid output file specified")
				).to_string();
				
				args.next().unwrap();
			},
			"-t" => {
				target = Target::from_string(args.peek().unwrap_or_else(
					|| rasm_error!(0, "{}", "No valid target specified")
				));
				args.next().unwrap();
			},
			_ => {
				input_file = arg.to_string();
				if output_file.is_empty() {
					let file = arg.to_string();
					output_file = match file.find('.') {
						Some(pos) => {
							String::from(&file[..pos]) + ".prg".into()
						},
						None => {
							file + ".prg".into()
						},
					}
				}

				input_file_given = true;
			},
		}
	}

	if !input_file_given {
		rasm_error!(0, "{}", "No input file specified");
	}

	let mut infile = File::open(&input_file).unwrap_or_else(
		|_| rasm_error!(0, "Failed to open input file {}", &input_file)
	);

	let constants = HashMap::<String, u16>::new();
	let mut labels = HashMap::<isize, HashMap<String, u16>>::new();
	labels.insert(-1, hashmap!{});

	let mut assembly_state = AssemblyState{
		target, pass: Pass::Constant,
		line_num: 1, program_counter: 0,
		current_block: -1, max_block: -1,
		constants, labels,
	};

	assemble(&infile, &mut assembly_state);
	infile.seek(SeekFrom::Start(0)).unwrap_or_else(|_| rasm_error!(0, "{}", "Error occurred while parsing file"));
	assembly_state.pass = Pass::Label;
	assemble(&infile, &mut assembly_state);
	infile.seek(SeekFrom::Start(0)).unwrap_or_else(|_| rasm_error!(0, "{}", "Error occurred while parsing file"));
	assembly_state.pass = Pass::Main;
	let (mut code, load_addr) = assemble(&infile, &mut assembly_state);

	code[0] = lo8(load_addr);
	code[1] = hi8(load_addr);
	fs::write(&output_file, code).unwrap_or_else(
		|_| rasm_error!(0, "Failed to write to output file {}", &output_file)
	);
}
