// Vaflo – A word game in Esperanto
// Copyright (C) 2024  Neil Roberts
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

mod trie_builder;

use std::process::ExitCode;
use std::io::BufWriter;
use std::fs::File;
use trie_builder::TrieBuilder;

fn main() -> ExitCode {
    let Some(filename) = std::env::args().nth(1)
    else {
        eprintln!("usage: make-dictionary <output_filename>");
        return ExitCode::FAILURE;
    };

    let mut builder = TrieBuilder::new();

    for line in std::io::stdin().lines() {
        match line {
            Ok(word) => builder.add_word(&word),
            Err(e) => {
                eprintln!("{}", e);
                return ExitCode::FAILURE;
            },
        };
    }

    if let Err(e) = File::create(&filename).and_then(|file| {
        builder.into_dictionary(&mut BufWriter::new(file))
    }) {
        eprintln!("{}: {}", filename, e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
