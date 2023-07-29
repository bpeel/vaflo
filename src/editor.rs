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

mod grid;

use std::process::ExitCode;
use grid::{WORD_LENGTH, N_WORDS_ON_AXIS};

struct SolutionGrid {
    // The solution contains the actual letters. The grid is stored as
    // an array including positions for the gaps to make it easier to
    // index. The gaps will just be ignored.
    letters: [char; WORD_LENGTH * WORD_LENGTH]
}

struct PuzzleGrid {
    // The puzzle is stored is indices into the solution grid so that
    // changing a letter will change it in both grids
    positions: [usize; WORD_LENGTH * WORD_LENGTH]
}

struct GridPair {
    solution: SolutionGrid,
    puzzle: PuzzleGrid,
}

enum EditDirection {
    Right,
    Down,
}

enum GridChoice {
    Solution,
    Puzzle,
}

struct Editor {
    grid_x: i32,
    grid_y: i32,
    grid_pair: GridPair,
    cursor_x: i32,
    cursor_y: i32,
    edit_direction: EditDirection,
    current_grid: GridChoice,
    words: [String; N_WORDS_ON_AXIS * 2],
}

fn is_gap_space(x: i32, y: i32) -> bool {
    x & 1 == 1 && y & 1 == 1
}

impl SolutionGrid {
    fn new() -> SolutionGrid {
        SolutionGrid {
            letters: ['A'; WORD_LENGTH * WORD_LENGTH]
        }
    }

    fn draw(&self, grid_x: i32, grid_y: i32) {
        for y in 0..WORD_LENGTH {
            ncurses::mv(grid_y + y as i32, grid_x);

            for x in 0..WORD_LENGTH {
                if is_gap_space(x as i32, y as i32) {
                    ncurses::addch(' ' as u32);
                } else {
                    ncurses::addch(self.letters[y * WORD_LENGTH + x] as u32);
                }
            }
        }
    }
}

impl PuzzleGrid {
    fn new() -> PuzzleGrid {
        let mut positions: [usize; WORD_LENGTH * WORD_LENGTH]
            = Default::default();

        for (i, position) in positions.iter_mut().enumerate() {
            *position = i;
        }

        PuzzleGrid { positions }
    }

    fn draw(&self, grid_x: i32, grid_y: i32, solution: &SolutionGrid) {
        for y in 0..WORD_LENGTH {
            ncurses::mv(grid_y + y as i32, grid_x);

            for x in 0..WORD_LENGTH {
                if is_gap_space(x as i32, y as i32) {
                    ncurses::addch(' ' as u32);
                } else {
                    let position = self.positions[y * WORD_LENGTH + x];
                    let letter = solution.letters[position];
                    ncurses::addch(letter as u32);
                }
            }
        }
    }
}

impl GridPair {
    fn new() -> GridPair {
        GridPair {
            solution: SolutionGrid::new(),
            puzzle: PuzzleGrid::new(),
        }
    }

    fn puzzle_x() -> i32 {
        WORD_LENGTH.max(9) as i32 + 2
    }

    fn draw(&self, grid_x: i32, grid_y: i32) {
        ncurses::mvaddstr(grid_y, grid_x, "Solution:");
        self.solution.draw(grid_x, grid_y + 1);

        let grid_x = grid_x + GridPair::puzzle_x();
        ncurses::mvaddstr(grid_y, grid_x, "Puzzle:");
        self.puzzle.draw(grid_x, grid_y + 1, &self.solution);
    }
}

impl Editor {
    fn new(grid_x: i32, grid_y: i32) -> Editor {
        let mut editor = Editor {
            grid_x,
            grid_y,
            grid_pair: GridPair::new(),
            cursor_x: 0,
            cursor_y: 0,
            edit_direction: EditDirection::Right,
            current_grid: GridChoice::Solution,
            words: Default::default(),
        };

        editor.update_words();

        editor
    }

    fn redraw(&self) {
        ncurses::clear();
        self.grid_pair.draw(self.grid_x, self.grid_y);

        let direction_ch = match self.edit_direction {
            EditDirection::Right => '>',
            EditDirection::Down => 'v',
        };

        let right_side = self.grid_x
            + GridPair::puzzle_x()
            + WORD_LENGTH as i32
            + 5;

        ncurses::mvaddch(self.grid_y, right_side, direction_ch as u32);

        ncurses::mvaddstr(self.grid_y + 2, right_side, "Words:");

        for (i, word) in self.words.iter().enumerate() {
            ncurses::mvaddstr(
                self.grid_y + 3 + i as i32,
                right_side,
                word,
            );
        }

        self.position_cursor();
    }

    fn position_cursor(&self) {
        let x = match self.current_grid {
            GridChoice::Solution => 0,
            GridChoice::Puzzle => GridPair::puzzle_x(),
        };

        ncurses::mv(
            self.grid_y + 1 + self.cursor_y,
            self.grid_x + x + self.cursor_x,
        );
    }

    fn move_cursor(&mut self, x_offset: i32, y_offset: i32) {
        let mut x = self.cursor_x + x_offset;
        let mut y = self.cursor_y + y_offset;

        if is_gap_space(x, y) {
            x += x_offset;
            y += y_offset;
        }

        if x >= 0
            && x < WORD_LENGTH as i32
            && y >= 0
            && y < WORD_LENGTH as i32
        {
            self.cursor_x = x;
            self.cursor_y = y;
            self.position_cursor();
        }
    }

    fn toggle_grid(&mut self) {
        self.current_grid = match self.current_grid {
            GridChoice::Solution => GridChoice::Puzzle,
            GridChoice::Puzzle => GridChoice::Solution,
        };
        self.position_cursor();
    }

    fn toggle_edit_direction(&mut self) {
        self.edit_direction = match self.edit_direction {
            EditDirection::Right => EditDirection::Down,
            EditDirection::Down => EditDirection::Right,
        };
        self.redraw();
    }

    fn add_character(&mut self, ch: char) {
        let position = self.cursor_x as usize
            + self.cursor_y as usize * WORD_LENGTH;

        let position = match self.current_grid {
            GridChoice::Solution => position,
            GridChoice::Puzzle => self.grid_pair.puzzle.positions[position],
        };

        self.grid_pair.solution.letters[position] = ch;
        self.update_words();

        match self.edit_direction {
            EditDirection::Down => {
                if self.cursor_y + 1 < WORD_LENGTH as i32 {
                    self.cursor_y += 1;
                    if is_gap_space(self.cursor_x, self.cursor_y) {
                        self.cursor_y += 1;
                    }
                }
            },
            EditDirection::Right => {
                if self.cursor_x + 1 < WORD_LENGTH as i32 {
                    self.cursor_x += 1;
                    if is_gap_space(self.cursor_x, self.cursor_y) {
                        self.cursor_x += 1;
                    }
                }
            },
        }

        self.redraw();
    }

    fn handle_key_code(&mut self, key: i32) {
        match key {
            ncurses::KEY_UP => self.move_cursor(0, -1),
            ncurses::KEY_DOWN => self.move_cursor(0, 1),
            ncurses::KEY_LEFT => self.move_cursor(-1, 0),
            ncurses::KEY_RIGHT => self.move_cursor(1, 0),
            _ => (),
        }
    }

    fn handle_char(&mut self, ch: ncurses::winttype) {
        if let Some(ch) = char::from_u32(ch as u32) {
            match ch {
                '\t' => self.toggle_grid(),
                '.' => self.toggle_edit_direction(),
                ch if ch.is_alphabetic() => {
                    for ch in ch.to_uppercase() {
                        self.add_character(ch);
                    }
                },
                _ => (),
            }
        }
    }

    fn handle_key(&mut self, key: ncurses::WchResult) {
        match key {
            ncurses::WchResult::KeyCode(code) => self.handle_key_code(code),
            ncurses::WchResult::Char(ch) => self.handle_char(ch),
        }
    }

    fn update_words(&mut self) {
        for word in 0..N_WORDS_ON_AXIS {
            let horizontal = &mut self.words[word];
            horizontal.clear();
            horizontal.extend((0..WORD_LENGTH).map(|pos| {
                self.grid_pair.solution.letters[pos + word * WORD_LENGTH * 2]
            }));

            let vertical = &mut self.words[word + N_WORDS_ON_AXIS];
            vertical.clear();
            vertical.extend((0..WORD_LENGTH).map(|pos| {
                self.grid_pair.solution.letters[pos * WORD_LENGTH + word * 2]
            }));
        }
    }
}

fn main() -> ExitCode {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    ncurses::initscr();
    ncurses::noecho();
    ncurses::keypad(ncurses::stdscr(), true);

    let mut editor = Editor::new(0, 0);

    editor.redraw();

    loop {
        if let Some(key) = ncurses::get_wch() {
            editor.handle_key(key);
        }
    }

    ncurses::endwin();

    ExitCode::SUCCESS
}
