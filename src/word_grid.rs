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

use super::letter_grid::{LetterGrid, LetterState, DEFAULT_LETTER, Letter};
use super::grid::{WORD_LENGTH, N_WORDS_ON_AXIS, N_WORDS};

#[derive(Debug, Clone)]
pub struct Word {
    pub letters: [Letter; WORD_LENGTH],
}

const DEFAULT_WORD: Word = Word { letters: [DEFAULT_LETTER; WORD_LENGTH] };

#[derive(Debug, Clone)]
pub struct WordGrid {
    words: [Word; N_WORDS],
    spare_letters: Vec<char>,
}

fn format_letter(f: &mut fmt::Formatter, letter: Letter) -> fmt::Result {
    match letter.state {
        LetterState::Fixed => write!(f, "{}", letter.value.to_uppercase()),
        LetterState::Movable => write!(f, "."),
    }
}

impl fmt::Display for WordGrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, word) in self.horizontal_words().iter().enumerate() {
            for &letter in word.letters.iter() {
                format_letter(f, letter)?;
            }

            writeln!(f)?;

            let vertical_letter = i * 2 + 1;

            if vertical_letter < WORD_LENGTH {
                for (i, word) in self.vertical_words().iter().enumerate() {
                    format_letter(f, word.letters[vertical_letter])?;
                    if i + 1 < N_WORDS_ON_AXIS {
                        write!(f, " ")?;
                    }
                }

                writeln!(f)?;
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
            words: [DEFAULT_WORD; N_WORDS],
            spare_letters: Vec::new(),
        };

        for word in 0..N_WORDS_ON_AXIS {
            for letter_num in 0..WORD_LENGTH {
                let letter = original_grid.horizontal_letter(word, letter_num);

                grid.horizontal_words_mut()[word].letters[letter_num] = letter;

                if letter.state == LetterState::Movable {
                    grid.spare_letters.push(letter.value);
                }

                let letter = original_grid.vertical_letter(word, letter_num);

                grid.vertical_words_mut()[word].letters[letter_num] = letter;

                if letter.state == LetterState::Movable && letter_num & 1 != 0 {
                    grid.spare_letters.push(letter.value);
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
            let letter = fix_slice[word_num].letters[i];

            match letter.state {
                LetterState::Fixed => assert_eq!(ch, letter.value),
                LetterState::Movable => {
                    let spare_pos = grid.spare_letters
                        .iter()
                        .position(|&spare| spare == ch)
                        .unwrap();
                    grid.spare_letters.swap_remove(spare_pos);

                    let letter = Letter {
                        value: ch,
                        state: LetterState::Fixed,
                    };

                    fix_slice[word_num].letters[i] = letter;

                    if i & 1 == 0 {
                        other_slice[i / 2].letters[word_num * 2] = letter;
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
        &self.words[N_WORDS_ON_AXIS..N_WORDS]
    }

    pub fn spare_letters(&self) -> &[char] {
        &self.spare_letters
    }

    fn horizontal_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[0..N_WORDS_ON_AXIS]
    }

    fn vertical_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[N_WORDS_ON_AXIS..N_WORDS]
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
                Letter { value: 'a', state: LetterState::Fixed },
                Letter { value: 'b', state: LetterState::Movable },
                Letter { value: 'c', state: LetterState::Fixed },
                Letter { value: 'd', state: LetterState::Movable },
                Letter { value: 'e', state: LetterState::Fixed },
            ],
        );
        assert_eq!(
            &grid.horizontal_words()[1].letters,
            &[
                Letter { value: 'i', state: LetterState::Movable },
                Letter { value: 'j', state: LetterState::Fixed },
                Letter { value: 'k', state: LetterState::Movable },
                Letter { value: 'l', state: LetterState::Movable },
                Letter { value: 'm', state: LetterState::Fixed },
            ],
        );
        assert_eq!(
            &grid.horizontal_words()[2].letters,
            &[
                Letter { value: 'q', state: LetterState::Fixed },
                Letter { value: 'r', state: LetterState::Fixed },
                Letter { value: 's', state: LetterState::Fixed },
                Letter { value: 't', state: LetterState::Fixed },
                Letter { value: 'u', state: LetterState::Movable },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[0].letters,
            &[
                Letter { value: 'a', state: LetterState::Fixed },
                Letter { value: 'f', state: LetterState::Fixed },
                Letter { value: 'i', state: LetterState::Movable },
                Letter { value: 'n', state: LetterState::Movable },
                Letter { value: 'q', state: LetterState::Fixed },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                Letter { value: 'c', state: LetterState::Fixed },
                Letter { value: 'g', state: LetterState::Movable },
                Letter { value: 'k', state: LetterState::Movable },
                Letter { value: 'o', state: LetterState::Fixed },
                Letter { value: 's', state: LetterState::Fixed },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                Letter { value: 'e', state: LetterState::Fixed },
                Letter { value: 'h', state: LetterState::Movable },
                Letter { value: 'm', state: LetterState::Fixed },
                Letter { value: 'p', state: LetterState::Fixed },
                Letter { value: 'u', state: LetterState::Movable },
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
                Letter { value: 'a', state: LetterState::Movable },
                Letter { value: 'f', state: LetterState::Movable },
                Letter { value: 't', state: LetterState::Fixed },
                Letter { value: 'n', state: LetterState::Movable },
                Letter { value: 'q', state: LetterState::Movable },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                Letter { value: 'c', state: LetterState::Movable },
                Letter { value: 'g', state: LetterState::Movable },
                Letter { value: 'g', state: LetterState::Fixed },
                Letter { value: 'o', state: LetterState::Movable },
                Letter { value: 's', state: LetterState::Movable },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                Letter { value: 'e', state: LetterState::Movable },
                Letter { value: 'h', state: LetterState::Movable },
                Letter { value: 'r', state: LetterState::Fixed },
                Letter { value: 'p', state: LetterState::Movable },
                Letter { value: 'u', state: LetterState::Movable },
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
                Letter { value: 'a', state: LetterState::Movable },
                Letter { value: 'f', state: LetterState::Movable },
                Letter { value: 'i', state: LetterState::Movable },
                Letter { value: 'n', state: LetterState::Movable },
                Letter { value: 'q', state: LetterState::Movable },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[1].letters,
            &[
                Letter { value: 't', state: LetterState::Fixed },
                Letter { value: 'i', state: LetterState::Fixed },
                Letter { value: 'g', state: LetterState::Fixed },
                Letter { value: 'e', state: LetterState::Fixed },
                Letter { value: 'r', state: LetterState::Fixed },
            ],
        );
        assert_eq!(
            &grid.vertical_words()[2].letters,
            &[
                Letter { value: 'e', state: LetterState::Movable },
                Letter { value: 'h', state: LetterState::Movable },
                Letter { value: 'm', state: LetterState::Movable },
                Letter { value: 'p', state: LetterState::Movable },
                Letter { value: 'u', state: LetterState::Movable },
            ],
        );
    }
}
