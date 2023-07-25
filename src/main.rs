mod permute;
mod dictionary;
mod word_solver;
mod grid;
mod grid_solver;

use std::process::ExitCode;
use std::io;
use std::ffi::OsStr;
use dictionary::Dictionary;

fn load_dictionary(filename: &OsStr) -> Result<Dictionary, io::Error> {
    std::fs::read(filename).map(|data| Dictionary::new(data.into_boxed_slice()))
}

fn run_grid(dictionary: &Dictionary, grid_buf: &str) -> bool {
    let grid = match grid_buf.parse::<grid::Grid>() {
        Err(e) => {
            eprintln!("{}", e);
            return false;
        },
        Ok(g) => g,
    };

    let mut solver = grid_solver::GridSolver::new(grid, dictionary);

    let mut first = true;

    while let Some(grid) = solver.next() {
        if first {
            first = false;
        } else {
            println!();
        }

        println!("{}", grid);
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
