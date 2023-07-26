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

mod pairs;
mod swap_solver;

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();

    if args.len() != 3 {
        eprintln!("usage: solve-waffle <start> <target>");
        return ExitCode::FAILURE;
    }

    let start = args.nth(1).unwrap().chars().collect::<Vec<char>>();
    let target = args.next().unwrap().chars().collect::<Vec<char>>();

    if start.len() != target.len() {
        eprintln!("start and target have different lengths");
        ExitCode::FAILURE
    } else {
        match swap_solver::solve(&start, &target) {
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

        ExitCode::SUCCESS
    }
}
