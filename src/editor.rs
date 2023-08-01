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
mod dictionary;
mod word_grid;
mod word_solver;
mod grid_solver;
mod permute;
mod pairs;
mod swap_solver;

use std::process::ExitCode;
use grid::{WORD_LENGTH, N_WORDS_ON_AXIS, N_LETTERS};
use dictionary::Dictionary;
use std::ffi::c_int;
use std::sync::{Arc, mpsc};
use std::thread;
use word_grid::WordGrid;
use grid_solver::GridSolver;
use std::fmt;
use std::io::{BufRead, Write};
use rand::Rng;

// Number of swaps to make when shuffling the puzzle
const N_SHUFFLE_SWAPS: usize = 10;

#[derive(Clone, Debug)]
struct SolutionGrid {
    // The solution contains the actual letters. The grid is stored as
    // an array including positions for the gaps to make it easier to
    // index. The gaps will just be ignored.
    letters: [char; WORD_LENGTH * WORD_LENGTH]
}

#[derive(Clone, Copy, Debug)]
enum PuzzleSquareState {
    Correct,
    WrongPosition,
    Wrong,
}

#[derive(Clone, Copy, Debug)]
struct PuzzleSquare {
    position: usize,
    state: PuzzleSquareState,
}

#[derive(Clone, Debug)]
struct PuzzleGrid {
    // The puzzle is stored is indices into the solution grid so that
    // changing a letter will change it in both grids
    squares: [PuzzleSquare; WORD_LENGTH * WORD_LENGTH]
}

#[derive(Clone, Debug)]
struct GridPair {
    solution: SolutionGrid,
    puzzle: PuzzleGrid,
}

#[derive(Debug)]
enum GridPairParseError {
    NonUppercaseLetter,
    TooShort,
    TooLong,
    DuplicateIndex,
    InvalidIndex,
}

enum EditDirection {
    Right,
    Down,
}

enum GridChoice {
    Solution,
    Puzzle,
}

#[derive(Default)]
struct Word {
    valid: bool,
    text: String,
}

struct Editor {
    dictionary: Arc<Dictionary>,
    grid_senders: Vec<mpsc::Sender<(usize, GridPair)>>,
    should_quit: bool,
    grid_x: i32,
    grid_y: i32,
    current_puzzle: usize,
    puzzles: Vec<GridPair>,
    cursor_x: i32,
    cursor_y: i32,
    edit_direction: EditDirection,
    current_grid: GridChoice,
    words: [Word; N_WORDS_ON_AXIS * 2],
    selected_position: Option<usize>,
    grid_id: usize,
    solutions: Vec<WordGrid>,
    shortest_swap_solution: Option<usize>,
}

enum SolutionEventKind {
    Grid(WordGrid),
    SwapSolution(usize),
}

struct SolutionEvent {
    id: usize,
    kind: SolutionEventKind,
}

struct SolverThread {
    word_join_handle: thread::JoinHandle<()>,
    word_grid_sender: mpsc::Sender<(usize, GridPair)>,
    swap_join_handle: thread::JoinHandle<()>,
    swap_grid_sender: mpsc::Sender<(usize, GridPair)>,
    event_receiver: mpsc::Receiver<SolutionEvent>,
}

fn is_gap_space(x: i32, y: i32) -> bool {
    x & 1 == 1 && y & 1 == 1
}

fn is_gap_position(position: usize) -> bool {
    is_gap_space(
        (position % WORD_LENGTH) as i32,
        (position / WORD_LENGTH) as i32,
    )
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

        let mut grid = PuzzleGrid {
            squares: [default_square; WORD_LENGTH * WORD_LENGTH],
        };

        grid.reset();

        grid
    }

    fn reset(&mut self) {
        for (i, square) in self.squares.iter_mut().enumerate() {
            square.position = i;
        }
    }

    fn shuffle(&mut self) {
        self.reset();

        let mut used_squares = 0;
        let mut rng = rand::thread_rng();

        // Make 10 random swaps out of squares that aren’t involved in
        // previous swaps
        for swap_num in 0..N_SHUFFLE_SWAPS {
            let n_positions = N_LETTERS - swap_num * 2;
            let a = rng.gen_range(0..n_positions - 1);
            let b = rng.gen_range(a + 1..n_positions);

            let mut positions = (0..WORD_LENGTH * WORD_LENGTH)
                .filter(|&pos| {
                    !is_gap_position(pos)
                        && used_squares & (1 << pos) == 0
                });

            assert_eq!(n_positions, positions.clone().count());

            let a_pos = positions.nth(a).unwrap();
            let b_pos = positions.nth(b - a - 1).unwrap();

            self.squares.swap(a_pos, b_pos);

            used_squares |= (1 << a_pos) | (1 << b_pos)
        }
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
                        .map(|p| p == y * WORD_LENGTH + x)
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
            if matches!(
                self.puzzle.squares[position].state,
                PuzzleSquareState::Correct,
            ) {
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

    fn to_grid(&self) -> Result<grid::Grid, grid::ParseError> {
        let mut grid_string = String::new();

        for y in 0..WORD_LENGTH {
            for x in 0..WORD_LENGTH {
                if is_gap_space(x as i32, y as i32) {
                    grid_string.push(' ');
                } else {
                    let pos = x + y * WORD_LENGTH;
                    let square = &self.puzzle.squares[pos];
                    let letter = self.solution.letters[square.position];

                    match square.state {
                        PuzzleSquareState::Correct => {
                            grid_string.extend(letter.to_uppercase());
                        },
                        PuzzleSquareState::Wrong
                            | PuzzleSquareState::WrongPosition =>
                        {
                            grid_string.extend(letter.to_lowercase());
                        },
                    }
                }
            }

            grid_string.push('\n');
        }

        grid_string.parse::<grid::Grid>()
    }

    fn minimum_swaps(&self) -> Option<usize> {
        let solution = self.solution
            .letters
            .iter()
            .map(|&letter| letter)
            .collect::<Vec<char>>();
        let puzzle = self.puzzle
            .squares
            .iter()
            .map(|square| self.solution.letters[square.position])
            .collect::<Vec<char>>();

        swap_solver::solve(&puzzle, &solution).map(|solution| solution.len())
    }
}

impl fmt::Display for GridPair {
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

impl std::str::FromStr for GridPair {
    type Err = GridPairParseError;

    fn from_str(s: &str) -> Result<GridPair, GridPairParseError> {
        let mut grid_pair = GridPair::new();
        let mut chars = s.chars();

        for (i, letter) in grid_pair.solution.letters.iter_mut().enumerate() {
            if is_gap_position(i) {
                continue;
            }

            match chars.next() {
                Some(ch) => {
                    if !ch.is_uppercase() {
                        return Err(GridPairParseError::NonUppercaseLetter);
                    }
                    *letter = ch;
                },
                None => return Err(GridPairParseError::TooShort),
            }
        }

        let mut used_positions = 0;

        for (i, square) in grid_pair.puzzle.squares.iter_mut().enumerate() {
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
                        return Err(GridPairParseError::InvalidIndex);
                    };

                    if used_positions & (1 << position) != 0 {
                        return Err(GridPairParseError::DuplicateIndex);
                    }

                    square.position = position;

                    used_positions |= 1 << position;
                },
                None => return Err(GridPairParseError::TooShort),
            }
        }

        if chars.next().is_some() {
            return Err(GridPairParseError::TooLong);
        }

        grid_pair.update_square_states();

        Ok(grid_pair)
    }
}

impl fmt::Display for GridPairParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GridPairParseError::NonUppercaseLetter => {
                write!(f, "non-uppercase letter")
            },
            GridPairParseError::TooShort => write!(f, "too short"),
            GridPairParseError::TooLong => write!(f, "too long"),
            GridPairParseError::DuplicateIndex => write!(f, "duplicate index"),
            GridPairParseError::InvalidIndex => write!(f, "invalid index"),
        }
    }
}

impl PuzzleSquareState {
    fn color(&self) -> i16 {
        *self as i16 + 1
    }
}

impl Editor {
    fn new(
        puzzles: Vec<GridPair>,
        dictionary: Arc<Dictionary>,
        grid_senders: Vec<mpsc::Sender<(usize, GridPair)>>,
        grid_x: i32,
        grid_y: i32,
    ) -> Editor {
        assert!(puzzles.len() > 0);

        let mut editor = Editor {
            dictionary,
            grid_senders,
            should_quit: false,
            grid_x,
            grid_y,
            current_puzzle: 0,
            puzzles,
            cursor_x: 0,
            cursor_y: 0,
            edit_direction: EditDirection::Right,
            current_grid: GridChoice::Solution,
            words: Default::default(),
            selected_position: None,
            grid_id: 0,
            solutions: Vec::new(),
            shortest_swap_solution: None,
        };

        editor.update_words();
        editor.send_grid();

        editor
    }

    fn redraw(&self) {
        ncurses::clear();
        self.puzzles[self.current_puzzle].draw(
            self.grid_x,
            self.grid_y,
            self.selected_position
        );

        let direction_ch = match self.edit_direction {
            EditDirection::Right => '>',
            EditDirection::Down => 'v',
        };

        let right_side = self.grid_x
            + GridPair::puzzle_x()
            + WORD_LENGTH as i32
            + 5;

        ncurses::mvaddch(self.grid_y, right_side, direction_ch as u32);

        ncurses::addstr(&format!(
            " {}/{}",
            self.current_puzzle + 1,
            self.puzzles.len(),
        ));

        ncurses::mvaddstr(self.grid_y + 2, right_side, "Words:");

        for (i, word) in self.words.iter().enumerate() {
            ncurses::mvaddstr(
                self.grid_y + 3 + i as i32,
                right_side,
                &word.text,
            );
            ncurses::addch(' ' as u32);
            ncurses::addstr(
                if word.valid {
                    "✅"
                } else {
                    "❌"
                }
            );
        }

        let mut y = self.grid_y + WORD_LENGTH as i32 + 3;

        if let Some(n_swaps) = self.shortest_swap_solution {
            ncurses::mvaddstr(
                y,
                self.grid_x,
                &format!("Minimum swaps: {}", n_swaps),
            );
            y += 2;
        }

        if !self.solutions.is_empty() {
            ncurses::mvaddstr(y, self.grid_x, "Solutions:");
            y += 2;

            let max_y = ncurses::getmaxy(ncurses::stdscr());

            for solution in self.solutions.iter() {
                if y + WORD_LENGTH as i32 > max_y {
                    break;
                }

                for line in solution.to_string().lines() {
                    if line.is_empty() {
                        break;
                    }
                    ncurses::mvaddstr(y, self.grid_x, line);
                    y += 1;
                }

                y += 1;
            }
        }

        self.position_cursor();

        ncurses::refresh();
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
            ncurses::refresh();
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
        ncurses::refresh();
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

        let grid_pair = &mut self.puzzles[self.current_puzzle];

        let position = match self.current_grid {
            GridChoice::Solution => position,
            GridChoice::Puzzle => {
                grid_pair.puzzle.squares[position].position
            },
        };

        grid_pair.solution.letters[position] = ch;
        grid_pair.update_square_states();
        self.update_words();
        self.send_grid();

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
            ncurses::KEY_NPAGE => self.move_between_puzzles(1),
            ncurses::KEY_PPAGE => self.move_between_puzzles(-1),
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
                let grid_pair = &mut self.puzzles[self.current_puzzle];
                grid_pair.puzzle.squares.swap(pos, cursor_pos);
                grid_pair.update_square_states();
                self.selected_position = None;
                self.send_grid();
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
                '\u{0012}' => self.shuffle_puzzle(), // Ctrl+R
                '\u{0013}' => self.handle_swap(), // Ctrl+S
                '\u{000e}' => self.new_puzzle(), // Ctrl+N
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
            let grid_pair = &self.puzzles[self.current_puzzle];

            let horizontal = &mut self.words[word];
            horizontal.text.clear();
            horizontal.text.extend((0..WORD_LENGTH).map(|pos| {
                grid_pair.solution.letters[pos + word * WORD_LENGTH * 2]
            }));

            let vertical = &mut self.words[word + N_WORDS_ON_AXIS];
            vertical.text.clear();
            vertical.text.extend((0..WORD_LENGTH).map(|pos| {
                grid_pair.solution.letters[pos * WORD_LENGTH + word * 2]
            }));
        }

        for word in self.words.iter_mut() {
            word.valid = self.dictionary.contains(word.text.chars());
        }
    }

    fn handle_solution_event(&mut self, event: SolutionEvent) {
        if event.id != self.grid_id {
            return;
        }

        match event.kind {
            SolutionEventKind::Grid(grid) => {
                self.solutions.push(grid);
                self.redraw();
            },
            SolutionEventKind::SwapSolution(n_swaps) => {
                self.shortest_swap_solution = Some(n_swaps);
                self.redraw();
            },
        }
    }

    fn send_grid(&mut self) {
        self.grid_id = self.grid_id.wrapping_add(1);
        self.solutions.clear();
        self.shortest_swap_solution = None;

        let grid_pair = self.puzzles[self.current_puzzle].clone();

        for grid_sender in self.grid_senders.iter() {
            let _ = grid_sender.send((self.grid_id, grid_pair.clone()));
        }
    }

    fn set_current_puzzle(&mut self, puzzle_num: usize) {
        if puzzle_num != self.current_puzzle {
            assert!(puzzle_num < self.puzzles.len());
            self.current_puzzle = puzzle_num;
            self.update_words();
            self.send_grid();
            self.redraw();
        }
    }

    fn move_between_puzzles(&mut self, offset: isize) {
        let next_puzzle = self.current_puzzle.saturating_add_signed(offset)
            .min(self.puzzles.len() - 1);
        self.set_current_puzzle(next_puzzle);
    }

    fn new_puzzle(&mut self) {
        self.puzzles.push(GridPair::new());
        self.set_current_puzzle(self.puzzles.len() - 1);
    }

    fn shuffle_puzzle(&mut self) {
        let grid_pair = &mut self.puzzles[self.current_puzzle];
        grid_pair.puzzle.shuffle();
        grid_pair.update_square_states();
        self.send_grid();
        self.redraw();
    }
}

fn load_dictionary() -> Result<Arc<Dictionary>, ()> {
    let data = match std::env::args_os().nth(1) {
        Some(filename) => {
            match std::fs::read(&filename) {
                Err(e) => {
                    eprintln!(
                        "{}: {}",
                        filename.to_string_lossy(),
                        e,
                    );
                    return Err(());
                },
                Ok(d) => d,
            }
        },
        None => Vec::new(),
    };

    Ok(Arc::new(Dictionary::new(data.into_boxed_slice())))
}

fn load_puzzles() -> Result<Vec<GridPair>, ()> {
    let filename = "puzzles.txt";
    let mut puzzles = Vec::new();

    let f = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(vec![GridPair::new()]);
            } else {
                eprintln!("{}: {}", filename, e);
                return Err(());
            }
        },
    };

    for (line_num, line) in std::io::BufReader::new(f).lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}: {}", filename, e);
                return Err(());
            },
        };

        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match line.parse::<GridPair>() {
            Ok(grid) => puzzles.push(grid),
            Err(e) => {
                eprintln!("{}:{}: {}", filename, line_num + 1, e);
                return Err(());
            },
        }
    }

    if puzzles.is_empty() {
        eprintln!("{}: empty file", filename);
        return Err(());
    }

    Ok(puzzles)
}

fn save_puzzles(puzzles: &[GridPair]) {
    let f = match std::fs::File::create("puzzles.txt.tmp") {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut writer = std::io::BufWriter::new(f);

    for puzzle in puzzles.iter() {
        if write!(writer, "{}\n", puzzle).is_err() {
            return;
        }
    }

    if writer.flush().is_err() {
        return;
    }

    std::mem::drop(writer);

    let _ = std::fs::rename("puzzles.txt.tmp", "puzzles.txt");
}

fn main_loop(
    editor: &mut Editor,
    solver_thread: &SolverThread,
    wakeup_fd: c_int,
) {
    while !editor.should_quit {
        let mut pollfds = [
            libc::pollfd {
                fd: libc::STDIN_FILENO,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: wakeup_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let poll_result = unsafe {
            libc::poll(
                &mut pollfds as *mut libc::pollfd,
                pollfds.len() as libc::nfds_t,
                -1, // timeout
            )
        };

        if poll_result < 0 {
            eprintln!("poll failed");
            break;
        }

        if (pollfds[0].revents | pollfds[1].revents)
            & (libc::POLLHUP | libc::POLLERR)
            != 0
        {
            break;
        }

        if pollfds[0].revents & libc::POLLIN != 0 {
            if let Some(key) = ncurses::get_wch() {
                editor.handle_key(key);
            }
        }

        if pollfds[1].revents & libc::POLLIN != 0 {
            let mut bytes = [0u8];

            let read_ret = unsafe {
                libc::read(wakeup_fd, bytes.as_mut_ptr().cast(), 1)
            };

            if read_ret <= 0 {
                break;
            }
        }

        for event in solver_thread.event_receiver.try_iter() {
            editor.handle_solution_event(event);
        }
    }
}

impl SolutionEvent {
    fn new(id: usize, kind: SolutionEventKind) -> SolutionEvent {
        SolutionEvent { id, kind }
    }
}

struct SkipReceiverIter<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> SkipReceiverIter<T> {
    fn new(receiver: mpsc::Receiver<T>) -> Self {
        SkipReceiverIter { receiver }
    }
}

impl<T> Iterator for SkipReceiverIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.receiver.try_iter().last().or_else(|| {
            self.receiver.recv().ok()
        })
    }
}

impl SolverThread {
    fn new(
        dictionary: Arc<Dictionary>,
        wakeup_fd: c_int,
    ) -> SolverThread {
        let (word_grid_sender, word_grid_receiver) =
            mpsc::channel::<(usize, GridPair)>();
        let (swap_grid_sender, swap_grid_receiver) =
            mpsc::channel::<(usize, GridPair)>();
        let (event_sender, event_receiver) = mpsc::channel();

        let word_event_sender = event_sender.clone();
        let swap_event_sender = event_sender;

        let word_join_handle = thread::spawn(move || {
            let wakeup_bytes = [b'!'];

            let skip_receiver = SkipReceiverIter::new(word_grid_receiver);

            for (grid_id, grid_pair) in skip_receiver {
                let Ok(grid) = grid_pair.to_grid()
                else {
                    continue;
                };

                let mut solver = GridSolver::new(
                    WordGrid::new(&grid),
                    &dictionary,
                );

                while let Some(solution) = solver.next() {
                    let event = SolutionEvent::new(
                        grid_id,
                        SolutionEventKind::Grid(solution),
                    );
                    if word_event_sender.send(event).is_err() {
                        break;
                    }
                    unsafe {
                        libc::write(wakeup_fd, wakeup_bytes.as_ptr().cast(), 1);
                    }
                }
            }
        });

        let swap_join_handle = thread::spawn(move || {
            let wakeup_bytes = [b'!'];

            let skip_receiver = SkipReceiverIter::new(swap_grid_receiver);

            for (grid_id, grid_pair) in skip_receiver {
                if let Some(n_swaps) = grid_pair.minimum_swaps() {
                    let event = SolutionEvent::new(
                        grid_id,
                        SolutionEventKind::SwapSolution(n_swaps),
                    );
                    if swap_event_sender.send(event).is_err() {
                        break;
                    }
                    unsafe {
                        libc::write(wakeup_fd, wakeup_bytes.as_ptr().cast(), 1);
                    }
                }
            }
        });

        SolverThread {
            word_join_handle,
            word_grid_sender,
            swap_join_handle,
            swap_grid_sender,
            event_receiver,
        }
    }

    fn join(self) {
        let SolverThread {
            word_join_handle,
            word_grid_sender,
            swap_join_handle,
            swap_grid_sender,
            event_receiver,
        } = self;

        // Drop the mpcs so that the thread will quit
        std::mem::drop(word_grid_sender);
        std::mem::drop(swap_grid_sender);
        std::mem::drop(event_receiver);

        let _ = word_join_handle.join();
        let _ = swap_join_handle.join();
    }
}

fn pipe() -> Result<(c_int, c_int), std::io::Error> {
    let mut fds = [0, 0];

    let pipe_result = unsafe {
        libc::pipe(fds.as_mut_ptr())
    };

    if pipe_result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok((fds[0], fds[1]))
    }
}

fn main() -> ExitCode {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    let Ok(dictionary) = load_dictionary()
    else {
        return ExitCode::FAILURE;
    };

    let Ok(puzzles) = load_puzzles()
    else {
        return ExitCode::FAILURE;
    };

    let (wakeup_read, wakeup_write) = match pipe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("pipe failed: {}", e);
            return ExitCode::FAILURE;
        },
    };

    ncurses::initscr();
    ncurses::raw();
    ncurses::noecho();
    ncurses::keypad(ncurses::stdscr(), true);
    ncurses::start_color();
    ncurses::nodelay(ncurses::stdscr(), true);

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

    let solver_thread = SolverThread::new(
        Arc::clone(&dictionary),
        wakeup_write
    );

    let mut editor = Editor::new(
        puzzles,
        dictionary,
        vec![
            solver_thread.word_grid_sender.clone(),
            solver_thread.swap_grid_sender.clone(),
        ],
        0,
        0,
    );

    editor.redraw();

    main_loop(&mut editor, &solver_thread, wakeup_read);

    save_puzzles(&editor.puzzles);

    std::mem::drop(editor);

    solver_thread.join();

    unsafe {
        libc::close(wakeup_read);
        libc::close(wakeup_write);
    }

    ncurses::endwin();

    ExitCode::SUCCESS
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn grid_pair_parse() {
        assert!(matches!(
            "aaaaaaaaaaaaaaaaaaaaa\
             abcdefhjklmnoprtuvwxy".parse::<GridPair>(),
            Err(GridPairParseError::NonUppercaseLetter),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwx".parse::<GridPair>(),
            Err(GridPairParseError::TooShort),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAA".parse::<GridPair>(),
            Err(GridPairParseError::TooShort),
        ));
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwxyz".parse::<GridPair>(),
            Err(GridPairParseError::TooLong),
        ));
        // Index to a space
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdeghjklmnoprtuvwxy".parse::<GridPair>(),
            Err(GridPairParseError::InvalidIndex),
        ));
        // Index too high
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwxz".parse::<GridPair>(),
            Err(GridPairParseError::InvalidIndex),
        ));
        // Index too low
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             abcdefhjklmnoprtuvwx@".parse::<GridPair>(),
            Err(GridPairParseError::InvalidIndex),
        ));
        // Duplicate index
        assert!(matches!(
            "AAAAAAAAAAAAAAAAAAAAA\
             aacdefhjklmnoprtuvwxy".parse::<GridPair>(),
            Err(GridPairParseError::DuplicateIndex),
        ));

        let grid = "ABCDEFHJKLMNOPRTUVWXY\
                    bacdefhjklmnoprtuvwxy".parse::<GridPair>().unwrap();

        for pos in 0..WORD_LENGTH * WORD_LENGTH {
            if !is_gap_position(pos) {
                assert_eq!(
                    grid.solution.letters[pos],
                    char::from_u32(pos as u32 + 'A' as u32).unwrap(),
                );
            }
        }

        assert_eq!(grid.puzzle.squares[0].position, 1);
        assert!(matches!(
            grid.puzzle.squares[0].state,
            PuzzleSquareState::WrongPosition,
        ));
        assert_eq!(grid.puzzle.squares[1].position, 0);
        assert!(matches!(
            grid.puzzle.squares[1].state,
            PuzzleSquareState::WrongPosition,
        ));

        for pos in 2..WORD_LENGTH * WORD_LENGTH {
            let square = &grid.puzzle.squares[pos];
            assert_eq!(square.position, pos);
            assert!(matches!(square.state, PuzzleSquareState::Correct));
        }
    }

    #[test]
    fn grid_pair_display() {
        let tests = [
            "ABCDEFHJKLMNOPRTUVWXY\
             bacdefhjklmnoprtuvwxy",
        ];

        for test in tests {
            assert_eq!(
                test,
                &test.parse::<GridPair>().unwrap().to_string(),
            );
        }
    }

    #[test]
    fn duplicate_correct() {
        let grid_pair = "KULPOEIKMANĜUIDPOMAĜI\
                         jlmorpaknbchdftwuyexv"
            .parse::<GridPair>().unwrap();

        let row = &grid_pair.puzzle.squares[
            WORD_LENGTH * (WORD_LENGTH - 1)..WORD_LENGTH * WORD_LENGTH
        ];

        assert!(matches!(row[0].state, PuzzleSquareState::Correct));
        assert!(matches!(row[1].state, PuzzleSquareState::WrongPosition));
        assert!(matches!(row[2].state, PuzzleSquareState::Wrong));
        assert!(matches!(row[3].state, PuzzleSquareState::Correct));
        assert!(matches!(row[4].state, PuzzleSquareState::WrongPosition));
    }
}
