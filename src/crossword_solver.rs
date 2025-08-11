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
use super::grid::{SolutionGrid, WORD_LENGTH};
use super::wildcard;
use std::collections::HashMap;

pub struct Crossword {
    pub cross_letter: char,
    pub a_words: Vec<String>,
    pub b_words: Vec<String>,
}

fn word_matches(
    pattern: &str,
    word: &str,
    cross_point: usize,
) -> Option<char> {
    let mut cross_letter = None;
    let mut word_chars = word.chars();

    for (i, pattern_ch) in pattern.chars().enumerate() {
        let Some(word_ch) = word_chars.next()
        else {
            return None;
        };

        if i == cross_point {
            cross_letter = Some(word_ch);
        } else if i & 1 == 0 && !wildcard::matches(pattern_ch, word_ch) {
            return None;
        }
    }

    if word_chars.next().is_some() {
        return None;
    }

    cross_letter
}

fn collect_words(
    pattern: &str,
    cross_point: usize,
    dictionary: &Dictionary,
) -> HashMap<char, Vec<String>> {
    let mut result = HashMap::<char, Vec<_>>::new();

    let mut words = WordIterator::new(&dictionary);

    while let Some(word) = words.next() {
        if let Some(cross_letter) = word_matches(pattern, word, cross_point) {
            result.entry(cross_letter)
                .and_modify(|v| v.push(word.to_string()))
                .or_insert_with(|| vec![word.to_string()]);
        }
    }

    result
}

fn find_crosswords_with_patterns(
    word_a: &str,
    cross_point_a: usize,
    word_b: &str,
    cross_point_b: usize,
    dictionary: &Dictionary,
) -> Vec<Crossword> {
    let a_words = collect_words(word_a, cross_point_a, dictionary);
    let mut b_words = collect_words(word_b, cross_point_b, dictionary);

    let mut crosswords = a_words.into_iter()
        .filter_map(|(cross_letter, a_words)| {
            b_words.remove(&cross_letter).map(|b_words| {
                Crossword {
                    cross_letter,
                    a_words,
                    b_words,
                }
            })
        })
        .collect::<Vec<Crossword>>();

    crosswords.sort_unstable_by_key(|crossword| crossword.cross_letter);

    crosswords
}

pub fn find_crosswords(
    solution: &SolutionGrid,
    cross_x: i32,
    cross_y: i32,
    dictionary: &Dictionary,
) -> Vec<Crossword> {
    let horizontal_word = solution.letters[
        cross_y as usize
            * WORD_LENGTH
            ..(cross_y as usize + 1) * WORD_LENGTH
    ].into_iter()
        .map(|ch| ch.to_lowercase())
        .flatten()
        .collect::<String>();

    let vertical_word = (0..WORD_LENGTH)
        .map(|y| {
            let pos = y * WORD_LENGTH + cross_x as usize;
            solution.letters[pos].to_lowercase()
        })
        .flatten()
        .collect::<String>();

    find_crosswords_with_patterns(
        &horizontal_word,
        cross_x as usize,
        &vertical_word,
        cross_y as usize,
        dictionary,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::grid::Grid;

    #[test]
    fn test_word_matches() {
        assert_eq!(word_matches("cart", "part", 0), Some('p'));
        assert_eq!(word_matches("car", "cab", 2), Some('b'));
        assert_eq!(word_matches("bat", "but", 2), Some('t'));
        assert_eq!(word_matches("but", "cut", 2), None);
        assert_eq!(word_matches("car", "cab", 1), None);
        assert_eq!(word_matches("car", "carb", 0), None);
        assert_eq!(word_matches("carb", "car", 0), None);
    }

    // Dictionary with the words: dormi, dorni, ebrii, farbi, farti,
    // furzi, kadre, kalve, kelke, kemie, klare, klere, kosme, koste,
    // krute, kupre, kvire, larĝi, larmi, marki, parki, perdi, sarki,
    // serĉi, servi
    static DICTIONARY_DATA: [u8; 197] = [
        0x00, 0x01, 0x2a, 0x01, 0x40, 0x64, 0x01, 0x61, 0x65, 0x01, 0x1f, 0x66,
        0x07, 0x01, 0x6b, 0x01, 0x2b, 0x61, 0x04, 0x2e, 0x65, 0x04, 0x1f, 0x6c,
        0x07, 0x1f, 0x6c, 0x01, 0x49, 0x6d, 0x07, 0x10, 0x70, 0x01, 0x31, 0x6f,
        0x0d, 0x6a, 0x72, 0x00, 0x04, 0x73, 0x43, 0x1c, 0x61, 0x10, 0x5b, 0x61,
        0x3a, 0x58, 0x61, 0x3d, 0x4c, 0x75, 0x00, 0x16, 0x61, 0x3a, 0x6a, 0x61,
        0x3d, 0x67, 0x64, 0x00, 0x10, 0x65, 0x3a, 0x58, 0x6c, 0x00, 0x04, 0x6f,
        0x00, 0x0d, 0x72, 0x00, 0x10, 0x72, 0x00, 0x10, 0x72, 0x00, 0x10, 0x72,
        0x00, 0x04, 0x73, 0x52, 0x66, 0x62, 0x4c, 0x60, 0x6d, 0x43, 0x60, 0x6d,
        0x56, 0x5d, 0x6d, 0x4f, 0x5a, 0x76, 0x00, 0x22, 0x61, 0x00, 0x1c, 0x62,
        0x00, 0x16, 0x65, 0x00, 0x1c, 0x75, 0x00, 0x04, 0x76, 0x00, 0x2e, 0x65,
        0x00, 0x2b, 0x69, 0x00, 0x31, 0x6c, 0x00, 0x16, 0x6d, 0x00, 0x22, 0x70,
        0x00, 0x0d, 0x72, 0x00, 0x10, 0x72, 0x00, 0x13, 0x72, 0x00, 0x22, 0x72,
        0x00, 0x16, 0x75, 0x00, 0x2a, 0x64, 0x00, 0x24, 0x69, 0x00, 0x24, 0x69,
        0x00, 0x1e, 0x6b, 0x00, 0x1e, 0x6b, 0x00, 0x1b, 0x6e, 0x00, 0x15, 0x72,
        0x00, 0x12, 0x74, 0x00, 0x12, 0x74, 0x00, 0x0c, 0x76, 0x00, 0x0c, 0x7a,
        0x00, 0x09, 0xc4, 0x89, 0x00, 0x05, 0xc4, 0x9d, 0x00, 0x04, 0x65, 0x00,
        0x01, 0x69, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn simple_cross() {
        let dictionary = Dictionary::new(Box::new(DICTIONARY_DATA.clone()));

        let grid = "KADREEOTLERNITMKEDUKO\
                    adnrlywckmbpuejxfovth"
            .parse::<Grid>().unwrap();

        let crosswords = find_crosswords(&grid.solution, 0, 2, &dictionary);

        assert_eq!(crosswords[0].cross_letter, 'd');
        assert_eq!(crosswords[0].a_words, &["dormi", "dorni"]);
        assert_eq!(crosswords[0].b_words, &["kadre"]);

        assert_eq!(crosswords[1].cross_letter, 'e');
        assert_eq!(crosswords[1].a_words, &["ebrii"]);
        assert_eq!(crosswords[1].b_words, &["klere"]);

        assert_eq!(crosswords[2].cross_letter, 'l');
        assert_eq!(crosswords[2].a_words, &["larmi", "larĝi"]);
        assert_eq!(crosswords[2].b_words, &["kalve", "kelke"]);

        assert_eq!(crosswords[3].cross_letter, 'm');
        assert_eq!(crosswords[3].a_words, &["marki"]);
        assert_eq!(crosswords[3].b_words, &["kemie"]);

        assert_eq!(crosswords[4].cross_letter, 'p');
        assert_eq!(crosswords[4].a_words, &["parki", "perdi"]);
        assert_eq!(crosswords[4].b_words, &["kupre"]);

        assert_eq!(crosswords[5].cross_letter, 's');
        assert_eq!(crosswords[5].a_words, &["sarki", "servi", "serĉi"]);
        assert_eq!(crosswords[5].b_words, &["kosme", "koste"]);

        assert_eq!(crosswords.len(), 6);
    }
}
