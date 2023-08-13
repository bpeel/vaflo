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
use super::{word_grid, word_solver};
use word_grid::WordGrid;

pub struct GridSolver<'a> {
    dictionary: &'a Dictionary,
    stack: Vec<StackEntry<'a>>,
}

struct StackEntry<'a> {
    grid: WordGrid,
    word_num: usize,
    word_solver: word_solver::Iter<'a>,
    is_solved: bool,
}

impl<'a> GridSolver<'a> {
    pub fn new(grid: WordGrid, dictionary: &'a Dictionary) -> GridSolver<'a> {
        let mut solver = GridSolver {
            dictionary,
            stack: Vec::new(),
        };

        solver.push_grid(grid);

        solver
    }

    fn push_grid(&mut self, grid: WordGrid) {
        // Find the unsolved word with the least number of movable letters
        let (word_num, word, is_solved) = match grid
            .words().iter()
            .map(|w| {
                (
                    w,
                    w.letters.iter().filter(|l| l.is_none()).count(),
                )
            })
            .enumerate()
            .filter(|(_, (_, n_movable_letters))| *n_movable_letters > 0)
            .min_by(|&(_, (_, a)), &(_, (_, b))| a.cmp(&b))
        {
            Some((word_num, (word, _))) => (word_num, word, false),
            None => (0, &grid.horizontal_words()[0], true),
        };

        let word_solver = word_solver::Iter::new(
            self.dictionary,
            word.clone(),
            grid.spare_letters().to_owned(),
        );

        self.stack.push(StackEntry {
            grid,
            word_num,
            word_solver,
            is_solved,
        });
    }

    pub fn next(&mut self) -> Option<WordGrid> {
        while let Some(mut entry) = self.stack.pop() {
            if entry.is_solved {
                // Double check that all of the words are really
                // valid. This can fail if fixing a word also made an
                // intersecting word be fixed.
                if entry.grid.words()
                    .iter()
                    .any(|w| {
                        !self.dictionary.contains(
                            w.letters.iter().map(|l| l.unwrap())
                        )
                    })
                {
                    continue;
                } else {
                    return Some(entry.grid);
                }
            }

            if let Some(word) = entry.word_solver.next() {
                let grid = entry.grid.fix_word(entry.word_num, word);
                self.stack.push(entry);
                self.push_grid(grid);
            }
        }

        None
    }
}
