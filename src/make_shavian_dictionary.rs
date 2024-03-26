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

mod shavian;
mod trie_builder;

use std::process::ExitCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::fs::File;
use trie_builder::TrieBuilder;

static DICTIONARY_FILENAME: &'static str = "data/dictionary.bin";
static LATIN_MAP_FILENAME: &'static str = "data/latin-map.txt";

#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "Latn")]
    latin: String,
    #[serde(rename = "Shaw")]
    shavian: String,
    pos: String,
    var: String,
}

static BANNED_POSITIONS: [&'static str; 1] = [
    "NP0",
];

static ALLOWED_VARIATIONS: [&'static str; 1] = [
    "RRP",
];

impl Entry {
    fn is_allowed(&self) -> bool {
        // Allow only shavian letters, ie, no punctuation
        self.shavian.chars().all(|ch| shavian::is_shavian(ch))
        // Must be five letters long
            && self.shavian.chars().count() == 5
        // No banned positions
            && BANNED_POSITIONS.iter().find(|&p| p == &self.pos).is_none()
        // Only certain variations allowed
            && ALLOWED_VARIATIONS.iter().find(|&v| v == &self.var).is_some()
    }
}

type ReadLexMap = HashMap<String, Vec<Entry>>;

fn main() -> ExitCode {
    let map = match serde_json::from_reader::<_, ReadLexMap>(std::io::stdin()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        },
    };

    let mut builder = TrieBuilder::new();
    let mut latin_map = HashMap::<&str, &str>::new();

    for entry in map.values().flatten() {
        if entry.is_allowed() {
            builder.add_word(&entry.shavian);
            latin_map.insert(&entry.shavian, &entry.latin);
        }
    }

    if let Err(e) = File::create(DICTIONARY_FILENAME).and_then(|file| {
        builder.into_dictionary(&mut BufWriter::new(file))
    }) {
        eprintln!("{}: {}", DICTIONARY_FILENAME, e);
        return ExitCode::FAILURE;
    }

    let mut keys = latin_map.keys().map(|&k| k).collect::<Vec<&str>>();

    keys.sort();

    if let Err(e) = File::create(LATIN_MAP_FILENAME).and_then(|file| {
        let mut file = BufWriter::new(file);

        for &key in keys.iter() {
            writeln!(file, "{} {}", key, latin_map[key])?;
        }

        Ok(())
    }) {
        eprintln!("{}: {}", LATIN_MAP_FILENAME, e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
