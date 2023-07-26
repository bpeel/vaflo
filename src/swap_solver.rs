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

use super::pairs;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem;

struct StackEntry {
    pair_iter: pairs::Iter,
    a: usize,
    b: usize,
}

pub fn solve<T>(
    start: &[T],
    target: &[T]
) -> Option<Vec<(usize, usize)>>
where
    T: Hash + Clone + Eq
{
    assert_eq!(start.len(), target.len());

    if start == target {
        return Some(Vec::new());
    }

    let mut best_solution = None;
    let mut visited_states = HashMap::new();
    let mut state = start.to_owned();
    let mut stack = Vec::<StackEntry>::new();
    let mut pair_iter = pairs::Iter::new(start.len());

    loop {
        match pair_iter.next() {
            Some((a, b)) => {
                // Don’t move items that are already in the right position
                if state[a] == target[a] || state[b] == target[b] {
                    continue;
                }

                // Don’t swap items if it doesn’t put one of them in
                // the right place
                if state[a] != target[b] && state[b] != target[a] {
                    continue;
                }

                state.swap(a, b);

                let n_moves = stack.len() + 1;

                // Have we already seen this state with fewer moves?
                match visited_states.get_mut(&state) {
                    Some(swaps) => {
                        if *swaps <= n_moves {
                            // Revert the swap and try the next one
                            state.swap(a, b);
                            continue;
                        } else {
                            *swaps = n_moves;
                        }
                    },
                    None => {
                        visited_states.insert(state.clone(), n_moves);
                    },
                }

                // Have we found a solution?
                if state == target {
                    let solution = best_solution.get_or_insert_with(Vec::new);
                    solution.clear();
                    solution.extend(stack.iter().map(|entry| {
                        (entry.a, entry.b)
                    }));
                    solution.push((a, b));

                    // Revert the swap and try the next one
                    state.swap(a, b);
                    continue;
                }

                let next_pair_iter = pairs::Iter::new(start.len());

                stack.push(StackEntry {
                    pair_iter: mem::replace(&mut pair_iter, next_pair_iter),
                    a,
                    b,
                });
            },
            None => {
                // Backtrack
                match stack.pop() {
                    Some(entry) => {
                        state.swap(entry.a, entry.b);
                        pair_iter = entry.pair_iter;
                    },
                    None => break,
                }
            },
        }
    }

    best_solution
}
