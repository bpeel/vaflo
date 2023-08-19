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

use super::dictionary::Dictionary;
use super::permute;
use super::word_grid::Word;
use super::letter_grid::LetterState;

pub struct Iter<'a> {
    dictionary: &'a Dictionary,
    permuter: permute::Iter,
    spare_letters: Vec<char>,
    template: Word,
    result_buf: String,
}

impl<'a> Iter<'a> {
    pub fn new(
        dictionary: &'a Dictionary,
        template: Word,
        mut spare_letters: Vec<char>,
    ) -> Iter<'a> {
        let n_movable = template.letters
            .iter()
            .filter(|l| l.state == LetterState::Movable)
            .count();

        // Sort the letters to make it easier to detect duplicate permutations
        spare_letters.sort_unstable();

        Iter {
            dictionary,
            permuter: permute::Iter::new(spare_letters.len(), n_movable),
            spare_letters,
            template,
            result_buf: String::new(),
        }
    }

    pub fn next(&mut self) -> Option<&str> {
        'permutation: while let Some(chosen_letters) = self.permuter.next() {
            if !is_unique_permutation(&self.spare_letters, chosen_letters) {
                continue;
            }

            self.result_buf.clear();

            let mut chosen_letters = chosen_letters.iter();

            for letter in self.template.letters.iter() {
                match letter.state {
                    LetterState::Movable => {
                        let index = *chosen_letters.next().unwrap();
                        let ch = self.spare_letters[index];
                        // Don’t accept permutations that don’t change
                        // the value of a movable letter
                        if letter.value == ch {
                            continue 'permutation;
                        }
                        self.result_buf.push(self.spare_letters[index]);
                    },
                    LetterState::Fixed => self.result_buf.push(letter.value),
                }
            }

            if self.dictionary.contains(self.result_buf.chars()) {
                return Some(&self.result_buf);
            }
        }

        None
    }
}

fn is_unique_permutation(
    spare_letters: &[char],
    permutation: &[usize],
) -> bool {
    // When there are multiple copies of the same letter, we don’t
    // want to reuse permutations that result in the same selection of
    // letters. To detect this we only allow permutations that use
    // duplicate letters in order starting from the first one.

    let mut used_letters = 0u32;

    for &index in permutation.iter() {
        let letter = spare_letters[index];

        // Check that all of the previous copies of the same letter
        // are also used
        for index in (0..index).rev() {
            if spare_letters[index] != letter {
                break;
            }

            if used_letters & (1u32 << index) == 0 {
                return false;
            }
        }

        used_letters |= 1u32 << index;
    }

    true
}
