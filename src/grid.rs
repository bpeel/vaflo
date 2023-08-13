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

pub const WORD_LENGTH: usize = 5;
pub const N_WORDS_ON_AXIS: usize = (WORD_LENGTH + 1) / 2;
// The number of letters not at an intersection per word
pub const N_SPACING_LETTERS: usize = WORD_LENGTH - N_WORDS_ON_AXIS;
// Total number of letters in the grid
pub const N_LETTERS: usize =
    (WORD_LENGTH + N_SPACING_LETTERS) * N_WORDS_ON_AXIS;

use std::fmt;

#[derive(Clone, Debug)]
pub struct SolutionGrid {
    // The solution contains the actual letters. The grid is stored as
    // an array including positions for the gaps to make it easier to
    // index. The gaps will just be ignored.
    pub letters: [char; WORD_LENGTH * WORD_LENGTH]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PuzzleSquareState {
    Correct,
    WrongPosition,
    Wrong,
}

#[derive(Clone, Copy, Debug)]
pub struct PuzzleSquare {
    pub position: usize,
    pub state: PuzzleSquareState,
}

#[derive(Clone, Debug)]
pub struct PuzzleGrid {
    // The puzzle is stored is indices into the solution grid so that
    // changing a letter will change it in both grids
    pub squares: [PuzzleSquare; WORD_LENGTH * WORD_LENGTH]
}

#[derive(Clone, Debug)]
pub struct Grid {
    pub solution: SolutionGrid,
    pub puzzle: PuzzleGrid,
}

#[derive(Debug)]
pub enum GridParseError {
    NonUppercaseLetter,
    TooShort,
    TooLong,
    DuplicateIndex,
    InvalidIndex,
}

pub fn is_gap_space(x: i32, y: i32) -> bool {
    x & 1 == 1 && y & 1 == 1
}

pub fn is_gap_position(position: usize) -> bool {
    is_gap_space(
        (position % WORD_LENGTH) as i32,
        (position / WORD_LENGTH) as i32,
    )
}

impl SolutionGrid {
    pub fn new() -> SolutionGrid {
        SolutionGrid {
            letters: ['A'; WORD_LENGTH * WORD_LENGTH]
        }
    }
}

impl PuzzleGrid {
    pub fn new() -> PuzzleGrid {
        let default_square = PuzzleSquare {
            position: 0,
            state: PuzzleSquareState::Correct,
        };

        let mut grid = PuzzleGrid {
            squares: [default_square; WORD_LENGTH * WORD_LENGTH],
        };

        grid.reset();

        grid
    }

    pub fn reset(&mut self) {
        for (i, square) in self.squares.iter_mut().enumerate() {
            square.position = i;
        }
    }

    pub fn is_solved(&self) -> bool {
        self.squares.iter().find(|square| {
            square.state != PuzzleSquareState::Correct
        }).is_none()
    }
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            solution: SolutionGrid::new(),
            puzzle: PuzzleGrid::new(),
        }
    }

    fn update_square_letters_for_word<I>(&mut self, positions: I)
    where
        I: IntoIterator<Item = usize> + Clone
    {
        let mut used_letters = 0;

        // Mark all of the letters in the correct position already as used
        for (i, position) in positions.clone().into_iter().enumerate() {
            if self.puzzle.squares[position].state
                == PuzzleSquareState::Correct
            {
                used_letters |= 1 << i;
            }
        }

        for position in positions.clone() {
            if self.puzzle.squares[position].state
                == PuzzleSquareState::Correct
            {
                continue;
            }

            let letter = self.solution.letters[position];
            let mut best_pos = None;

            for (i, position) in positions.clone().into_iter().enumerate() {
                let square = self.puzzle.squares[position];
                let puzzle_letter =
                    self.solution.letters[square.position];

                if used_letters & (1 << i) == 0 && puzzle_letter == letter {
                    // It’s better to use a letter in the
                    // WrongPosition state in case it was marked by a
                    // word that crosses this one because we don’t
                    // want to have two yellow letters for the same
                    // letter.
                    if square.state == PuzzleSquareState::WrongPosition {
                        best_pos = Some((i, position));
                        break;
                    } else if best_pos.is_none() {
                        best_pos = Some((i, position));
                    }
                }
            }

            if let Some((i, position)) = best_pos {
                used_letters |= 1 << i;
                self.puzzle.squares[position].state =
                    PuzzleSquareState::WrongPosition;
            }
        }
    }

    pub fn update_square_states(&mut self) {
        for (i, square) in self.puzzle.squares.iter_mut().enumerate() {
            if self.solution.letters[i]
                == self.solution.letters[square.position]
            {
                square.state = PuzzleSquareState::Correct;
            } else {
                square.state = PuzzleSquareState::Wrong;
            }
        }

        for word in WordPositions::new() {
            self.update_square_letters_for_word(word);
        }
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, letter) in self.solution.letters.iter().enumerate() {
            if !is_gap_position(i) {
                write!(f, "{}", letter)?;
            }
        }

        for (i, square) in self.puzzle.squares.iter().enumerate() {
            if !is_gap_position(i) {
                write!(
                    f,
                    "{}",
                    char::from_u32(
                        b'a' as u32
                            + square.position as u32
                    ).unwrap(),
                )?;
            }
        }

        Ok(())
    }
}

impl std::str::FromStr for Grid {
    type Err = GridParseError;

    fn from_str(s: &str) -> Result<Grid, GridParseError> {
        let mut grid = Grid::new();
        let mut chars = s.chars();

        for (i, letter) in grid.solution.letters.iter_mut().enumerate() {
            if is_gap_position(i) {
                continue;
            }

            match chars.next() {
                Some(ch) => {
                    if !ch.is_uppercase() {
                        return Err(GridParseError::NonUppercaseLetter);
                    }
                    *letter = ch;
                },
                None => return Err(GridParseError::TooShort),
            }
        }

        let mut used_positions = 0;

        for (i, square) in grid.puzzle.squares.iter_mut().enumerate() {
            if is_gap_position(i) {
                continue;
            }

            match chars.next() {
                Some(ch) => {
                    let Some(position) = (ch as usize).checked_sub('a' as usize)
                        .filter(|pos| {
                            *pos < WORD_LENGTH * WORD_LENGTH
                                && !is_gap_position(*pos)
                        })
                    else {
                        return Err(GridParseError::InvalidIndex);
                    };

                    if used_positions & (1 << position) != 0 {
                        return Err(GridParseError::DuplicateIndex);
                    }

                    square.position = position;

                    used_positions |= 1 << position;
                },
                None => return Err(GridParseError::TooShort),
            }
        }

        if chars.next().is_some() {
            return Err(GridParseError::TooLong);
        }

        grid.update_square_states();

        Ok(grid)
    }
}

impl fmt::Display for GridParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GridParseError::NonUppercaseLetter => {
                write!(f, "non-uppercase letter")
            },
            GridParseError::TooShort => write!(f, "too short"),
            GridParseError::TooLong => write!(f, "too long"),
            GridParseError::DuplicateIndex => write!(f, "duplicate index"),
            GridParseError::InvalidIndex => write!(f, "invalid index"),
        }
    }
}

#[derive(Clone)]
pub struct WordPositions {
    word_num: usize,
}

impl WordPositions {
    pub fn new() -> WordPositions {
        WordPositions { word_num: 0 }
    }
}

impl Iterator for WordPositions {
    type Item = std::iter::StepBy<std::ops::Range<usize>>;

    fn next(&mut self) -> Option<<WordPositions as Iterator>::Item> {
        if self.word_num >= N_WORDS_ON_AXIS * 2 {
            None
        } else {
            let i = self.word_num / 2;

            let positions = if self.word_num & 1 == 0 {
                (i * 2 * WORD_LENGTH..(i * 2 + 1) * WORD_LENGTH).step_by(1)
            } else {
                (i * 2..i * 2 + WORD_LENGTH * WORD_LENGTH).step_by(WORD_LENGTH)
            };

            self.word_num += 1;

            Some(positions)
        }
    }

    fn nth(&mut self, n: usize) -> Option<<WordPositions as Iterator>::Item> {
        self.word_num = self.word_num
            .saturating_add(n)
            .min(N_WORDS_ON_AXIS * 2);
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl std::iter::ExactSizeIterator for WordPositions {
    fn len(&self) -> usize {
        N_WORDS_ON_AXIS * 2 - self.word_num
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn grid_parse() {
        assert!(matches!(
            "aaaaaaaaaaaaaaaaaaaaa\
             abcdefhjklmnoprtuvwxy".parse::<Grid>(),
            Err(GridParseError::NonUppercaseLetter),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwx".parse::<Grid>(),
            Err(GridParseError::TooShort),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAA".parse::<Grid>(),
            Err(GridParseError::TooShort),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwxyz".parse::<Grid>(),
            Err(GridParseError::TooLong),
        ));
        // Index to a space
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdeghjklmnoprtuvwxy".parse::<Grid>(),
            Err(GridParseError::InvalidIndex),
        ));
        // Index too high
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwxz".parse::<Grid>(),
            Err(GridParseError::InvalidIndex),
        ));
        // Index too low
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwx@".parse::<Grid>(),
            Err(GridParseError::InvalidIndex),
        ));
        // Duplicate index
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             aacdefhjklmnoprtuvwxy".parse::<Grid>(),
            Err(GridParseError::DuplicateIndex),
        ));

        let grid = "ABCDEFHJKLMNOPRTUVWXY\
                    bacdefhjklmnoprtuvwxy".parse::<Grid>().unwrap();

        for pos in 0..WORD_LENGTH * WORD_LENGTH {
            if !is_gap_position(pos) {
                assert_eq!(
                    grid.solution.letters[pos],
                    char::from_u32(pos as u32 + 'A' as u32).unwrap(),
                );
            }
        }

        assert_eq!(grid.puzzle.squares[0].position, 1);
        assert_eq!(
            grid.puzzle.squares[0].state,
            PuzzleSquareState::WrongPosition,
        );
        assert_eq!(grid.puzzle.squares[1].position, 0);
        assert_eq!(
            grid.puzzle.squares[1].state,
            PuzzleSquareState::WrongPosition,
        );

        for pos in 2..WORD_LENGTH * WORD_LENGTH {
            let square = &grid.puzzle.squares[pos];
            assert_eq!(square.position, pos);
            assert_eq!(square.state, PuzzleSquareState::Correct);
        }
    }

    #[test]
    fn grid_display() {
        let tests = [
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy",
        ];

        for test in tests {
            assert_eq!(
                test,
                &test.parse::<Grid>().unwrap().to_string(),
            );
        }
    }

    #[test]
    fn duplicate_correct() {
        let grid = "KULPOEIKMANĜUIDPOMAĜI\
                    jlmorpaknbchdftwuyexv"
            .parse::<Grid>().unwrap();

        let row = &grid.puzzle.squares[
            WORD_LENGTH * (WORD_LENGTH - 1)..WORD_LENGTH * WORD_LENGTH
        ];

        assert_eq!(row[0].state, PuzzleSquareState::Correct);
        assert_eq!(row[1].state, PuzzleSquareState::WrongPosition);
        assert_eq!(row[2].state, PuzzleSquareState::Wrong);
        assert_eq!(row[3].state, PuzzleSquareState::Correct);
        assert_eq!(row[4].state, PuzzleSquareState::WrongPosition);
    }

    #[test]
    fn vertical_square_states() {
        let grid = "MORSAUUKROLASDOOURSOJ\
                    ardxnhpfmvulwtybkeocj"
            .parse::<Grid>().unwrap();

        let squares = &grid.puzzle.squares;

        assert_eq!(squares[4].state, PuzzleSquareState::Correct);
        assert_eq!(squares[9].state, PuzzleSquareState::Wrong);
        assert_eq!(squares[14].state, PuzzleSquareState::Correct);
        assert_eq!(squares[19].state, PuzzleSquareState::Correct);
        assert_eq!(squares[24].state, PuzzleSquareState::WrongPosition);
    }

    #[test]
    fn solved() {
        let grid = "MORSAUUKROLASDOOURSOJ\
                    ardxnhpfmvulwtybkeocj"
            .parse::<Grid>().unwrap();
        assert!(!grid.puzzle.is_solved());

        let grid = "MORSAUUKROLASDOOURSOJ\
                    arcdnhfjvlmewpxbukoty"
            .parse::<Grid>().unwrap();
        assert!(grid.puzzle.is_solved());
    }

    #[test]
    fn gaps() {
        assert_eq!(
            (0..WORD_LENGTH * WORD_LENGTH)
                .filter(|&position| !is_gap_position(position))
                .count(),
            N_LETTERS,
        );
    }

    #[test]
    fn word_positions() {
        let base_positions = WordPositions::new()
            .map(|positions| {
                positions.map(|pos| {
                    char::from_u32(pos as u32 + b'a' as u32).unwrap()
                }).collect::<String>()
            });

        let mut positions = base_positions.clone();

        // abcde
        // f h j
        // klmno
        // p r t
        // uvwxy

        assert_eq!(positions.len(), 6);
        assert_eq!(&positions.next().unwrap(), "abcde");
        assert_eq!(positions.len(), 5);
        assert_eq!(&positions.next().unwrap(), "afkpu");
        assert_eq!(positions.len(), 4);
        assert_eq!(&positions.next().unwrap(), "klmno");
        assert_eq!(&positions.next().unwrap(), "chmrw");
        assert_eq!(&positions.next().unwrap(), "uvwxy");
        assert_eq!(positions.len(), 1);
        assert_eq!(&positions.next().unwrap(), "ejoty");
        assert!(positions.next().is_none());
        assert_eq!(positions.len(), 0);

        let mut positions = base_positions.clone();

        assert_eq!(&positions.nth(0).unwrap(), "abcde");
        assert_eq!(&positions.nth(1).unwrap(), "klmno");
        assert_eq!(&positions.nth(2).unwrap(), "ejoty");
        assert!(WordPositions::new().nth(6).is_none());
    }
}
