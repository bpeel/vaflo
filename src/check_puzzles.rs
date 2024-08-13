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
mod stem_word;

use std::process::ExitCode;
use letter_grid::LetterGrid;
use dictionary::Dictionary;
use std::sync::{Arc, mpsc, Mutex};
use std::{fmt, thread};
use word_grid::WordGrid;
use grid_solver::GridSolver;
use std::io::BufRead;
use grid::Grid;
use std::collections::{HashMap, VecDeque, hash_map};
use clap::Parser;
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "check-puzzles")]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    puzzles: Option<OsString>,
    #[arg(short, long, value_name = "FILE")]
    dictionary: Option<OsString>,
}

enum PuzzleMessageKind {
    GridParseError(grid::GridParseError),
    LetterGridParseError(letter_grid::ParseError),
    SolutionCount(usize),
    NoSwapSolutionFound,
    MinimumSwaps(usize),
    BadWord(String),
    DuplicateWord(String),
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
            PuzzleMessageKind::DuplicateWord(word) => {
                write!(f, "“{}” appears more than once", word.to_uppercase())
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


fn load_dictionary(filename: Option<OsString>) -> Result<Arc<Dictionary>, ()> {
    let filename = filename.unwrap_or("data/dictionary.bin".into());

    match std::fs::read(&filename) {
        Err(e) => {
            eprintln!("{}: {}", filename.to_string_lossy(), e);
            Err(())
        },
        Ok(d) => Ok(Arc::new(Dictionary::new(d.into_boxed_slice()))),
    }
}

fn load_puzzles(filename: Option<OsString>) -> Result<VecDeque<String>, ()> {
    let filename = filename.unwrap_or("puzzles.txt".into());

    let mut puzzles = VecDeque::new();

    let f = match std::fs::File::open(&filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", filename.to_string_lossy(), e);
            return Err(());
        },
    };

    for line in std::io::BufReader::new(f).lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}: {}", filename.to_string_lossy(), e);
                return Err(());
            },
        };

        puzzles.push_back(line);
    }

    if puzzles.is_empty() {
        eprintln!("{}: empty file", filename.to_string_lossy());
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

    while solver.next().is_some() {
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
    let mut words = HashMap::new();

    for positions in grid::WordPositions::new() {
        let word_chars = positions.map(|pos| grid.solution.letters[pos]);
        let word = || { word_chars.clone().collect::<String>() };

        let mut stem = word();
        let stem_len = stem_word::stem(&stem).len();
        stem.truncate(stem_len);

        match words.entry(stem) {
            hash_map::Entry::Occupied(entry) => {
                let counter = entry.into_mut();

                if *counter == 1 {
                    tx.send(PuzzleMessage {
                        puzzle_num,
                        kind: PuzzleMessageKind::DuplicateWord(word()),
                    })?;
                }

                *counter += 1;
            },
            hash_map::Entry::Vacant(entry) => {
                if !dictionary.contains(word_chars.clone()) {
                    tx.send(PuzzleMessage {
                        puzzle_num,
                        kind: PuzzleMessageKind::BadWord(word()),
                    })?;
                }

                entry.insert(1);
            },
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
    let cli = Cli::parse();

    let Ok(dictionary) = load_dictionary(cli.dictionary)
    else {
        return ExitCode::FAILURE;
    };

    let Ok(puzzles) = load_puzzles(cli.puzzles)
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
