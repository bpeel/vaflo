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

mod permute;
mod dictionary;
mod word_solver;
mod grid;
mod letter_grid;
mod word_grid;
mod grid_solver;
mod pairs;
mod swap_solver;

use std::process::ExitCode;
use std::io;
use std::ffi::OsStr;
use dictionary::Dictionary;
use letter_grid::LetterGrid;
use grid::{N_WORDS_ON_AXIS, WORD_LENGTH};

fn load_dictionary(filename: &OsStr) -> Result<Dictionary, io::Error> {
    std::fs::read(filename).map(|data| Dictionary::new(data.into_boxed_slice()))
}

fn grid_to_array(grid: &LetterGrid) -> Vec<char> {
    let mut letters = Vec::new();

    for word_num in 0..N_WORDS_ON_AXIS {
        for letter_num in 0..WORD_LENGTH {
            letters.push(grid.horizontal_letter(word_num, letter_num).value);
        }

        let letter_num = word_num * 2 + 1;

        if letter_num < WORD_LENGTH {
            for word_num in 0..N_WORDS_ON_AXIS {
                letters.push(grid.vertical_letter(word_num, letter_num).value);
            }
        }
    }

    letters
}

fn word_grid_to_array(grid: &word_grid::WordGrid) -> Vec<char> {
    let mut letters = Vec::new();

    for word_num in 0..N_WORDS_ON_AXIS {
        let word = &grid.horizontal_words()[word_num];

        for letter_num in 0..WORD_LENGTH {
            letters.push(word.letters[letter_num].unwrap());
        }

        let letter_num = word_num * 2 + 1;

        if letter_num < WORD_LENGTH {
            for word_num in 0..N_WORDS_ON_AXIS {
                let word = &grid.vertical_words()[word_num];
                letters.push(word.letters[letter_num].unwrap());
            }
        }
    }

    letters
}

fn run_grid(dictionary: &Dictionary, grid_buf: &str) -> bool {
    let grid = match grid_buf.parse::<LetterGrid>() {
        Err(e) => {
            eprintln!("{}", e);
            return false;
        },
        Ok(g) => g,
    };

    let start_order = grid_to_array(&grid);

    let word_grid = word_grid::WordGrid::new(&grid);
    let mut solver = grid_solver::GridSolver::new(word_grid, dictionary);

    let mut first = true;

    while let Some(grid) = solver.next() {
        if first {
            first = false;
        } else {
            println!();
        }

        println!("{}", grid);

        let target_order = word_grid_to_array(&grid);

        match swap_solver::solve(&start_order, &target_order) {
            Some(swaps) => {
                print!("{} swaps: ", swaps.len());

                for (i, swap) in swaps.into_iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{},{}", swap.0, swap.1);
                }
                println!();
            },
            None => println!("No solution found"),
        }
    }

    true
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();

    if args.len() != 2 {
        eprintln!("usage: solve-waffle <dictionary>");
        return ExitCode::FAILURE;
    }

    let dictionary_filename = args.nth(1).unwrap();

    let dictionary = match load_dictionary(&dictionary_filename) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", dictionary_filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        }
    };

    let mut grid_buf = String::new();

    for line in std::io::stdin().lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}", e);
                return ExitCode::FAILURE;
            },
        };

        if line.is_empty() {
            if !run_grid(&dictionary, &grid_buf) {
                return ExitCode::FAILURE;
            }
            grid_buf.clear();
        } else {
            if !grid_buf.is_empty() {
                grid_buf.push('\n');
            }

            grid_buf.push_str(&line);
        }
    }

    if !grid_buf.is_empty() && !run_grid(&dictionary, &grid_buf) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
