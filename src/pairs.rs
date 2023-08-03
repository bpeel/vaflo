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

pub struct Iter {
    size: usize,
    a: usize,
    b: usize,
}

impl Iter {
    pub fn new(size: usize) -> Iter {
        Iter {
            size,
            a: 0,
            b: 0,
        }
    }
}

impl Iterator for Iter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<(usize, usize)> {
        self.b += 1;

        let b = self.a + self.b;

        if b < self.size {
            Some((self.a, b))
        } else {
            self.b = 1;
            self.a += 1;

            if self.a + 1 < self.size {
                Some((self.a, self.a + 1))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn iter() {
        let mut iter = Iter::new(4);

        assert_eq!(iter.next(), Some((0, 1)));
        assert_eq!(iter.next(), Some((0, 2)));
        assert_eq!(iter.next(), Some((0, 3)));
        assert_eq!(iter.next(), Some((1, 2)));
        assert_eq!(iter.next(), Some((1, 3)));
        assert_eq!(iter.next(), Some((2, 3)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn empty() {
        assert_eq!(Iter::new(0).next(), None);
    }

    #[test]
    fn single() {
        assert_eq!(Iter::new(1).next(), None);
    }

    #[test]
    fn single_pair() {
        let mut iter = Iter::new(2);

        assert_eq!(iter.next(), Some((0, 1)));
        assert_eq!(iter.next(), None);
    }
}
