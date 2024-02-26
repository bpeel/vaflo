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
use std::str::FromStr;
use super::grid;
use grid::{WORD_LENGTH, N_WORDS_ON_AXIS, N_SPACING_LETTERS, N_LETTERS};

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
pub struct LetterGrid {
    letters: [Letter; N_LETTERS],
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

pub const DEFAULT_LETTER: Letter = Letter {
    value: 'a',
    state: LetterState::Movable,
};

impl fmt::Display for LetterGrid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for word_num in 0..N_WORDS_ON_AXIS {
            for i in 0..WORD_LENGTH {
                self.horizontal_letter(word_num, i).fmt(f)?;
            }

            let vertical_letter = word_num * 2 + 1;

            if vertical_letter < WORD_LENGTH {
                writeln!(f)?;

                for i in 0..N_WORDS_ON_AXIS {
                    self.vertical_letter(i, vertical_letter).fmt(f)?;

                    if i + 1 < N_WORDS_ON_AXIS {
                        write!(f, " ")?;
                    }
                }

                writeln!(f)?;
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

impl FromStr for LetterGrid {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<LetterGrid, ParseError> {
        let mut grid = LetterGrid { letters: [DEFAULT_LETTER; N_LETTERS] };

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

impl LetterGrid {
    pub fn from_grid(grid: &grid::Grid) -> Result<LetterGrid, ParseError> {
        let mut letter_grid =
            LetterGrid { letters: [DEFAULT_LETTER; N_LETTERS] };

        for position in 0..WORD_LENGTH * WORD_LENGTH {
            if grid::is_gap_position(position) {
                continue;
            }

            let x = position % WORD_LENGTH;
            let y = position / WORD_LENGTH;

            let letter_num = if y & 1 == 0 {
                y / 2 * WORD_LENGTH + x
            } else {
                WORD_LENGTH * N_WORDS_ON_AXIS
                    + x / 2 * N_SPACING_LETTERS
                    + y / 2
            };

            let solution_pos = grid.puzzle.squares[position].position;

            letter_grid.letters[letter_num] = Letter {
                value: grid.solution.letters[solution_pos],

                state: match grid.puzzle.squares[position].state {
                    grid::PuzzleSquareState::Correct => LetterState::Fixed,
                    grid::PuzzleSquareState::WrongPosition
                        | grid::PuzzleSquareState::Wrong
                        => LetterState::Movable,
                },
            };
        }

        Ok(letter_grid)
    }

    fn set_horizontal_word(
        &mut self,
        line_num: usize,
        word: &str,
    ) -> Result<(), ParseError> {
        let mut letter_num = 0;
        let word_offset = line_num / 2 * WORD_LENGTH;

        for ch in word.chars() {
            if letter_num >= WORD_LENGTH {
                return Err(ParseError::LineTooLong(line_num));
            }

            let letter = Letter::from_char(line_num, ch)?;

            self.letters[word_offset + letter_num] = letter;

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
        let letter_offset = line_num / 2 + WORD_LENGTH * N_WORDS_ON_AXIS;

        for ch in line.chars() {
            if char_num & 1 == 0 {
                let word_num = char_num / 2;
                self.letters[letter_offset + word_num * N_SPACING_LETTERS] =
                    Letter::from_char(line_num, ch)?;
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

    pub fn horizontal_letter(&self, word: usize, letter: usize) -> Letter {
        self.letters[word * WORD_LENGTH + letter]
    }

    pub fn vertical_letter(&self, word: usize, letter: usize) -> Letter {
        if letter & 1 == 0 {
            self.horizontal_letter(letter / 2, word * 2)
        } else {
            self.letters[
                WORD_LENGTH * N_WORDS_ON_AXIS
                    + word * N_SPACING_LETTERS
                    + letter / 2
            ]
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

        let grid = grid_source.parse::<LetterGrid>().unwrap();

        assert_eq!(&grid.to_string(), grid_source);

        assert_eq!(
            grid.horizontal_letter(0, 0),
            Letter { value: 'a', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(0, 1),
            Letter { value: 'b', state: LetterState::Movable },
        );
        assert_eq!(
            grid.horizontal_letter(0, 2),
            Letter { value: 'c', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(0, 3),
            Letter { value: 'd', state: LetterState::Movable },
        );
        assert_eq!(
            grid.horizontal_letter(0, 4),
            Letter { value: 'e', state: LetterState::Fixed },
        );

        assert_eq!(
            grid.horizontal_letter(1, 0),
            Letter { value: 'i', state: LetterState::Movable },
        );
        assert_eq!(
            grid.horizontal_letter(1, 1),
            Letter { value: 'j', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(1, 2),
            Letter { value: 'k', state: LetterState::Movable },
        );
        assert_eq!(
            grid.horizontal_letter(1, 3),
            Letter { value: 'l', state: LetterState::Movable },
        );
        assert_eq!(
            grid.horizontal_letter(1, 4),
            Letter { value: 'm', state: LetterState::Fixed },
        );

        assert_eq!(
            grid.horizontal_letter(2, 0),
            Letter { value: 'q', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(2, 1),
            Letter { value: 'r', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(2, 2),
            Letter { value: 's', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(2, 3),
            Letter { value: 't', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.horizontal_letter(2, 4),
            Letter { value: 'u', state: LetterState::Movable },
        );

        assert_eq!(
            grid.vertical_letter(0, 0),
            Letter { value: 'a', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(0, 1),
            Letter { value: 'f', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(0, 2),
            Letter { value: 'i', state: LetterState::Movable },
        );
        assert_eq!(
            grid.vertical_letter(0, 3),
            Letter { value: 'n', state: LetterState::Movable },
        );
        assert_eq!(
            grid.vertical_letter(0, 4),
            Letter { value: 'q', state: LetterState::Fixed },
        );

        assert_eq!(
            grid.vertical_letter(1, 0),
            Letter { value: 'c', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(1, 1),
            Letter { value: 'g', state: LetterState::Movable },
        );
        assert_eq!(
            grid.vertical_letter(1, 2),
            Letter { value: 'k', state: LetterState::Movable },
        );
        assert_eq!(
            grid.vertical_letter(1, 3),
            Letter { value: 'o', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(1, 4),
            Letter { value: 's', state: LetterState::Fixed },
        );

        assert_eq!(
            grid.vertical_letter(2, 0),
            Letter { value: 'e', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(2, 1),
            Letter { value: 'h', state: LetterState::Movable },
        );
        assert_eq!(
            grid.vertical_letter(2, 2),
            Letter { value: 'm', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(2, 3),
            Letter { value: 'p', state: LetterState::Fixed },
        );
        assert_eq!(
            grid.vertical_letter(2, 4),
            Letter { value: 'u', state: LetterState::Movable },
        );
    }

    #[test]
    fn bad_character() {
        assert_eq!(
            "line 2: unexpected character: -",
            &"ABCDE\nA C -".parse::<LetterGrid>().unwrap_err().to_string(),
        );
        assert_eq!(
            "line 2: unexpected character: B",
            &"ABCDE\nABCDE".parse::<LetterGrid>().unwrap_err().to_string(),
        );
        assert_eq!(
            "line 1: unexpected character: U+0009",
            &"ABCD\t".parse::<LetterGrid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn bad_lowercase() {
        assert_eq!(
            "line 1: letter doesn’t have simple case: İ",
            &"ABCDİ".parse::<LetterGrid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn line_too_long() {
        assert_eq!(
            "line 1: line too long",
            &"ABCDEF".parse::<LetterGrid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn line_too_short() {
        assert_eq!(
            "line 1: line too short",
            &"ABCD".parse::<LetterGrid>().unwrap_err().to_string(),
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
                .parse::<LetterGrid>().unwrap_err().to_string(),
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
                .parse::<LetterGrid>().unwrap_err().to_string(),
        );
    }

    #[test]
    fn from_grid() {
        let grid = "ABCDEFGHIJKLMNOPQRSTUbacdefhjklmnoprtuvwxy"
            .parse::<grid::Grid>().unwrap();
        let letter_grid = LetterGrid::from_grid(&grid).unwrap();

        assert_eq!(
            &letter_grid.to_string(),
            "baCDE\n\
             F G H\n\
             IJKLM\n\
             N O P\n\
             QRSTU",
        );
    }
}
