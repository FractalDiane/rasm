extern crate lazy_static;
extern crate regex;
extern crate maplit;

mod instructions;
mod regexes;
mod utility;

use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, SeekFrom};
use std::collections::HashMap;
use std::io::Seek;

use crate::utility::*;
use crate::instructions::get_instruction_bytes;

#[derive(PartialEq, Debug)]
pub enum Pass {
	Constant,
	Label,
	Main,
}

fn assemble(file: &File, pass: Pass, labels: &mut HashMap<String, u16>, constants: &mut HashMap<String, u16>) -> (Vec<u8>, u16) {
	let mut line_num = 1;
	let (mut program_counter, mut load_addr) = (0x0801usize, 0x0801u16);
	let mut code = vec![0, 0];

	for ln in io::BufReader::new(file).lines() {
		let line = ln.unwrap();
		let trimmed = line.trim();
		if let Some(matches) = regexes::REGEX_ASSIGN.captures(trimmed) {
			let name = &matches[1];
			let value_str = &matches[2];
			let value = parse_value(value_str, &constants, &labels).expect("TODO: Undefined Symbol");

			if name == "*" {
				load_addr = value;
				program_counter = value as usize;
			} else if pass == Pass::Constant {
				constants.insert(name.into(), value);
			}
		} else if let Some(matches) = regexes::REGEX_LABEL.captures(trimmed) {
			if pass == Pass::Label {
				labels.insert(matches[1].into(), program_counter as u16);
			}
		} else if let Some(matches) = regexes::REGEX_PSEUDO.captures(trimmed) {
			if pass != Pass::Constant {
				match &matches[1] {
					"byte" => {
						let bytes = matches[2].split(',').map(|b| {
							let byte = parse_value(b.trim(), &constants, &labels);
							match byte {
								Some(b) => b as u8,
								None => {
									if pass == Pass::Label {
										0
									} else {
										panic!("TODO: Undefined symbol");
									}
								},
							}
						}).collect::<Vec<u8>>();
						
						program_counter += bytes.len();
						if pass == Pass::Main {
							code.extend(bytes);
						}
					},
					"text" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| c as u8).collect::<Vec<u8>>();
						program_counter += vec.len();
						if pass == Pass::Main {
							code.extend(vec);
						}
					},
					"null" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| c as u8).collect::<Vec<u8>>();
						program_counter += vec.len() + 1;
						if pass == Pass::Main {
							code.extend(vec);
							code.push(0);
						}
					},
					"shift" => {
						let text = matches[2].trim_matches('"');
						let mut chars = text.chars().map(|c| c as u8).collect::<Vec<u8>>();
						*chars.last_mut().unwrap() |= 0x80;
						program_counter += chars.len();
						if pass == Pass::Main {
							code.extend(chars);
						}
					},
					_ => {
						panic!("Invalid pseudo-op \"{}\"", &matches[1]);
					},
				}
			}
		} else if let Some(matches) = regexes::REGEX_INSTR.captures(trimmed) {
			if pass != Pass::Constant {
				let mnemonic = &matches[1].to_lowercase();
				let operand = &matches.get(2).map_or("", |m| m.as_str());

				let bytes = get_instruction_bytes(mnemonic, operand, &constants, &labels, program_counter, &pass);
				program_counter += bytes.len();
				if pass == Pass::Main {
					code.extend(bytes);
				}
			}
		} else if !trimmed.is_empty() && !trimmed.starts_with(';') {
			panic!("Invalid syntax in line {}: {}", line_num, trimmed);
		}

		line_num += 1;
	}

	(code, load_addr)
}

fn main() {
	let mut input_file = String::new();
	let mut output_file = String::new();

	let mut args = env::args().peekable();
	args.next();
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"-o" => {
				output_file = args.peek().unwrap().to_string();
				args.next();
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
			},
		}
	}

	let mut infile = File::open(input_file).unwrap();

	let mut labels = HashMap::<String, u16>::new();
	let mut constants = HashMap::<String, u16>::new();

	assemble(&infile, Pass::Constant, &mut labels, &mut constants);
	infile.seek(SeekFrom::Start(0)).unwrap();
	assemble(&infile, Pass::Label, &mut labels, &mut constants);
	infile.seek(SeekFrom::Start(0)).unwrap();
	let (mut code, load_addr) = assemble(&infile, Pass::Main, &mut labels, &mut constants);

	code[0] = lo8(load_addr);
	code[1] = hi8(load_addr);
	fs::write(output_file, code).unwrap();
}
