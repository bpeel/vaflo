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

use std::iter::repeat;

pub struct Iter<'a, T> {
    order: &'a mut [T],
    stack: Vec<usize>,
}

impl<'a, T> Iter<'a, T> {
    pub fn new(items: &'a mut [T], n_take: usize) -> Iter<T> {
        assert!(n_take <= items.len());

        Iter {
            order: items,
            stack: Vec::with_capacity(n_take),
        }
    }

    pub fn current(&self) -> &[T] {
        &self.order[0..self.stack.capacity()]
    }

    pub fn next(&mut self) -> bool {
        // Handle the first call specially
        if self.stack.is_empty() {
            if self.stack.capacity() <= 0 {
                false
            } else {
                self.stack.extend(repeat(0).take(self.stack.capacity()));
                true
            }
        } else {
            // Backtrack all of the stack entries that have reached
            // the end of the options
            while let Some(entry) = self.stack.pop() {
                // Revert the swap
                self.order.swap(self.stack.len(), self.stack.len() + entry);

                // Is there a value left to pick for this level of the stack?
                if entry + 1 < self.order.len() - self.stack.len() {
                    self.order.swap(
                        self.stack.len(),
                        self.stack.len() + entry + 1,
                    );
                    self.stack.push(entry + 1);
                    // For the rest the values start by picking the
                    // first possible one. The order is already right
                    // for this.
                    let to_add = self.stack.capacity() - self.stack.len();
                    self.stack.extend(repeat(0).take(to_add));
                    return true;
                }
            }

            // If we make it here we’ve exhausted every combination
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_different() {
        let mut values = HashSet::<[u8; 3]>::new();
        let mut items = [0u8, 1u8, 2u8, 3u8, 4u8];
        let mut iter = Iter::new(&mut items, 3);

        while iter.next() {
            let order = iter.current();
            assert_eq!(order.len(), 3);
            let order = [order[0], order[1], order[2]];
            if !values.insert(order) {
                unreachable!("duplicate permutation returned");
            }
        }

        assert_eq!(values.len(), 5 * 4 * 3);
    }

    #[test]
    fn expected_values() {
        let values = [
            [0, 1],
            [0, 2],
            [1, 0],
            [1, 2],
            [2, 1],
            [2, 0],
        ];

        let mut items = [0u8, 1u8, 2u8];
        let mut iter = Iter::new(&mut items, 2);

        for value in values {
            assert!(iter.next());
            assert_eq!(iter.current(), &value);
        }

        assert!(!iter.next());
    }

    #[test]
    fn single() {
        let mut items = [0u8];
        let mut iter = Iter::new(&mut items, 1);

        assert!(iter.next());
        assert_eq!(iter.current(), &[0]);
        assert!(!iter.next());
    }

    #[test]
    fn empty() {
        let mut items = [0u8, 62, 5, 1, 2, 42];
        let mut iter = Iter::new(&mut items, 0);

        assert!(!iter.next());
    }
}
