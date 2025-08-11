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

use super::dictionary::{Dictionary, WordIterator};
use super::wildcard;

fn pattern_matches(pattern: &str, word: &str) -> bool {
    let mut word_chars = word.chars();

    for pattern_ch in pattern.chars().map(char::to_lowercase).flatten() {
        let Some(word_ch) = word_chars.next()
        else {
            return false;
        };

        if !wildcard::matches(pattern_ch, word_ch) {
            return false;
        }
    }

    word_chars.next().is_none()
}

pub fn search(pattern: &str, dictionary: &Dictionary) -> Vec<String> {
    let mut iterator = WordIterator::new(dictionary);
    let mut results = Vec::new();

    while let Some(word) = iterator.next() {
        if pattern_matches(pattern, word) {
            results.push(word.to_string());
        }
    }

    results
}

#[cfg(test)]
mod test {
    use super::*;

    // Dictionary with the words: etoso, haŭto, ninĵo, ratoj
    static DICTIONARY_DATA: [u8; 62] = [
        0x00, 0x01, 0x2a, 0x01, 0x13, 0x65, 0x01, 0x0a, 0x68, 0x01, 0x0a, 0x6e,
        0x00, 0x01, 0x72, 0x00, 0x10, 0x61, 0x00, 0x10, 0x61, 0x00, 0x04, 0x69,
        0x00, 0x04, 0x74, 0x00, 0x14, 0x6e, 0x00, 0x0b, 0x6f, 0x00, 0x05, 0x74,
        0x00, 0x08, 0xc5, 0xad, 0x00, 0x0b, 0x6f, 0x00, 0x0b, 0x73, 0x00, 0x08,
        0x74, 0x00, 0x05, 0xc4, 0xb5, 0x00, 0x04, 0x6a, 0x00, 0x01, 0x6f, 0x00,
        0x00, 0x00,
    ];

    #[test]
    fn simple_search() {
        let dictionary = Dictionary::new(Box::new(DICTIONARY_DATA.clone()));

        assert_eq!(search("y.y.Y", &dictionary), &["etoso", "ninĵo"]);
        assert_eq!(search("....o", &dictionary), &["etoso", "haŭto", "ninĵo"]);
        assert_eq!(search("......", &dictionary), &[""; 0]);
        assert_eq!(search("....", &dictionary), &[""; 0]);
    }
}
