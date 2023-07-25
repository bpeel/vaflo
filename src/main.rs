mod permute;
mod dictionary;
mod word_solver;

use std::process::ExitCode;
use std::io;
use std::ffi::OsStr;
use dictionary::Dictionary;

fn load_dictionary(filename: &OsStr) -> Result<Dictionary, io::Error> {
    std::fs::read(filename).map(|data| Dictionary::new(data.into_boxed_slice()))
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

    let Some(letters) = letters.to_str()
    else {
        eprintln!("Invalid UTF-8 in letters");
        return ExitCode::FAILURE;
    };

    let mut letters: Vec<char> = letters.chars().collect();

    let mut word_iter = word_solver::Iter::new(
        &dictionary,
        &word_template,
        &mut letters
    );

    while let Some(word) = word_iter.next() {
        println!("{}", word);
    }

    ExitCode::SUCCESS
}
