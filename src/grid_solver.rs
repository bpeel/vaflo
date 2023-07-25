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
use super::{grid, word_solver};
use grid::{Grid, LetterState};

pub struct GridSolver<'a> {
    dictionary: &'a Dictionary,
    stack: Vec<StackEntry<'a>>,
}

struct StackEntry<'a> {
    grid: Grid,
    word_num: usize,
    word_solver: word_solver::Iter<'a>,
    is_solved: bool,
}

impl<'a> GridSolver<'a> {
    pub fn new(grid: Grid, dictionary: &'a Dictionary) -> GridSolver<'a> {
        let mut solver = GridSolver {
            dictionary,
            stack: Vec::new(),
        };

        solver.push_grid(grid);

        solver
    }

    fn push_grid(&mut self, grid: Grid) {
        // Find the unsolved word with the least number of movable letters
        let (word_num, word, is_solved) = match grid
            .horizontal_words().iter()
            .chain(grid.vertical_words().iter())
            .map(|w| {
                (
                    w,
                    w.letters.iter().filter(|l| {
                        l.state == LetterState::Movable
                    }).count()
                )
            })
            .enumerate()
            .filter(|(_, (_, n_movable_letters))| *n_movable_letters > 0)
            .min_by(|&(_, (_, a)), &(_, (_, b))| a.cmp(&b))
        {
            Some((word_num, (word, _))) => (word_num, word, false),
            None => (0, &grid.horizontal_words()[0], true),
        };

        // Collect all the movable letters
        let movable_letters: Vec<char> = grid
            .horizontal_words().iter()
            .chain(grid.vertical_words().iter())
            .map(|w| w.letters.iter())
            .flatten()
            .filter_map(|letter| {
                match letter.state {
                    LetterState::Movable => Some(letter.value),
                    LetterState::Fixed => None,
                }
            })
            .collect();

        let word_solver = word_solver::Iter::new(
            self.dictionary,
            word.clone(),
            movable_letters,
        );

        self.stack.push(StackEntry {
            grid,
            word_num,
            word_solver,
            is_solved,
        });
    }

    pub fn next(&mut self) -> Option<Grid> {
        while let Some(mut entry) = self.stack.pop() {
            if entry.is_solved {
                return Some(entry.grid)
            }

            if let Some(word) = entry.word_solver.next() {
                let grid = if entry.word_num < grid::N_WORDS_ON_AXIS {
                    entry.grid.fix_horizontal_word(entry.word_num, word)
                } else {
                    entry.grid.fix_vertical_word(
                        entry.word_num - grid::N_WORDS_ON_AXIS,
                        word,
                    )
                };

                self.stack.push(entry);
                self.push_grid(grid);
            }
        }

        None
    }
}
