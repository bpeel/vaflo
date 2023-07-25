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

use std::fmt;
use std::str::FromStr;

pub const WORD_LENGTH: usize = 5;
pub const N_WORDS_ON_AXIS: usize = (WORD_LENGTH + 1) / 2;
// The number of letters not at an intersection per word
const N_SPACING_LETTERS: usize = WORD_LENGTH - N_WORDS_ON_AXIS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetterState {
    Fixed,
    Movable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Letter {
    pub value: char,
    pub state: LetterState,
}

#[derive(Debug, Clone)]
pub struct Word {
    pub letters: [Letter; WORD_LENGTH],
}

#[derive(Debug, Clone)]
pub struct Grid {
    words: [Word; N_WORDS_ON_AXIS * 2],
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedCharacter(usize, char),
    BadLowercase(usize, char),
    LineTooLong(usize),
    LineTooShort(usize),
    NotEnoughLines,
    TooManyLines,
}

const DEFAULT_LETTER: Letter = Letter {
    value: 'a',
    state: LetterState::Movable,
};

const DEFAULT_WORD: Word = Word {
    letters: [DEFAULT_LETTER; WORD_LENGTH],
};

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, word) in self.horizontal_words().iter().enumerate() {
            for letter in word.letters.iter() {
                letter.fmt(f)?;
            }

            let vertical_letter = i * 2 + 1;

            if vertical_letter < WORD_LENGTH {
                write!(f, "\n")?;

                for (i, word) in self.vertical_words().iter().enumerate() {
                    word.letters[vertical_letter].fmt(f)?;
                    if i + 1 < N_WORDS_ON_AXIS {
                        write!(f, " ")?;
                    }
                }

                write!(f, "\n")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Letter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.state {
            LetterState::Fixed => write!(f, "{}", self.value.to_uppercase()),
            LetterState::Movable => write!(f, "{}", self.value.to_lowercase()),
        }
    }
}

impl Letter {
    fn from_char(line: usize, ch: char) -> Result<Letter, ParseError> {
        if ch.is_uppercase() {
            let mut lowercase = ch.to_lowercase();

            let Some(lowercase_letter) = lowercase.next()
            else {
                return Err(ParseError::BadLowercase(line, ch));
            };

            if lowercase.next().is_some() {
                return Err(ParseError::BadLowercase(line, ch));
            }

            Ok(Letter {
                value: lowercase_letter,
                state: LetterState::Fixed,
            })
        } else if ch.is_lowercase() {
            Ok(Letter {
                value: ch,
                state: LetterState::Movable,
            })
        } else {
            Err(ParseError::UnexpectedCharacter(line, ch))
        }
    }
}

impl FromStr for Grid {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Grid, ParseError> {
        let mut grid = Grid {
            words: [DEFAULT_WORD; N_WORDS_ON_AXIS * 2],
        };

        let mut line_num = 0;

        for line in s.lines() {
            if line_num >= WORD_LENGTH {
                return Err(ParseError::TooManyLines);
            }

            if line_num & 1 == 0 {
                grid.set_horizontal_word(line_num, line)?;
            } else {
                grid.set_vertical_word_letters(line_num, line)?;
            }

            line_num += 1;
        }

        if line_num >= WORD_LENGTH {
            Ok(grid)
        } else {
            Err(ParseError::NotEnoughLines)
        }
    }
}

impl Grid {
    pub fn fix_horizontal_word(&self, word_num: usize, word: &str) -> Grid {
        let mut grid = self.clone();

        for (i, ch) in word.chars().enumerate() {
            let letter = Letter {
                value: ch,
                state: LetterState::Fixed,
            };

            grid.horizontal_words_mut()[word_num].letters[i] = letter;

            if i & 1 == 0 {
                grid.vertical_words_mut()[i / 2].letters[word_num * 2] = letter;
            }
        }

        grid
    }

    pub fn fix_vertical_word(&self, word_num: usize, word: &str) -> Grid {
        let mut grid = self.clone();

        for (i, ch) in word.chars().enumerate() {
            let letter = Letter {
                value: ch,
                state: LetterState::Fixed,
            };

            grid.vertical_words_mut()[word_num].letters[i] = letter;

            if i & 1 == 0 {
                let word = &mut grid.horizontal_words_mut()[i / 2];
                word.letters[word_num * 2] = letter;
            }
        }

        grid
    }

    fn set_horizontal_word(
        &mut self,
        line_num: usize,
        word: &str,
    ) -> Result<(), ParseError> {
        let mut letter_num = 0;

        for ch in word.chars() {
            if letter_num >= WORD_LENGTH {
                return Err(ParseError::LineTooLong(line_num));
            }

            let letter = Letter::from_char(line_num, ch)?;

            let horizontal_word =
                &mut self.horizontal_words_mut()[line_num / 2];
            horizontal_word.letters[letter_num] = letter;

            if letter_num & 1 == 0 {
                let mut vertical_word =
                    &mut self.vertical_words_mut()[letter_num / 2];
                vertical_word.letters[line_num] = letter;
            }

            letter_num += 1;
        }

        if letter_num < WORD_LENGTH {
            Err(ParseError::LineTooShort(line_num))
        } else {
            Ok(())
        }
    }

    fn set_vertical_word_letters(
        &mut self,
        line_num: usize,
        line: &str,
    ) -> Result<(), ParseError> {
        let mut char_num = 0;

        for ch in line.chars() {
            if char_num & 1 == 0 {
                let vertical_word =
                    &mut self.vertical_words_mut()[char_num / 2];
                let letter = Letter::from_char(line_num, ch)?;
                vertical_word.letters[line_num] = letter;
            } else if ch != ' ' {
                return Err(ParseError::UnexpectedCharacter(line_num, ch));
            }

            char_num += 1;
        }

        if char_num < WORD_LENGTH {
            Err(ParseError::LineTooShort(line_num))
        } else {
            Ok(())
        }
    }

    pub fn horizontal_words(&self) -> &[Word] {
        &self.words[0..N_WORDS_ON_AXIS]
    }

    pub fn vertical_words(&self) -> &[Word] {
        &self.words[N_WORDS_ON_AXIS..N_WORDS_ON_AXIS * 2]
    }

    fn horizontal_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[0..N_WORDS_ON_AXIS]
    }

    fn vertical_words_mut(&mut self) -> &mut [Word] {
        &mut self.words[N_WORDS_ON_AXIS..N_WORDS_ON_AXIS * 2]
    }

    pub fn letters(&self) -> LetterIter {
        LetterIter {
            grid: self,
            pos: 0,
        }
    }
}

fn format_character(ch: char, f: &mut fmt::Formatter) -> fmt::Result {
    if ch.is_control() {
        write!(f, "U+{:04x}", ch as u32)
    } else {
        write!(f, "{}", ch)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedCharacter(line_num, ch) => {
                write!(f, "line {}: unexpected character: ", line_num + 1)?;
                format_character(*ch, f)
            },
            ParseError::BadLowercase(line_num, ch) => {
                write!(
                    f,
                    "line {}: letter doesn’t have simple case: ",
                    line_num + 1,
                )?;
                format_character(*ch, f)
            },
            ParseError::LineTooLong(line_num) => {
                write!(f, "line {}: line too long", line_num + 1)
            },
            ParseError::LineTooShort(line_num) => {
                write!(f, "line {}: line too short", line_num + 1)
            },
            ParseError::NotEnoughLines => write!(f, "not enough lines"),
            ParseError::TooManyLines => write!(f, "too many lines"),
        }
    }
}

pub struct LetterIter<'a> {
    grid: &'a Grid,
    pos: usize,
}

impl<'a> Iterator for LetterIter<'a> {
    type Item = Letter;

    fn next(&mut self) -> Option<Letter> {
        let pos = self.pos;

        if pos < N_WORDS_ON_AXIS * WORD_LENGTH {
            self.pos += 1;

            let word = &self.grid.horizontal_words()[pos / WORD_LENGTH];
            Some(word.letters[pos % WORD_LENGTH])
        } else {
            let pos = pos - N_WORDS_ON_AXIS * WORD_LENGTH;

            if pos < N_WORDS_ON_AXIS * N_SPACING_LETTERS {
                self.pos += 1;

                let word = &self.grid.vertical_words()[pos / N_SPACING_LETTERS];
                Some(word.letters[pos % N_SPACING_LETTERS * 2 + 1])
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
    fn parse() {
        let grid_source = "AbCdE\n\
                           F g h\n\
                           iJklM\n\
                           n O P\n\
                           QRSTu";

        let grid = grid_source.parse::<Grid>().unwrap();

        assert_eq!(&grid.to_string(), grid_source);

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
    fn bad_character() {
        assert_eq!(
            "line 2: unexpected character: -",
            &"ABCDE\nA C -".parse::<Grid>().unwrap_err().to_string(),
        );
        assert_eq!(
            "line 2: unexpected character: B",
            &"ABCDE\nABCDE".parse::<Grid>().unwrap_err().to_string(),
        );
        assert_eq!(
            "line 1: unexpected character: U+0009",
            &"ABCD\t".parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn bad_lowercase() {
        assert_eq!(
            "line 1: letter doesn’t have simple case: İ",
            &"ABCDİ".parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn line_too_long() {
        assert_eq!(
            "line 1: line too long",
            &"ABCDEF".parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn line_too_short() {
        assert_eq!(
            "line 1: line too short",
            &"ABCD".parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn too_many_lines() {
        assert_eq!(
            "too many lines",
            &"ABCDE\n\
              F G H\n\
              IJKLM\n\
              N O P\n\
              QRSTU\n\
              poop"
                .parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn not_enough_lines() {
        assert_eq!(
            "not enough lines",
            &"ABCDE\n\
              F G H\n\
              IJKLM\n\
              N O P"
                .parse::<Grid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn fix_horizontal_word() {
        let grid = "abcde\n\
                    f g h\n\
                    ijklm\n\
                    n o p\n\
                    qrstu"
            .parse::<Grid>().unwrap();

        let grid = grid.fix_horizontal_word(1, "tiger");

        assert_eq!(
            &grid.to_string(),
            "abcde\n\
             f g h\n\
             TIGER\n\
             n o p\n\
             qrstu",
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
        let grid = "abcde\n\
                    f g h\n\
                    ijklm\n\
                    n o p\n\
                    qrstu"
            .parse::<Grid>().unwrap();

        let grid = grid.fix_vertical_word(1, "tiger");

        assert_eq!(
            &grid.to_string(),
            "abTde\n\
             f I h\n\
             ijGlm\n\
             n E p\n\
             qrRtu",
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

    #[test]
    fn letters() {
        let grid = "abcde\n\
                    f g h\n\
                    ijklm\n\
                    n o p\n\
                    qrstu"
            .parse::<Grid>().unwrap();

        let letters = grid.letters().map(|l| l.value).collect::<String>();

        assert_eq!(&letters, "abcdeijklmqrstufngohp");
        assert_eq!(
            letters.chars().count(),
            N_WORDS_ON_AXIS * (WORD_LENGTH + N_SPACING_LETTERS),
        );
    }
}
