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

mod grid;

use std::process::ExitCode;
use grid::Grid;
use std::collections::HashMap;
use std::io::BufRead;

fn load_puzzles() -> Result<Vec<Grid>, ()> {
    let filename = "puzzles.txt";
    let mut puzzles = Vec::new();

    let f = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}: {}", filename, e);
            return Err(());
        },
    };

    for (line_num, line) in std::io::BufReader::new(f).lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}: {}", filename, e);
                return Err(());
            },
        };

        let grid = match line.parse::<Grid>() {
            Ok(grid) => grid,
            Err(e) => {
                eprintln!("{}:{} {}", filename, line_num + 1, e);
                return Err(());
            },
        };

        puzzles.push(grid);
    }

    if puzzles.is_empty() {
        eprintln!("{}: empty file", filename);
        return Err(());
    }

    Ok(puzzles)
}

fn count_words(puzzles: &[Grid]) -> HashMap<String, Vec<usize>> {
    let mut words = HashMap::<String, Vec<usize>>::new();

    for (puzzle_num, grid) in puzzles.iter().enumerate() {
        for word in grid::WordPositions::new().map(|positions| {
            positions.map(|position| grid.solution.letters[position])
                .collect::<String>()
        })
        {
            words.entry(word)
                .and_modify(|puzzles| puzzles.push(puzzle_num))
                .or_insert_with(|| vec![puzzle_num]);
        }
    }

    words
}

fn sort_words<I>(words: I) -> Vec<(String, Vec<usize>)>
where
    I: IntoIterator<Item = (String, Vec<usize>)>
{
    let mut words = words.into_iter().collect::<Vec<_>>();

    words.sort_unstable_by_key(|(_, puzzles)| usize::MAX - puzzles.len());

    words
}

fn main() -> ExitCode {
    let Ok(puzzles) = load_puzzles()
    else {
        return ExitCode::FAILURE;
    };

    for (word, puzzles) in sort_words(count_words(&puzzles)) {
        print!("{}", word);

        for puzzle in puzzles {
            print!(" {}", puzzle + 1);
        }

        println!();
    }

    ExitCode::SUCCESS
}
