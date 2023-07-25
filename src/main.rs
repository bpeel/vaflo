mod permute;
mod dictionary;
mod word_solver;
mod grid;

use std::process::ExitCode;
use std::io;
use std::ffi::OsStr;
use dictionary::Dictionary;

fn load_dictionary(filename: &OsStr) -> Result<Dictionary, io::Error> {
    std::fs::read(filename).map(|data| Dictionary::new(data.into_boxed_slice()))
}

fn convert_word_template(s: &str) -> Result<grid::Word, String> {
    let mut word = grid::Word {
        letters: [
            grid::Letter { value: 'a', state: grid::LetterState::Movable };
            grid::WORD_LENGTH
        ],
    };

    let mut chars = s.chars();

    for letter in word.letters.iter_mut() {
        match chars.next() {
            Some(ch) => {
                if ch != '.' {
                    letter.state = grid::LetterState::Fixed;
                    letter.value = ch;
                }
            },
            None => {
                return Err("word too short".to_string());
            },
        }
    }

    if chars.next().is_some() {
        Err("word too long".to_string())
    } else {
        Ok(word)
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();

    if args.len() != 4 {
        eprintln!("usage: solve-waffle <dictionary> <word_template> <letters>");
        return ExitCode::FAILURE;
    }

    let dictionary_filename = args.nth(1).unwrap();
    let word_template = args.next().unwrap();
    let letters = args.next().unwrap();

    let dictionary = match load_dictionary(&dictionary_filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", dictionary_filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        }
    };

    let Some(word_template) = word_template.to_str()
    else {
        eprintln!("Invalid UTF-8 in word");
        return ExitCode::FAILURE;
    };

    let word_template = match convert_word_template(word_template) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        },
    };

    let Some(letters) = letters.to_str()
    else {
        eprintln!("Invalid UTF-8 in letters");
        return ExitCode::FAILURE;
    };

    let mut word_iter = word_solver::Iter::new(
        &dictionary,
        word_template,
        letters.chars().collect(),
    );

    while let Some(word) = word_iter.next() {
        println!("{}", word);
    }

    ExitCode::SUCCESS
}
