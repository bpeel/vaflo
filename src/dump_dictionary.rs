// Vaflo – A word game in Esperanto
// Copyright (C) 2023  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

mod dictionary;

use dictionary::Dictionary;
use std::process::ExitCode;

fn load_dictionary() -> Result<Dictionary, ()> {
    let filename = std::env::args_os()
        .nth(1)
        .unwrap_or("data/dictionary.bin".into());

    match std::fs::read(&filename) {
        Err(e) => {
            eprintln!(
                "{}: {}",
                filename.to_string_lossy(),
                e,
            );
            Err(())
        },
        Ok(d) => Ok(Dictionary::new(d.into_boxed_slice())),
    }
}

fn main() -> ExitCode {
    let Ok(dictionary) = load_dictionary()
    else {
        return ExitCode::FAILURE;
    };

    let mut iterator = dictionary::WordIterator::new(&dictionary);

    while let Some(word) = iterator.next() {
        println!("{}", word);
    }

    ExitCode::SUCCESS
}
