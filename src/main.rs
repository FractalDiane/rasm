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

#[derive(PartialEq)]
pub enum Pass {
	Constant,
	Label,
	Main,
}

fn assemble(file: &File, pass: Pass, target: &Target, labels: &mut HashMap<String, u16>, constants: &mut HashMap<String, u16>) -> (Vec<u8>, u16) {
	let mut line_num = 1;
	let (mut program_counter, mut load_addr) = (0x0801usize, 0x0801u16);
	let mut code = vec![0, 0];

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
			let value = parse_expression(value_str, &constants, &labels)
				.expect(&format!("Line {}: Undefined label \"{}\"", line_num, &matches[2]));
			
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
							let byte = parse_expression(b.trim(), &constants, &labels);
							match byte {
								Some(b) => b as u8,
								None => {
									if pass == Pass::Label {
										u8::MAX
									} else {
										panic!("Line {}: Undefined symbol \"{}\"", line_num, &matches[2]);
									}
								},
							}
						}).collect::<Vec<u8>>();
						
						program_counter += bytes.len();
						if pass == Pass::Main {
							code.extend(bytes);
						}
					},
					"word" => {
						let words = matches[2].split(',').map(|w| {
							let word = parse_expression(w.trim(), &constants, &labels);
							match word {
								Some(w) => w,
								None => {
									if pass == Pass::Label {
										u16::MAX
									} else {
										panic!("Line {}: Undefined symbol \"{}\"", line_num, &matches[2]);
									}
								}
							}
						}).collect::<Vec<u16>>();

						program_counter += 2 * words.len();
						if pass == Pass::Main {
							code.extend(words.iter().fold(vec![], |mut vec, w| { vec.extend(vec![lo8(*w), hi8(*w)]); vec }));
						}
					},
					"addrstring" => {
						let bytes = match parse_expression(&matches[2], &constants, &labels) {
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
								if pass == Pass::Label {
									vec![b'0'; 5]
								} else {
									panic!("Line {}: Undefined label \"{}\"", line_num, &matches[2]);
								}
							},
						};

						program_counter += bytes.len();
						if pass == Pass::Main {
							code.extend(bytes);
						}
					},
					"string" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| char_format(c as u8, target)).collect::<Vec<u8>>();
						program_counter += vec.len();
						if pass == Pass::Main {
							code.extend(vec);
						}
					},
					"cstring" => {
						let text = matches[2].trim_matches('"');
						let vec = text.chars().map(|c| char_format(c as u8, target)).collect::<Vec<u8>>();
						program_counter += vec.len() + 1;
						if pass == Pass::Main {
							code.extend(vec);
							code.push(0);
						}
					},
					"cbmstring" => {
						let text = matches[2].trim_matches('"');
						let mut chars = text.chars().map(|c| char_format(c as u8, target)).collect::<Vec<u8>>();
						*chars.last_mut().unwrap() |= 0x80;
						program_counter += chars.len();
						if pass == Pass::Main {
							code.extend(chars);
						}
					},
					_ => {
						panic!("Line {}: Invalid pseudo-op \"{}\"", line_num, &matches[1]);
					},
				}
			}
		} else if let Some(matches) = regexes::REGEX_INSTR.captures(trimmed) {
			if pass != Pass::Constant {
				let mnemonic = &matches[1].to_lowercase();
				let operand = &matches.get(2).map_or("", |m| m.as_str());

				let bytes = get_instruction_bytes(mnemonic, operand, &constants, &labels, program_counter, line_num, &pass);
				program_counter += bytes.len();
				if pass == Pass::Main {
					code.extend(bytes);
				}
			}
		} else if !trimmed.is_empty() {
			panic!("Line {}: Invalid syntax\n\t{}", line_num, trimmed);
		}

		line_num += 1;
	}

	(code, load_addr)
}

fn main() {
	std::panic::set_hook(Box::new(|info| {
		if let Some(error) = info.payload().downcast_ref::<String>() {
			eprintln!("\x1b[0;91mERROR:\x1b[0m {}", error);
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
				output_file = args.peek().expect(&format!("{}", "No valid output file specified")).to_string();
				args.next().unwrap();
			},
			"-t" => {
				target = Target::from_string(args.peek().expect(&format!("{}", "No valid target specified")));
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
		panic!("{}", "No input file specified");
	}

	let mut infile = File::open(&input_file).expect(&format!("Failed to open input file {}", &input_file));

	let mut labels = HashMap::<String, u16>::new();
	let mut constants = HashMap::<String, u16>::new();

	assemble(&infile, Pass::Constant, &target, &mut labels, &mut constants);
	infile.seek(SeekFrom::Start(0)).expect("Error occurred while parsing file");
	assemble(&infile, Pass::Label, &target, &mut labels, &mut constants);
	infile.seek(SeekFrom::Start(0)).expect("Error occurred while parsing file");
	let (mut code, load_addr) = assemble(&infile, Pass::Main, &target, &mut labels, &mut constants);

	code[0] = lo8(load_addr);
	code[1] = hi8(load_addr);
	fs::write(&output_file, code).expect(&format!("Failed to write to output file {}", &output_file));
}
