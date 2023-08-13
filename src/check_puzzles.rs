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
use std::sync::{Arc, mpsc, Mutex};
use std::{fmt, thread};
use word_grid::WordGrid;
use grid_solver::GridSolver;
use std::io::BufRead;
use grid::Grid;
use std::collections::VecDeque;

enum PuzzleMessageKind {
    GridParseError(grid::GridParseError),
    LetterGridParseError(letter_grid::ParseError),
    SolutionCount(usize),
    NoSwapSolutionFound,
    MinimumSwaps(usize),
    BadWord(String),
}

struct PuzzleMessage {
    puzzle_num: usize,
    kind: PuzzleMessageKind,
}

struct PuzzleQueue {
    data: Mutex<PuzzleQueueData>,
}

struct PuzzleQueueData {
    next_puzzle_num: usize,
    jobs: VecDeque<String>,
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
            PuzzleMessageKind::BadWord(word) => {
                write!(f, "“{}” is not in the dictionary", word.to_uppercase())
            },
        }
    }
}

impl PuzzleQueue {
    fn new(jobs: VecDeque<String>) -> PuzzleQueue {
        PuzzleQueue {
            data: Mutex::new(PuzzleQueueData {
                next_puzzle_num: 0,
                jobs,
            })
        }
    }

    fn next(&self) -> Option<(usize, String)> {
        let mut data = self.data.lock().unwrap();

        data.jobs
            .pop_front()
            .map(|job| {
                let puzzle_num = data.next_puzzle_num;
                data.next_puzzle_num += 1;
                (puzzle_num, job)
            })
    }
}

fn minimum_swaps(grid: &Grid) -> Option<usize> {
    let puzzle = grid.puzzle
        .squares
        .iter()
        .map(|square| grid.solution.letters[square.position])
        .collect::<Vec<char>>();

    swap_solver::solve(&puzzle, &grid.solution.letters)
        .map(|solution| solution.len())
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

fn load_puzzles() -> Result<VecDeque<String>, ()> {
    let filename = "puzzles.txt";
    let mut puzzles = VecDeque::new();

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

        puzzles.push_back(line);
    }

    if puzzles.is_empty() {
        eprintln!("{}: empty file", filename);
        return Err(());
    }

    Ok(puzzles)
}

fn count_solutions(grid: &LetterGrid, dictionary: &Dictionary) -> usize {
    let mut solver = GridSolver::new(
        WordGrid::new(grid),
        dictionary,
    );

    let mut count = 0;

    while let Some(_) = solver.next() {
        count += 1;
    }

    count
}

fn check_words(
    dictionary: &Dictionary,
    puzzle_num: usize,
    grid: &Grid,
    tx: &mpsc::Sender<PuzzleMessage>,
) -> Result<(), mpsc::SendError<PuzzleMessage>> {
    for positions in grid::WordPositions::new() {
        let word = positions.map(|pos| grid.solution.letters[pos]);

        if !dictionary.contains(word.clone()) {
            tx.send(PuzzleMessage {
                puzzle_num,
                kind: PuzzleMessageKind::BadWord(word.collect::<String>()),
            })?;
        }
    }

    Ok(())
}

fn check_puzzles(
    dictionary: &Dictionary,
    puzzles: &PuzzleQueue,
    tx: mpsc::Sender<PuzzleMessage>,
) -> Result<(), mpsc::SendError<PuzzleMessage>> {
    while let Some((puzzle_num, puzzle_string)) = puzzles.next() {
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

        check_words(dictionary, puzzle_num, &grid, &tx)?;

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

    let Ok(puzzles) = load_puzzles()
    else {
        return ExitCode::FAILURE;
    };

    let n_puzzles = puzzles.len();

    let puzzles = Arc::new(PuzzleQueue::new(puzzles));

    let (tx, rx) = mpsc::channel();
    let n_threads = Into::<usize>::into(
        thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN)
    ).min(n_puzzles);

    let handles = (0..n_threads).map(|_| {
        let puzzles = Arc::clone(&puzzles);
        let tx = tx.clone();
        let dictionary = Arc::clone(&dictionary);

        thread::spawn(move || check_puzzles(&dictionary, &puzzles, tx))
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
