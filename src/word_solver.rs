// Waffle Solve
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

pub struct Iter<'a> {
    dictionary: &'a Dictionary,
    permuter: permute::Iter,
    spare_letters: Vec<char>,
    template: &'a str,
    result_buf: String,
}

impl<'a> Iter<'a> {
    pub fn new(
        dictionary: &'a Dictionary,
        template: &'a str,
        spare_letters: &str,
    ) -> Iter<'a> {
        let n_gaps = template.chars().filter(|&c| c == '.').count();

        let spare_letters: Vec<char> = spare_letters.chars().collect();

        Iter {
            dictionary,
            permuter: permute::Iter::new(spare_letters.len(), n_gaps),
            spare_letters,
            template,
            result_buf: String::new(),
        }
    }

    pub fn next(&mut self) -> Option<&str> {
        while let Some(chosen_letters) = self.permuter.next() {
            self.result_buf.clear();

            let mut chosen_letters = chosen_letters.iter();

            for ch in self.template.chars() {
                if ch == '.' {
                    let index = *chosen_letters.next().unwrap();
                    self.result_buf.push(self.spare_letters[index]);
                } else {
                    self.result_buf.push(ch);
                }
            }

            if self.dictionary.contains(&self.result_buf) {
                return Some(&self.result_buf);
            }
        }

        None
    }
}
