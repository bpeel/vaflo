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

#[derive(Clone, Copy)]
enum PuzzleSquareState {
    Correct,
    WrongPosition,
    Wrong,
}

#[derive(Clone, Copy)]
struct PuzzleSquare {
    position: usize,
    state: PuzzleSquareState,
}

struct PuzzleGrid {
    // The puzzle is stored is indices into the solution grid so that
    // changing a letter will change it in both grids
    squares: [PuzzleSquare; WORD_LENGTH * WORD_LENGTH]
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
    should_quit: bool,
    grid_x: i32,
    grid_y: i32,
    grid_pair: GridPair,
    cursor_x: i32,
    cursor_y: i32,
    edit_direction: EditDirection,
    current_grid: GridChoice,
    words: [String; N_WORDS_ON_AXIS * 2],
    selected_position: Option<usize>,
}

fn is_gap_space(x: i32, y: i32) -> bool {
    x & 1 == 1 && y & 1 == 1
}

fn addch_utf8(ch: char) {
    let mut buf = [0u8; 4];

    ncurses::addstr(ch.encode_utf8(&mut buf));
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
                    addch_utf8(self.letters[y * WORD_LENGTH + x]);
                }
            }
        }
    }
}

impl PuzzleGrid {
    fn new() -> PuzzleGrid {
        let default_square = PuzzleSquare {
            position: 0,
            state: PuzzleSquareState::Correct,
        };

        let mut squares = [default_square; WORD_LENGTH * WORD_LENGTH];

        for (i, square) in squares.iter_mut().enumerate() {
            square.position = i;
        }

        PuzzleGrid { squares }
    }

    fn draw(
        &self,
        grid_x: i32,
        grid_y: i32,
        solution: &SolutionGrid,
        selected_position: Option<usize>,
    ) {
        for y in 0..WORD_LENGTH {
            ncurses::mv(grid_y + y as i32, grid_x);

            for x in 0..WORD_LENGTH {
                if is_gap_space(x as i32, y as i32) {
                    ncurses::addch(' ' as u32);
                } else {
                    let square = self.squares[y * WORD_LENGTH + x];
                    let is_selected = selected_position
                        .map(|p| p == square.position)
                        .unwrap_or(false);

                    if is_selected {
                        ncurses::attron(ncurses::A_BOLD());
                    }

                    ncurses::attron(ncurses::COLOR_PAIR(square.state.color()));

                    addch_utf8(solution.letters[square.position]);

                    ncurses::attroff(ncurses::COLOR_PAIR(square.state.color()));

                    if is_selected {
                        ncurses::attroff(ncurses::A_BOLD());
                    }
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

    fn draw(&self, grid_x: i32, grid_y: i32, selected_position: Option<usize>) {
        ncurses::mvaddstr(grid_y, grid_x, "Solution:");
        self.solution.draw(grid_x, grid_y + 1);

        let grid_x = grid_x + GridPair::puzzle_x();
        ncurses::mvaddstr(grid_y, grid_x, "Puzzle:");
        self.puzzle.draw(
            grid_x, grid_y + 1,
            &self.solution,
            selected_position,
        );
    }

    fn update_square_letters_for_word<I>(&mut self, positions: I)
    where
        I: IntoIterator<Item = usize> + Clone
    {
        let mut used_letters = 0;

        // Mark all of the letters in the correct position already as used
        for (i, position) in positions.clone().into_iter().enumerate() {
            if matches!(
                self.puzzle.squares[position].state,
                PuzzleSquareState::Correct,
            ) {
                used_letters |= 1 << i;
            }
        }

        for position in positions.clone() {
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
                    if matches!(
                        square.state,
                        PuzzleSquareState::WrongPosition,
                    ) {
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

    fn update_square_states(&mut self) {
        for (i, square) in self.puzzle.squares.iter_mut().enumerate() {
            if self.solution.letters[i]
                == self.solution.letters[square.position]
            {
                square.state = PuzzleSquareState::Correct;
            } else {
                square.state = PuzzleSquareState::Wrong;
            }
        }

        for i in 0..N_WORDS_ON_AXIS {
            self.update_square_letters_for_word(
                i * 2 * WORD_LENGTH..(i * 2 + 1) * WORD_LENGTH,
            );
            self.update_square_letters_for_word(
                (i..i + WORD_LENGTH * WORD_LENGTH).step_by(WORD_LENGTH),
            );
        }
    }
}

impl PuzzleSquareState {
    fn color(&self) -> i16 {
        *self as i16 + 1
    }
}

impl Editor {
    fn new(grid_x: i32, grid_y: i32) -> Editor {
        let mut editor = Editor {
            should_quit: false,
            grid_x,
            grid_y,
            grid_pair: GridPair::new(),
            cursor_x: 0,
            cursor_y: 0,
            edit_direction: EditDirection::Right,
            current_grid: GridChoice::Solution,
            words: Default::default(),
            selected_position: None,
        };

        editor.update_words();

        editor
    }

    fn redraw(&self) {
        ncurses::clear();
        self.grid_pair.draw(self.grid_x, self.grid_y, self.selected_position);

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

    fn backspace(&mut self) {
        match self.edit_direction {
            EditDirection::Right => self.move_cursor(-1, 0),
            EditDirection::Down => self.move_cursor(0, -1),
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
            GridChoice::Puzzle => {
                self.grid_pair.puzzle.squares[position].position
            },
        };

        self.grid_pair.solution.letters[position] = ch;
        self.update_words();
        self.grid_pair.update_square_states();

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
            ncurses::KEY_BACKSPACE => self.backspace(),
            _ => (),
        }
    }

    fn cursor_pos(&self) -> usize {
        self.cursor_x as usize + self.cursor_y as usize * WORD_LENGTH
    }

    fn handle_mark(&mut self) {
        if matches!(self.current_grid, GridChoice::Puzzle) {
            self.selected_position = Some(self.cursor_pos());

            self.redraw();
        }
    }

    fn handle_swap(&mut self) {
        if matches!(self.current_grid, GridChoice::Puzzle) {
            if let Some(pos) = self.selected_position {
                let cursor_pos = self.cursor_pos();
                self.grid_pair.puzzle.squares.swap(pos, cursor_pos);
                self.selected_position = None;
                self.grid_pair.update_square_states();
                self.redraw();
            }
        }
    }

    fn handle_char(&mut self, ch: ncurses::winttype) {
        if let Some(ch) = char::from_u32(ch as u32) {
            match ch {
                '\t' => self.toggle_grid(),
                '.' => self.toggle_edit_direction(),
                ' ' => self.handle_mark(),
                '\u{0003}' => self.should_quit = true, // Ctrl+C
                '\u{0013}' => self.handle_swap(), // Ctrl+S
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
    ncurses::raw();
    ncurses::noecho();
    ncurses::keypad(ncurses::stdscr(), true);
    ncurses::start_color();

    ncurses::init_pair(
        PuzzleSquareState::Correct.color(),
        ncurses::COLOR_GREEN,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        PuzzleSquareState::WrongPosition.color(),
        ncurses::COLOR_YELLOW,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        PuzzleSquareState::Wrong.color(),
        ncurses::COLOR_WHITE,
        ncurses::COLOR_BLACK,
    );

    let mut editor = Editor::new(0, 0);

    editor.redraw();

    while !editor.should_quit {
        if let Some(key) = ncurses::get_wch() {
            editor.handle_key(key);
        }
    }

    ncurses::endwin();

    ExitCode::SUCCESS
}
