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
use fmt::Write;
use super::grid;
use grid::Grid;
use std::str::FromStr;
use std::collections::HashMap;

pub const MAXIMUM_SWAPS: u32 = 15;

pub struct SaveState {
    grid: Grid,
    swaps_remaining: u32,
}

#[derive(Debug)]
pub enum ParseError {
    MissingColon,
    InvalidGrid(grid::GridParseError),
    InvalidSwapsRemaining,
}

#[derive(Debug)]
pub enum LoadSaveStatesError {
    MissingColon(usize),
    InvalidPuzzleNumber(usize),
    DuplicatePuzzle(usize),
    BadPuzzle(usize, ParseError),
}

impl SaveState {
    pub fn new(grid: Grid, swaps_remaining: u32) -> SaveState {
        assert!(swaps_remaining <= MAXIMUM_SWAPS);

        SaveState { grid, swaps_remaining }
    }

    pub fn swaps_remaining(&self) -> u32 {
        self.swaps_remaining
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }
}

impl fmt::Display for SaveState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.grid, self.swaps_remaining)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::MissingColon => write!(f, "missing colon"),
            ParseError::InvalidGrid(e) => write!(f, "{}", e),
            ParseError::InvalidSwapsRemaining => {
                write!(f, "the number of swaps remaining is invalid")
            },
        }
    }
}

impl fmt::Display for LoadSaveStatesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadSaveStatesError::MissingColon(state_num) => {
                write!(f, "missing colon in state {}", state_num)
            },
            LoadSaveStatesError::InvalidPuzzleNumber(state_num) => {
                write!(f, "invalid puzzle number in state {}", state_num)
            },
            LoadSaveStatesError::BadPuzzle(puzzle_num, error) => {
                write!(f, "puzzle {}: {}", puzzle_num, error)
            },
            LoadSaveStatesError::DuplicatePuzzle(puzzle_num) => {
                write!(f, "puzzle {} appears more than once", puzzle_num)
            },
        }
    }
}

impl From<grid::GridParseError> for ParseError {
    fn from(parse_error: grid::GridParseError) -> ParseError {
        ParseError::InvalidGrid(parse_error)
    }
}

impl FromStr for SaveState {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<SaveState, ParseError> {
        let Some((grid, swaps_remaining)) = s.split_once(':')
        else {
            return Err(ParseError::MissingColon);
        };

        let grid = grid.parse::<Grid>()?;

        let Ok(swaps_remaining) = swaps_remaining.parse::<u32>()
        else {
            return Err(ParseError::InvalidSwapsRemaining);
        };

        if swaps_remaining > MAXIMUM_SWAPS {
            return Err(ParseError::InvalidSwapsRemaining);
        }

        Ok(SaveState::new(grid, swaps_remaining))
    }
}

pub fn save_states_to_string<I>(states: I) -> String
where
    I: IntoIterator<Item = (usize, SaveState)>
{
    let mut result = String::new();

    for (puzzle_num, save_state) in states {
        if !result.is_empty() {
            result.push(',');
        }
        write!(&mut result, "{}:{}", puzzle_num, save_state).unwrap();
    }

    result
}

pub fn load_save_states(
    s: &str,
) -> Result<HashMap<usize, SaveState>, LoadSaveStatesError> {
    let mut states = HashMap::new();

    for (state_num, day_string) in s.split(',').enumerate() {
        let Some((puzzle_num, state_string)) = day_string.split_once(':')
        else {
            return Err(LoadSaveStatesError::MissingColon(state_num));
        };

        let Ok(puzzle_num) = puzzle_num.parse::<usize>()
        else {
            return Err(LoadSaveStatesError::InvalidPuzzleNumber(state_num));
        };

        if states.contains_key(&puzzle_num) {
            return Err(LoadSaveStatesError::DuplicatePuzzle(puzzle_num));
        }

        match state_string.parse::<SaveState>() {
            Ok(save_state) => {
                states.insert(puzzle_num, save_state);
            },
            Err(e) => {
                return Err(LoadSaveStatesError::BadPuzzle(puzzle_num, e));
            },
        }
    }

    Ok(states)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bad_string() {
        assert!(matches!(
            "".parse::<SaveState>(),
            Err(ParseError::MissingColon),
        ));

        assert!(matches!(
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy\
             :foo".parse::<SaveState>(),
            Err(ParseError::InvalidSwapsRemaining),
        ));

        assert!(matches!(
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy\
             :16".parse::<SaveState>(),
            Err(ParseError::InvalidSwapsRemaining),
        ));

        assert!(matches!(
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklm\
             :15".parse::<SaveState>(),
            Err(ParseError::InvalidGrid(grid::GridParseError::TooShort)),
        ));
    }

    #[test]
    fn parse() {
        let state_string = "ABCDEFHJKLMNOPRTUVWXY\
                            bacdefhjklmnoprtuvwxy\
                            :14";

        let save_state = state_string.parse::<SaveState>().unwrap();

        assert_eq!(state_string, &save_state.to_string());

        assert_eq!(
            &save_state.grid().to_string(),
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy",
        );

        assert_eq!(save_state.swaps_remaining(), 14);
    }

    #[test]
    fn bad_save_states() {
        assert!(matches!(
            load_save_states(""),
            Err(LoadSaveStatesError::MissingColon(0)),
        ));
        assert!(matches!(
            load_save_states(
                "0:\
                 ABCDEFHJKLMNOPRTUVWXY\
                 bacdefhjklmnoprtuvwxy:\
                 10,\
                 foo:\
                 ABCDEFHJKLMNOPRTUVWXY\
                 bacdefhjklmnoprtuvwxy:\
                 11,"
            ),
            Err(LoadSaveStatesError::InvalidPuzzleNumber(1)),
        ));
        assert!(matches!(
            load_save_states(
                "3:\
                 ABCDEFHJKLMNOPRTUVWXY\
                 bacdefhjklmnoprtuvwxy:\
                 10,\
                 3:\
                 ABCDEFHJKLMNOPRTUVWXY\
                 bacdefhjklmnoprtuvwxy:\
                 11"
            ),
            Err(LoadSaveStatesError::DuplicatePuzzle(3)),
        ));
        assert!(matches!(
            load_save_states(
                "3:\
                 ABCDEFHJKLMNOPRTUVWXY\
                 bacdefhjklmnopr"
            ),
            Err(LoadSaveStatesError::BadPuzzle(3, ParseError::MissingColon)),
        ));
    }

    #[test]
    fn test_load_save_states() {
        let save_states_string =
            "3:\
             ABCDEFHJKLMNOPRTUVWXY\
             abcdefhjklmnoprtuvwxy:\
             11,\
             4:\
             ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy:\
             10";

        let save_states = load_save_states(save_states_string).unwrap();

        let mut keys = save_states.keys().map(|&x| x).collect::<Vec<_>>();
        keys.sort_unstable();
        assert_eq!(&keys, &[3, 4]);

        assert_eq!(save_states[&3].swaps_remaining(), 11);
        assert_eq!(
            save_states[&3].grid().to_string(),
            "ABCDEFHJKLMNOPRTUVWXY\
             abcdefhjklmnoprtuvwxy",
        );
        assert_eq!(save_states[&4].swaps_remaining(), 10);
        assert_eq!(
            save_states[&4].grid().to_string(),
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy",
        );

        let mut save_states = save_states.into_iter().collect::<Vec<_>>();

        save_states.sort_unstable_by(|(a, _), (|b, _)| a.cmp(b));

        assert_eq!(
            &save_states_to_string(save_states),
            save_states_string,
        );
    }
}
