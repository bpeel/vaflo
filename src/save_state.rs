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
use grid::PuzzleSquareState;
use std::str::FromStr;
use std::collections::HashMap;
use super::stars::{MAXIMUM_SWAPS, MAXIMUM_STARS};

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

// Positions of the stars in the grid in the share text
static STAR_POSITIONS: [u32; MAXIMUM_STARS as usize + 1] = [
    0,
    1 << 12,
    (1 << 6) | (1 << 18),
    (1 << 6) | (1 << 12) | (1 << 18),
    (1 << 6) | (1 << 8) | (1 << 16) | (1 << 18),
    (1 << 6) | (1 << 8) | (1 << 12) | (1 << 16) | (1 << 18),
];

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

pub struct Statistics {
    star_counts: [u32; MAXIMUM_STARS as usize + 1],
    fail_count: u32,
    n_played: u32,
    total_stars: u32,
    current_streak: u32,
    best_streak: u32,
}

impl Statistics {
    pub fn new(save_states: &HashMap<usize, SaveState>) -> Statistics
    {
        let mut puzzles = save_states
            .iter()
            .map(|(&k, v)| (k, v))
            .collect::<Vec<_>>();

        puzzles.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

        let mut star_counts = [0; MAXIMUM_STARS as usize + 1];
        let mut fail_count = 0;
        let n_played = puzzles.len() as u32;
        let mut total_stars = 0;
        let mut current_streak = 0;
        let mut best_streak = 0;
        let mut last_puzzle_num = None;

        for (puzzle_num, save_state) in puzzles {
            let streak_continued = last_puzzle_num.map(|last_puzzle_num| {
                last_puzzle_num + 1 == puzzle_num
            }).unwrap_or(false);

            if !streak_continued {
                current_streak = 0;
            }

            if save_state.grid().puzzle.is_solved() {
                current_streak += 1;

                if current_streak > best_streak {
                    best_streak = current_streak;
                }

                let stars = save_state.swaps_remaining().min(MAXIMUM_STARS);

                star_counts[stars as usize] += 1;
                total_stars += stars;
            } else {
                current_streak = 0;

                if save_state.swaps_remaining() <= 0 {
                    fail_count += 1;
                }
            }

            last_puzzle_num = Some(puzzle_num);
        }

        Statistics {
            star_counts,
            fail_count,
            n_played,
            total_stars,
            current_streak,
            best_streak,
        }
    }

    pub fn star_count(&self, n_stars: u32) -> u32 {
        self.star_counts[n_stars as usize]
    }

    pub fn fail_count(&self) -> u32 {
        self.fail_count
    }

    pub fn n_played(&self) -> u32 {
        self.n_played
    }

    pub fn total_stars(&self) -> u32 {
        self.total_stars
    }

    pub fn current_streak(&self) -> u32 {
        self.current_streak
    }

    pub fn best_streak(&self) -> u32 {
        self.best_streak
    }

    pub fn share_text(
        &self,
        puzzle_num: usize,
        save_state: &SaveState,
    ) -> String {
        let mut results = String::new();

        let is_solved = save_state.grid.puzzle.is_solved();

        write!(results, "#shawffle{} ", puzzle_num + 1).unwrap();

        if is_solved {
            write!(results, "{}", save_state.swaps_remaining).unwrap();
        } else {
            results.push('X');
        }

        write!(results, "/{}\n\n", MAXIMUM_STARS).unwrap();

        let star_positions = STAR_POSITIONS[
            if is_solved {
                save_state.swaps_remaining.min(MAXIMUM_STARS) as usize
            } else {
                0
            }
        ];

        for y in 0..grid::WORD_LENGTH {
            for x in 0..grid::WORD_LENGTH {
                let position = y * grid::WORD_LENGTH + x;

                let ch = if star_positions & (1 << position) != 0 {
                    '⭐'
                } else if grid::is_gap_space(x as i32, y as i32) {
                    '⬜'
                } else {
                    match save_state.grid.puzzle.squares[position].state {
                        PuzzleSquareState::Correct => '🟩',
                        PuzzleSquareState::WrongPosition
                            | PuzzleSquareState::Wrong => '⬛',
                    }
                };

                results.push(ch);
            }

            results.push('\n');
        }

        write!(
            results,
            "\n\
             {} streak: {}\n\
             https://vaflo.net",
            if is_solved {
                '🔥'
            } else {
                '💔'
            },
            self.current_streak(),
        ).unwrap();

        results
    }
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

    fn add_puzzle(
        puzzle_num: usize,
        grid: &str,
        swaps_remaining: usize,
        buf: &mut String,
    ) {
        if !buf.is_empty() {
            buf.push(',');
        }

        write!(
            buf,
            "{}:{}:{}",
            puzzle_num,
            grid,
            swaps_remaining
        ).unwrap();
    }

    fn add_unsolved(
        puzzle_num: usize,
        swaps_remaining: usize,
        buf: &mut String,
    ) {
        add_puzzle(
            puzzle_num,
            "MORSAUUKROLASDOOURSOJ\
             ardxnhpfmvulwtybkeocj",
            swaps_remaining,
            buf,
        );
    }

    fn add_fail(puzzle_num: usize, buf: &mut String) {
        add_unsolved(puzzle_num, 0, buf);
    }

    fn add_unfinished(puzzle_num: usize, buf: &mut String) {
        add_unsolved(puzzle_num, 1, buf);
    }

    fn add_solved(puzzle_num: usize, n_stars: usize, buf: &mut String) {
        add_puzzle(
            puzzle_num,
            "MORSAUUKROLASDOOURSOJ\
             arcdnhfjvlmewpxbukoty",
            n_stars,
            buf,
        );
    }

    #[test]
    fn statistics() {
        let mut buf = String::new();

        add_solved(0, 0, &mut buf);
        add_solved(1, 1, &mut buf);
        add_solved(2, 2, &mut buf);
        add_solved(3, 3, &mut buf);
        add_solved(4, 4, &mut buf);
        add_solved(5, 5, &mut buf);

        add_fail(6, &mut buf);

        add_solved(7, 2, &mut buf);
        add_solved(8, 3, &mut buf);

        let statistics = Statistics::new(&load_save_states(&buf).unwrap());

        assert_eq!(statistics.best_streak(), 6);
        assert_eq!(statistics.current_streak(), 2);
        assert_eq!(statistics.star_count(0), 1);
        assert_eq!(statistics.star_count(1), 1);
        assert_eq!(statistics.star_count(2), 2);
        assert_eq!(statistics.star_count(3), 2);
        assert_eq!(statistics.star_count(4), 1);
        assert_eq!(statistics.star_count(5), 1);
        assert_eq!(statistics.fail_count(), 1);
        assert_eq!(statistics.n_played(), 9);
        assert_eq!(
            statistics.total_stars(),
            0 * 1
                + 1 * 1
                + 2 * 2
                + 3 * 2
                + 4 * 1
                + 5 * 1,
        );
    }

    #[test]
    fn unfinished_statistics() {
        let mut buf = String::new();

        add_solved(4, 4, &mut buf);
        add_solved(5, 5, &mut buf);

        add_unfinished(6, &mut buf);

        add_solved(7, 2, &mut buf);

        let statistics = Statistics::new(&load_save_states(&buf).unwrap());

        assert_eq!(statistics.best_streak(), 2);
        assert_eq!(statistics.current_streak(), 1);
        assert_eq!(statistics.n_played(), 4);
        assert_eq!(statistics.total_stars(), 4 + 5 + 2);
    }

    #[test]
    fn statistics_gap() {
        let mut buf = String::new();

        add_solved(4, 4, &mut buf);
        add_solved(5, 5, &mut buf);

        add_solved(7, 2, &mut buf);

        let statistics = Statistics::new(&load_save_states(&buf).unwrap());

        assert_eq!(statistics.best_streak(), 2);
        assert_eq!(statistics.current_streak(), 1);
        assert_eq!(statistics.n_played(), 3);
        assert_eq!(statistics.total_stars(), 4 + 5 + 2);
    }

    fn make_save_states_for_stars(n_stars: usize) -> HashMap<usize, SaveState> {
        let mut buf = String::new();
        add_solved(4, n_stars, &mut buf);
        load_save_states(&buf).unwrap()
    }

    #[test]
    fn share_text_solved() {
        let save_states = make_save_states_for_stars(0);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 0/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⬜🟩⬜🟩\n\
             🟩🟩🟩🟩🟩\n\
             🟩⬜🟩⬜🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );

        let save_states = make_save_states_for_stars(1);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 1/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⬜🟩⬜🟩\n\
             🟩🟩⭐🟩🟩\n\
             🟩⬜🟩⬜🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );

        let save_states = make_save_states_for_stars(2);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 2/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⭐🟩⬜🟩\n\
             🟩🟩🟩🟩🟩\n\
             🟩⬜🟩⭐🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );

        let save_states = make_save_states_for_stars(3);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 3/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⭐🟩⬜🟩\n\
             🟩🟩⭐🟩🟩\n\
             🟩⬜🟩⭐🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );

        let save_states = make_save_states_for_stars(4);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 4/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⭐🟩⭐🟩\n\
             🟩🟩🟩🟩🟩\n\
             🟩⭐🟩⭐🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );

        let save_states = make_save_states_for_stars(5);
        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 5/5\n\
             \n\
             🟩🟩🟩🟩🟩\n\
             🟩⭐🟩⭐🟩\n\
             🟩🟩⭐🟩🟩\n\
             🟩⭐🟩⭐🟩\n\
             🟩🟩🟩🟩🟩\n\
             \n\
             🔥 streak: 1\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );
    }

    #[test]
    fn share_text_failed() {
        let mut buf = String::new();
        add_fail(4, &mut buf);
        let save_states = load_save_states(&buf).unwrap();

        let statistics = Statistics::new(&save_states);
        let save_state = save_states.values().next().unwrap();

        assert_eq!(
            "#shawffle5 X/5\n\
             \n\
             🟩🟩⬛⬛🟩\n\
             🟩⬜⬛⬜⬛\n\
             ⬛⬛⬛⬛🟩\n\
             ⬛⬜⬛⬜🟩\n\
             ⬛⬛🟩⬛⬛\n\
             \n\
             💔 streak: 0\n\
             https://vaflo.net",
            &statistics.share_text(4, &save_state)
        );
    }

    #[test]
    fn share_text_stars() {
        let grid = "MORSAUUKROLASDOOURSOJ\
                    arcdnhfjvlmewpxbukoty"
            .parse::<Grid>()
            .unwrap();

        let mut buf = String::new();
        add_solved(4, 4, &mut buf);
        let statistics = Statistics::new(&load_save_states(&buf).unwrap());

        for n_stars in 0..=MAXIMUM_STARS {
            let save_state = SaveState::new(grid.clone(), n_stars);

            let share_text = statistics.share_text(1, &save_state);
            let n_stars_in_share_text = share_text.chars()
                .filter(|&ch| ch == '⭐')
                .count();

            assert_eq!(n_stars_in_share_text, n_stars as usize);
        }
    }
}
