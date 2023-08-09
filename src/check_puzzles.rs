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

mod letter_grid;
mod dictionary;
mod word_grid;
mod word_solver;
mod grid_solver;
mod permute;
mod pairs;
mod swap_solver;
mod grid;
mod stars;

use std::process::ExitCode;
use letter_grid::LetterGrid;
use dictionary::Dictionary;
use std::sync::{Arc, mpsc};
use std::{fmt, thread};
use word_grid::WordGrid;
use grid_solver::GridSolver;
use std::io::BufRead;
use grid::Grid;

enum PuzzleMessageKind {
    GridParseError(grid::GridParseError),
    LetterGridParseError(letter_grid::ParseError),
    SolutionCount(usize),
    NoSwapSolutionFound,
    MinimumSwaps(usize),
}

struct PuzzleMessage {
    puzzle_num: usize,
    kind: PuzzleMessageKind,
}

impl fmt::Display for PuzzleMessageKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PuzzleMessageKind::GridParseError(e) => write!(f, "{}", e),
            PuzzleMessageKind::LetterGridParseError(e) => write!(f, "{}", e),
            PuzzleMessageKind::SolutionCount(count) => {
                write!(f, "puzzle has {} solutions", count)
            },
            PuzzleMessageKind::NoSwapSolutionFound => {
                write!(f, "no solution found by swapping letters")
            },
            PuzzleMessageKind::MinimumSwaps(swaps) => {
                write!(f, "minimum number of swaps is {}", swaps)
            },
        }
    }
}

fn minimum_swaps(grid: &Grid) -> Option<usize> {
    let solution = grid.solution
        .letters
        .iter()
        .map(|&letter| letter)
        .collect::<Vec<char>>();
    let puzzle = grid.puzzle
        .squares
        .iter()
        .map(|square| grid.solution.letters[square.position])
        .collect::<Vec<char>>();

    swap_solver::solve(&puzzle, &solution).map(|solution| solution.len())
}


fn load_dictionary() -> Result<Arc<Dictionary>, ()> {
    let filename = "data/dictionary.bin";

    match std::fs::read(filename) {
        Err(e) => {
            eprintln!("{}: {}", filename, e);
            Err(())
        },
        Ok(d) => Ok(Arc::new(Dictionary::new(d.into_boxed_slice()))),
    }
}

fn load_puzzles() -> Result<Vec<String>, ()> {
    let filename = "puzzles.txt";
    let mut puzzles = Vec::new();

    let f = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", filename, e);
            return Err(());
        },
    };

    for line in std::io::BufReader::new(f).lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}: {}", filename, e);
                return Err(());
            },
        };

        puzzles.push(line);
    }

    if puzzles.is_empty() {
        eprintln!("{}: empty file", filename);
        return Err(());
    }

    Ok(puzzles)
}

fn count_solutions(grid: &LetterGrid, dictionary: &Dictionary) -> usize {
    let mut solver = GridSolver::new(
        WordGrid::new(&grid),
        dictionary,
    );

    let mut count = 0;

    while let Some(_) = solver.next() {
        count += 1;
    }

    count
}

fn check_puzzles<'a, I>(
    dictionary: &Dictionary,
    first_puzzle_num: usize,
    puzzles: I,
    tx: mpsc::Sender<PuzzleMessage>,
) -> Result<(), mpsc::SendError<PuzzleMessage>>
where
    I: IntoIterator<Item = String>
{
    for (puzzle_num, puzzle_string) in puzzles.into_iter().enumerate() {
        let puzzle_num = puzzle_num + first_puzzle_num;

        let grid = match puzzle_string.parse::<Grid>() {
            Ok(grid) => grid,
            Err(e) => {
                tx.send(PuzzleMessage {
                    puzzle_num,
                    kind: PuzzleMessageKind::GridParseError(e),
                })?;
                continue;
            },
        };

        match LetterGrid::from_grid(&grid) {
            Ok(letter_grid) => {
                let solution_count = count_solutions(&letter_grid, dictionary);

                if solution_count != 1 {
                    tx.send(PuzzleMessage {
                        puzzle_num,
                        kind: PuzzleMessageKind::SolutionCount(solution_count),
                    })?;
                }
            },
            Err(e) => {
                tx.send(PuzzleMessage {
                    puzzle_num,
                    kind: PuzzleMessageKind::LetterGridParseError(e),
                })?;
            },
        }

        match minimum_swaps(&grid) {
            Some(swaps) => {
                if swaps != stars::MAXIMUM_SWAPS as usize
                    - stars::MAXIMUM_STARS as usize
                {
                    tx.send(PuzzleMessage {
                        puzzle_num,
                        kind: PuzzleMessageKind::MinimumSwaps(swaps),
                    })?;
                }
            },
            None => {
                tx.send(PuzzleMessage {
                    puzzle_num,
                    kind: PuzzleMessageKind::NoSwapSolutionFound,
                })?;
            },
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    let Ok(dictionary) = load_dictionary()
    else {
        return ExitCode::FAILURE;
    };

    let Ok(mut puzzles) = load_puzzles()
    else {
        return ExitCode::FAILURE;
    };

    let (tx, rx) = mpsc::channel();
    let n_threads = Into::<usize>::into(
        thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN)
    ).min(puzzles.len());

    let puzzles_per_thread = (puzzles.len() + n_threads - 1) / n_threads;

    let handles = (0..n_threads).map(|_| {
        let n_puzzles = puzzles_per_thread.min(puzzles.len());
        let first_puzzle = puzzles.len() - n_puzzles;
        let puzzles = puzzles.drain(first_puzzle..).collect::<Vec<_>>();
        let tx = tx.clone();
        let dictionary = Arc::clone(&dictionary);

        thread::spawn(move || {
            check_puzzles(
                &dictionary,
                first_puzzle,
                puzzles,
                tx
            )
        })
    }).collect::<Vec<_>>();

    std::mem::drop(tx);

    let mut result = ExitCode::SUCCESS;

    for message in rx {
        result = ExitCode::FAILURE;

        eprintln!("puzzle {}: {}", message.puzzle_num + 1, message.kind);
    }

    for handle in handles {
        if let Err(e) = handle.join() {
            std::panic::resume_unwind(e);
        }
    }

    result
}
