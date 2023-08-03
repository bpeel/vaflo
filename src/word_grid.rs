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

use std::fmt;

use super::letter_grid::{LetterGrid, LetterState};
use super::grid::{WORD_LENGTH, N_WORDS_ON_AXIS};

#[derive(Debug, Clone)]
pub struct Word {
    pub letters: [Option<char>; WORD_LENGTH],
}

const DEFAULT_WORD: Word = Word { letters: [None; WORD_LENGTH] };

#[derive(Debug, Clone)]
pub struct WordGrid {
    words: [Word; N_WORDS_ON_AXIS * 2],
    spare_letters: Vec<char>,
}

fn format_letter(f: &mut fmt::Formatter, letter: Option<char>) -> fmt::Result {
    match letter {
        Some(ch) => write!(f, "{}", ch.to_uppercase()),
        None => write!(f, "."),
    }
}

impl fmt::Display for WordGrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, word) in self.horizontal_words().iter().enumerate() {
            for &letter in word.letters.iter() {
                format_letter(f, letter)?;
            }

            write!(f, "\n")?;

            let vertical_letter = i * 2 + 1;

            if vertical_letter < WORD_LENGTH {
                for (i, word) in self.vertical_words().iter().enumerate() {
                    format_letter(f, word.letters[vertical_letter])?;
                    if i + 1 < N_WORDS_ON_AXIS {
                        write!(f, " ")?;
                    }
                }

                write!(f, "\n")?;
            }
        }

        write!(f, "\nSpare letters: ")?;

        for letter in self.spare_letters.iter() {
            write!(f, "{}", letter)?;
        }

        Ok(())
    }
}

impl WordGrid {
    pub fn new(original_grid: &LetterGrid) -> WordGrid {
        let mut grid = WordGrid {
            words: [DEFAULT_WORD; N_WORDS_ON_AXIS * 2],
            spare_letters: Vec::new(),
        };

        for word in 0..N_WORDS_ON_AXIS {
            for letter_num in 0..WORD_LENGTH {
                let letter = original_grid.horizontal_letter(word, letter_num);

                match letter.state {
                    LetterState::Fixed => {
                        grid.horizontal_words_mut()[word].letters[letter_num] =
                            Some(letter.value);
                    },
                    LetterState::Movable => {
                        grid.spare_letters.push(letter.value);
                    },
                }

                let letter = original_grid.vertical_letter(word, letter_num);

                match letter.state {
                    LetterState::Fixed => {
                        grid.vertical_words_mut()[word].letters[letter_num] =
                            Some(letter.value);
                    },
                    LetterState::Movable => {
                        if letter_num & 1 != 0 {
                            grid.spare_letters.push(letter.value);
                        }
                    },
                }
            }
        }

        grid
    }

    pub fn fix_word(&self, word_num: usize, word: &str) -> WordGrid {
        let mut grid = self.clone();

        let (a_slice, b_slice) = grid.words.split_at_mut(N_WORDS_ON_AXIS);

        let (fix_slice, other_slice, word_num) = if word_num < N_WORDS_ON_AXIS {
            (a_slice, b_slice, word_num)
        } else {
            (b_slice, a_slice, word_num - N_WORDS_ON_AXIS)
        };

        for (i, ch) in word.chars().enumerate() {
            match fix_slice[word_num].letters[i] {
                Some(old_ch) => assert_eq!(ch, old_ch),
                None => {
                    let spare_pos = grid.spare_letters
                        .iter()
                        .position(|&spare| spare == ch)
                        .unwrap();
                    grid.spare_letters.swap_remove(spare_pos);

                    fix_slice[word_num].letters[i] = Some(ch);

                    if i & 1 == 0 {
                        other_slice[i / 2].letters[word_num * 2] = Some(ch);
                    }
                },
            }
        }

        grid
    }

    pub fn words(&self) -> &[Word] {
        &self.words
    }

    pub fn horizontal_words(&self) -> &[Word] {
        &self.words[0..N_WORDS_ON_AXIS]
    }

    pub fn vertical_words(&self) -> &[Word] {
        &self.words[N_WORDS_ON_AXIS..N_WORDS_ON_AXIS * 2]
    }

    pub fn spare_letters(&self) -> &[char] {
        &self.spare_letters
    }

    fn horizontal_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[0..N_WORDS_ON_AXIS]
    }

    fn vertical_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[N_WORDS_ON_AXIS..N_WORDS_ON_AXIS * 2]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let grid_source = "AbCdE\n\
                           F g h\n\
                           iJklM\n\
                           n O P\n\
                           QRSTu";

        let grid = WordGrid::new(&grid_source.parse().unwrap());

        assert_eq!(
            &grid.to_string(),
            "A.C.E\n\
             F . .\n\
             .J..M\n\
             . O P\n\
             QRST.\n\
             \n\
             Spare letters: bdnigklhu"
        );

        assert_eq!(
            &grid.horizontal_words()[0].letters,
            &[
                Some('a'),
                None,
                Some('c'),
                None,
                Some('e'),
            ],
        );
        assert_eq!(
            &grid.horizontal_words()[1].letters,
            &[
                None,
                Some('j'),
                None,
                None,
                Some('m'),
            ],
        );
        assert_eq!(
            &grid.horizontal_words()[2].letters,
            &[
                Some('q'),
                Some('r'),
                Some('s'),
                Some('t'),
                None,
            ],
        );
        assert_eq!(
            &grid.vertical_words()[0].letters,
            &[
                Some('a'),
                Some('f'),
                None,
                None,
                Some('q'),
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                Some('c'),
                None,
                None,
                Some('o'),
                Some('s'),
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                Some('e'),
                None,
                Some('m'),
                Some('p'),
                None,
            ],
        );
    }

    #[test]
    fn fix_horizontal_word() {
        let grid = WordGrid::new(
            &"abcde\n\
              f g h\n\
              ijklm\n\
              n o p\n\
              qrstu".parse().unwrap()
        );

        let grid = grid.fix_word(1, "tiger");

        assert_eq!(
            &grid.to_string(),
            ".....\n\
             . . .\n\
             TIGER\n\
             . . .\n\
             .....\n\
             \n\
             Spare letters: abfcdnspjuklomqh"
        );

        assert_eq!(
            &grid.vertical_words()[0].letters,
            &[
                None,
                None,
                Some('t'),
                None,
                None,
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                None,
                None,
                Some('g'),
                None,
                None,
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                None,
                None,
                Some('r'),
                None,
                None,
            ],
        );
    }

    #[test]
    fn fix_vertical_word() {
        let grid = WordGrid::new(
            &"abcde\n\
              f g h\n\
              ijklm\n\
              n o p\n\
              qrstu".parse().unwrap()
        );

        let grid = grid.fix_word(4, "tiger");

        assert_eq!(
            &grid.to_string(),
            "..T..\n\
             . I .\n\
             ..G..\n\
             . E .\n\
             ..R..\n\
             \n\
             Spare letters: abfcdnspjuklomqh"
        );

        assert_eq!(
            &grid.vertical_words()[0].letters,
            &[
                None,
                None,
                None,
                None,
                None,
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                Some('t'),
                Some('i'),
                Some('g'),
                Some('e'),
                Some('r'),
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                None,
                None,
                None,
                None,
                None,
            ],
        );
    }
}
