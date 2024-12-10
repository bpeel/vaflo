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

mod letter_grid;
mod dictionary;
mod word_grid;
mod word_solver;
mod grid_solver;
mod permute;
mod pairs;
mod swap_solver;
mod grid;
mod word_counter;
mod stem_word;
mod solver_state;
mod crossword_solver;
mod word_search;

use std::process::ExitCode;
use letter_grid::LetterGrid;
use grid::{WORD_LENGTH, N_LETTERS, N_WORDS};
use dictionary::Dictionary;
use std::ffi::c_int;
use std::sync::{Arc, mpsc};
use std::thread;
use word_grid::WordGrid;
use grid_solver::GridSolver;
use std::io::{BufRead, Write};
use rand::Rng;
use rand::seq::SliceRandom;
use grid::{Grid, SolutionGrid, PuzzleGrid, PuzzleSquareState};
use word_counter::WordCounter;
use solver_state::{SolverState, SolverStatePair};
use chrono::{naive::Days, NaiveDate};

// Number of swaps to make when shuffling the puzzle
const N_SHUFFLE_SWAPS: usize = 10;

const WRONG_LETTER_COLOR: i16 = 1;
const FIRST_STATE_COLOR: i16 = 2;

enum EditDirection {
    Right,
    Down,
}

enum GridChoice {
    Solution,
    Puzzle,
}

#[derive(Default, Eq, PartialEq)]
enum WordState {
    #[default]
    Invalid,
    Duplicate,
    Valid,
}

#[derive(Default)]
struct Word {
    state: WordState,
    text: String,
}

enum SearchResults {
    None,
    Crosswords(Vec<crossword_solver::Crossword>),
    Words(Vec<String>),
}

struct Editor {
    dictionary: Arc<Dictionary>,
    solver_state: Arc<SolverStatePair>,
    should_quit: bool,
    grid_x: i32,
    grid_y: i32,
    current_puzzle: usize,
    puzzles: Vec<Grid>,
    cursor_x: i32,
    cursor_y: i32,
    edit_direction: EditDirection,
    current_grid: GridChoice,
    words: [Word; N_WORDS],
    selected_position: Option<usize>,
    grid_id: usize,
    solutions: Vec<WordGrid>,
    had_all_solutions: bool,
    shortest_swap_solution: Option<usize>,
    word_counter: WordCounter,
    search_results: SearchResults,
    // Number of puzzles when the data was loaded
    initial_n_puzzles: usize,
}

enum SolutionEventKind {
    Grid(WordGrid),
    GridEnd,
    SwapSolution(usize),
}

struct SolutionEvent {
    id: usize,
    kind: SolutionEventKind,
}

struct SolverThread {
    word_join_handle: thread::JoinHandle<()>,
    swap_join_handle: thread::JoinHandle<()>,
    solver_state: Arc<SolverStatePair>,
    event_receiver: mpsc::Receiver<SolutionEvent>,
}

fn addch_utf8(ch: char) {
    let mut buf = [0u8; 4];

    ncurses::addstr(ch.encode_utf8(&mut buf));
}

fn draw_solution_grid(grid: &SolutionGrid, grid_x: i32, grid_y: i32) {
    for y in 0..WORD_LENGTH {
        ncurses::mv(grid_y + y as i32, grid_x);

        for x in 0..WORD_LENGTH {
            if grid::is_gap_space(x as i32, y as i32) {
                ncurses::addch(' ' as u32);
            } else {
                addch_utf8(grid.letters[y * WORD_LENGTH + x]);
            }
        }
    }
}

fn shuffle_grid(grid: &mut PuzzleGrid) {
    grid.reset();

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
                !grid::is_gap_position(pos)
                    && used_squares & (1 << pos) == 0
            });

        assert_eq!(n_positions, positions.clone().count());

        let a_pos = positions.nth(a).unwrap();
        let b_pos = positions.nth(b - a - 1).unwrap();

        grid.squares.swap(a_pos, b_pos);

        used_squares |= (1 << a_pos) | (1 << b_pos)
    }
}

fn draw_puzzle_grid(
    grid: &PuzzleGrid,
    grid_x: i32,
    grid_y: i32,
    solution: &SolutionGrid,
    selected_position: Option<usize>,
) {
    for y in 0..WORD_LENGTH {
        ncurses::mv(grid_y + y as i32, grid_x);

        for x in 0..WORD_LENGTH {
            if grid::is_gap_space(x as i32, y as i32) {
                ncurses::addch(' ' as u32);
            } else {
                let square = grid.squares[y * WORD_LENGTH + x];
                let is_selected = selected_position
                    .map(|p| p == y * WORD_LENGTH + x)
                    .unwrap_or(false);

                if is_selected {
                    ncurses::attron(ncurses::A_BOLD());
                }

                let color = ncurses::COLOR_PAIR(color_for_state(square.state));

                ncurses::attron(color);

                addch_utf8(solution.letters[square.position]);

                ncurses::attroff(color);

                if is_selected {
                    ncurses::attroff(ncurses::A_BOLD());
                }
            }
        }
    }
}

fn puzzle_x() -> i32 {
    WORD_LENGTH.max(9) as i32 + 2
}

fn draw_grid(
    grid: &Grid,
    grid_x: i32,
    grid_y: i32,
    selected_position: Option<usize>,
) {
    ncurses::mvaddstr(grid_y, grid_x, "Solution:");
    draw_solution_grid(&grid.solution, grid_x, grid_y + 1);

    let grid_x = grid_x + puzzle_x();
    ncurses::mvaddstr(grid_y, grid_x, "Puzzle:");
    draw_puzzle_grid(
        &grid.puzzle,
        grid_x, grid_y + 1,
        &grid.solution,
        selected_position,
    );
}

fn minimum_swaps<F>(
    grid: &Grid,
    should_cancel: F,
) -> Option<usize>
where
    F: FnMut() -> bool,
{
    let puzzle = grid.puzzle
        .squares
        .iter()
        .map(|square| grid.solution.letters[square.position])
        .collect::<Vec<char>>();

    swap_solver::solve_cancellable(
        &puzzle,
        &grid.solution.letters,
        should_cancel
    ).map(|solution| solution.len())
}

#[inline(always)]
fn color_for_state(state: PuzzleSquareState) -> i16 {
    state as i16 + FIRST_STATE_COLOR
}

fn date_string_for_puzzle(puzzle_num: usize) -> String {
    let start_date = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();

    match start_date.checked_add_days(Days::new(puzzle_num as u64)) {
        None => "?".to_string(),
        Some(puzzle_date) => puzzle_date.format("%a %Y-%m-%d").to_string(),
    }
}

impl Editor {
    fn new(
        puzzles: Vec<Grid>,
        dictionary: Arc<Dictionary>,
        solver_state: Arc<SolverStatePair>,
        grid_x: i32,
        grid_y: i32,
    ) -> Editor {
        assert!(!puzzles.is_empty());

        let initial_n_puzzles = puzzles.len();

        let mut editor = Editor {
            dictionary,
            solver_state,
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
            had_all_solutions: false,
            shortest_swap_solution: None,
            word_counter: WordCounter::new(),
            search_results: SearchResults::None,
            initial_n_puzzles,
        };

        editor.update_words();
        editor.update_word_counts();
        editor.send_grid();

        editor
    }

    fn redraw(&self) {
        ncurses::clear();
        let grid = &self.puzzles[self.current_puzzle];

        draw_grid(
            grid,
            self.grid_x,
            self.grid_y,
            self.selected_position
        );

        let direction_ch = match self.edit_direction {
            EditDirection::Right => '>',
            EditDirection::Down => 'v',
        };

        let right_side = self.grid_x
            + puzzle_x()
            + WORD_LENGTH as i32
            + 5;

        ncurses::mvaddch(self.grid_y, right_side, direction_ch as u32);

        ncurses::addstr(&format!(
            " {}/{} {}",
            self.current_puzzle + 1,
            self.puzzles.len(),
            date_string_for_puzzle(self.current_puzzle),
        ));

        if self.current_puzzle >= self.initial_n_puzzles {
            ncurses::addstr(&format!(
                " +{}",
                self.current_puzzle - self.initial_n_puzzles + 1,
            ));
        }

        self.draw_words(right_side, self.grid_y + 2);
        self.draw_search_results(
            right_side,
            self.grid_y + 2 + N_WORDS as i32 + 2,
        );

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
            ncurses::mvaddstr(y, self.grid_x, "Solutions");

            if !self.had_all_solutions {
                ncurses::addstr("…");
            }

            ncurses::addch(':' as u32);

            y += 2;

            let max_y = ncurses::getmaxy(ncurses::stdscr());

            let wrong_letter_color = ncurses::COLOR_PAIR(WRONG_LETTER_COLOR);

            for solution in self.solutions.iter() {
                if y + WORD_LENGTH as i32 > max_y {
                    break;
                }

                let mut position = 0;

                for line in solution.to_string().lines() {
                    if line.is_empty() {
                        break;
                    }

                    ncurses::mv(y, self.grid_x);

                    for ch in line.chars() {
                        let mut letter = [0u8; 4];
                        let letter = ch.encode_utf8(&mut letter);

                        if ch != grid.solution.letters[position] {
                            ncurses::attron(wrong_letter_color);
                            ncurses::attron(ncurses::A_BOLD());
                            ncurses::addstr(letter);
                            ncurses::attroff(ncurses::A_BOLD());
                            ncurses::attroff(wrong_letter_color);
                        } else {
                            ncurses::addstr(letter);
                        }

                        position += 1;
                    }

                    y += 1;
                }

                y += 1;
            }
        }

        self.position_cursor();

        ncurses::refresh();
    }

    fn draw_words(&self, x: i32, y: i32) {
        let wrong_letter_color = ncurses::COLOR_PAIR(WRONG_LETTER_COLOR);

        ncurses::mvaddstr(y, x, "Words:");

        for (i, word) in self.words.iter().enumerate() {
            ncurses::mvaddstr(
                y + 1 + i as i32,
                x,
                &word.text,
            );
            ncurses::addch(' ' as u32);
            ncurses::addstr(
                match word.state {
                    WordState::Valid => "✅",
                    WordState::Duplicate => "♻️",
                    WordState::Invalid => "❌",
                }
            );

            for (word, count, last_use)
                in self.word_counter.counts(&word.text)
            {
                ncurses::addstr(&format!(" {}({},", word, count));

                let too_new = self.current_puzzle - last_use < 30;

                if too_new {
                    ncurses::attron(wrong_letter_color);
                }

                ncurses::addstr(&format!("#{}", last_use + 1));

                if too_new {
                    ncurses::attroff(wrong_letter_color);
                }

                ncurses::addch(')' as u32);
            }
        }
    }

    fn draw_search_results(&self, x: i32, y: i32) {
        match self.search_results {
            SearchResults::None => (),
            SearchResults::Crosswords(ref crosswords) => {
                self.draw_crosswords(crosswords, x, y);
            },
            SearchResults::Words(ref words) => {
                self.draw_words_results(words, x, y);
            },
        }
    }

    fn draw_search_words<T: AsRef<str>>(
        &self,
        start_x: i32,
        start_y: i32,
        words: &[T],
    ) -> i32 {
        let max_x = ncurses::getmaxx(ncurses::stdscr());
        let max_y = ncurses::getmaxy(ncurses::stdscr());

        let mut x = start_x;
        let mut y = start_y;

        ncurses::mv(start_y, start_x);

        for word in words.iter() {
            if x + WORD_LENGTH as i32 + 1 > max_x {
                x = start_x;
                y += 1;

                if y >= max_y {
                    break;
                }

                ncurses::mv(y, x);
            }
            ncurses::addch(' ' as u32);
            ncurses::addstr(word.as_ref());
            x += WORD_LENGTH as i32 + 1;
        }

        y - start_y + 1
    }

    fn draw_crosswords(
        &self,
        crosswords: &Vec<crossword_solver::Crossword>,
        start_x: i32,
        mut y: i32,
    ) {
        let max_y = ncurses::getmaxy(ncurses::stdscr());

        ncurses::mvaddstr(y, start_x, "Crosswords:");
        y += 2;

        for crossword in crosswords.iter() {
            if y >= max_y {
                break;
            }

            ncurses::mv(y, start_x);
            addch_utf8(crossword.cross_letter);
            ncurses::addch(':' as u32);

            y += self.draw_search_words(start_x + 2, y, &crossword.a_words);
            y += self.draw_search_words(start_x + 2, y, &crossword.b_words);
        }
    }

    fn draw_words_results<T: AsRef<str>>(
        &self,
        words: &[T],
        x: i32,
        y: i32,
    ) {
        ncurses::mvaddstr(y, x, "Search results:");
        self.draw_search_words(x - 1, y + 2, words);
    }

    fn position_cursor(&self) {
        let x = match self.current_grid {
            GridChoice::Solution => 0,
            GridChoice::Puzzle => puzzle_x(),
        };

        ncurses::mv(
            self.grid_y + 1 + self.cursor_y,
            self.grid_x + x + self.cursor_x,
        );
    }

    fn move_cursor(&mut self, x_offset: i32, y_offset: i32) {
        let mut x = self.cursor_x + x_offset;
        let mut y = self.cursor_y + y_offset;

        if grid::is_gap_space(x, y) {
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

        let grid = &mut self.puzzles[self.current_puzzle];

        let position = match self.current_grid {
            GridChoice::Solution => position,
            GridChoice::Puzzle => {
                grid.puzzle.squares[position].position
            },
        };

        grid.solution.letters[position] = ch;
        grid.update_square_states();
        self.update_words();
        self.send_grid();

        match self.edit_direction {
            EditDirection::Down => {
                if self.cursor_y + 1 < WORD_LENGTH as i32 {
                    if grid::is_gap_space(self.cursor_x, self.cursor_y + 1) {
                        self.edit_direction = EditDirection::Right;
                        self.cursor_x += 1;
                    } else {
                        self.cursor_y += 1;
                    }
                }
            },
            EditDirection::Right => {
                if self.cursor_x + 1 < WORD_LENGTH as i32 {
                    if grid::is_gap_space(self.cursor_x + 1, self.cursor_y) {
                        self.edit_direction = EditDirection::Down;
                        self.cursor_y += 1;
                    } else {
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
            ncurses::KEY_HOME => self.set_current_puzzle(0),
            ncurses::KEY_END => self.set_current_puzzle(self.puzzles.len() - 1),
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
                let grid = &mut self.puzzles[self.current_puzzle];
                grid.puzzle.squares.swap(pos, cursor_pos);
                grid.update_square_states();
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
                '$' => self.toggle_edit_direction(),
                ' ' => self.handle_mark(),
                '\u{0003}' => self.should_quit = true, // Ctrl+C
                '\u{0010}' => self.pattern_search(), // Ctrl+P
                '\u{0012}' => self.shuffle_puzzle(), // Ctrl+R
                '\u{0013}' => self.handle_swap(), // Ctrl+S
                '\u{000a}' => self.shuffle_search_results(), // Ctrl+J
                '\u{000e}' => self.new_puzzle(), // Ctrl+N
                '\u{0018}' => self.find_crosswords(), // Ctrl+X
                ch if ch.is_alphabetic() || ch == '.' => {
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
        let grid = &self.puzzles[self.current_puzzle];

        for (word_num, positions) in grid::WordPositions::new().enumerate() {
            let word = &mut self.words[word_num];
            word.text.clear();
            word.text.extend(positions.map(|pos| grid.solution.letters[pos]));

            let state = 'find_duplicate: {
                let word = &self.words[word_num];
                let stem = stem_word::stem(&word.text);

                for other_word in &self.words[0..word_num] {
                    if stem == stem_word::stem(&other_word.text) {
                        break 'find_duplicate WordState::Duplicate;
                    }
                }

                if self.dictionary.contains(word.text.chars()) {
                    WordState::Valid
                } else {
                    WordState::Invalid
                }
            };

            self.words[word_num].state = state;
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
            SolutionEventKind::GridEnd => {
                self.had_all_solutions = true;
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
        self.had_all_solutions = false;
        self.shortest_swap_solution = None;

        let grid = self.puzzles[self.current_puzzle].clone();

        self.solver_state.set_grid(self.grid_id, grid);
    }

    fn set_current_puzzle(&mut self, puzzle_num: usize) {
        if puzzle_num != self.current_puzzle {
            assert!(puzzle_num < self.puzzles.len());
            self.current_puzzle = puzzle_num;
            self.update_words();
            self.update_word_counts();
            self.search_results = SearchResults::None;
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
        let mut grid = Grid::new();
        let letters = &mut grid.solution.letters;

        // Initialise all of the letters with the ‘.’ search pattern
        // placeholder to make it easier to search for words.
        for (i, letter) in letters.iter_mut().enumerate() {
            if !grid::is_gap_position(i) {
                *letter = '.';
            }
        }

        self.cursor_x = 0;
        self.cursor_y = 0;
        self.current_grid = GridChoice::Solution;
        self.edit_direction = EditDirection::Right;

        self.puzzles.push(grid);
        self.set_current_puzzle(self.puzzles.len() - 1);
    }

    fn shuffle_puzzle(&mut self) {
        let grid = &mut self.puzzles[self.current_puzzle];
        shuffle_grid(&mut grid.puzzle);
        grid.update_square_states();
        self.send_grid();
        self.redraw();
    }

    fn find_crosswords(&mut self) {
        let crosswords = crossword_solver::find_crosswords(
            &self.puzzles[self.current_puzzle].solution,
            self.cursor_x,
            self.cursor_y,
            &self.dictionary,
        );

        self.search_results = SearchResults::Crosswords(crosswords);

        self.redraw();
    }

    fn pattern_search(&mut self) {
        let solution = &self.puzzles[self.current_puzzle].solution;

        let pattern = if self.cursor_y & 1 == 0 {
            solution.letters[
                self.cursor_y as usize
                    * WORD_LENGTH
                    ..(self.cursor_y as usize + 1) * WORD_LENGTH
            ].into_iter().collect::<String>()
        } else {
            (0..WORD_LENGTH)
                .map(|y| {
                    let pos = y * WORD_LENGTH + self.cursor_x as usize;
                    solution.letters[pos]
                })
                .collect::<String>()
        };

        let words = word_search::search(&pattern, &self.dictionary);

        self.search_results = SearchResults::Words(words);

        self.redraw();
    }

    fn shuffle_search_results(&mut self) {
        let mut rng = rand::thread_rng();

        match self.search_results {
            SearchResults::None => (),
            SearchResults::Crosswords(ref mut crosswords) => {
                for crossword in crosswords.iter_mut() {
                    crossword.a_words.shuffle(&mut rng);
                    crossword.b_words.shuffle(&mut rng);
                }

                crosswords.shuffle(&mut rng);
            },
            SearchResults::Words(ref mut words) => {
                words.shuffle(&mut rng);
            },
        }

        self.redraw()
    }

    fn update_word_counts(&mut self) {
        self.word_counter.clear();

        for (puzzle_num, puzzle) in self.puzzles.iter().enumerate() {
            if puzzle_num == self.current_puzzle {
                continue;
            }

            for positions in grid::WordPositions::new() {
                let word = positions.map(|pos| puzzle.solution.letters[pos]);
                self.word_counter.push(word, puzzle_num);
            }
        }
    }
}

fn load_dictionary() -> Result<Arc<Dictionary>, ()> {
    let filename = std::env::args_os()
        .nth(1)
        .unwrap_or("data/dictionary.bin".into());

    match std::fs::read(&filename) {
        Err(e) => {
            eprintln!(
                "{}: {}",
                filename.to_string_lossy(),
                e,
            );
            Err(())
        },
        Ok(d) => Ok(Arc::new(Dictionary::new(d.into_boxed_slice()))),
    }
}

fn load_puzzles() -> Result<Vec<Grid>, ()> {
    let filename = "puzzles.txt";
    let mut puzzles = Vec::new();

    let f = match std::fs::File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(vec![Grid::new()]);
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

        match line.parse::<Grid>() {
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

fn save_puzzles(puzzles: &[Grid]) {
    let f = match std::fs::File::create("puzzles.txt.tmp") {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut writer = std::io::BufWriter::new(f);

    for puzzle in puzzles.iter() {
        if writeln!(writer, "{}", puzzle).is_err() {
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
            if std::io::Error::last_os_error().kind()
                == std::io::ErrorKind::Interrupted
            {
                continue;
            }

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

struct EventSender {
    sender: mpsc::Sender<SolutionEvent>,
    wakeup_fd: c_int,
}

impl EventSender {
    fn new(
        sender: mpsc::Sender<SolutionEvent>,
        wakeup_fd: c_int,
    ) -> EventSender {
        EventSender {
            sender,
            wakeup_fd,
        }
    }

    fn send(
        &self,
        event: SolutionEvent,
    ) -> Result<(), mpsc::SendError<SolutionEvent>> {
        self.sender.send(event)?;

        let wakeup_bytes = [b'!'];

        unsafe {
            libc::write(self.wakeup_fd, wakeup_bytes.as_ptr().cast(), 1);
        }

        Ok(())
    }
}

impl SolverThread {
    fn new(
        dictionary: Arc<Dictionary>,
        wakeup_fd: c_int,
    ) -> SolverThread {
        let (event_sender, event_receiver) = mpsc::channel();

        let word_event_sender = EventSender::new(
            event_sender.clone(),
            wakeup_fd,
        );
        let swap_event_sender = EventSender::new(event_sender, wakeup_fd);

        let solver_state = Arc::new(SolverStatePair::new());
        let word_solver_state = Arc::clone(&solver_state);
        let swap_solver_state = Arc::clone(&solver_state);

        let word_join_handle = thread::spawn(move || {
            let mut completed_grid_id = None;

            'thread_loop: loop {
                let (grid_id, grid) = match word_solver_state.wait(
                    completed_grid_id
                ) {
                    SolverState::Idle => unreachable!(),
                    SolverState::Task { grid_id, grid } => (grid_id, grid),
                    SolverState::Quit => break 'thread_loop,
                };

                completed_grid_id = Some(grid_id);

                let Ok(grid) = LetterGrid::from_grid(&grid)
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
                        break 'thread_loop;
                    }
                }

                if word_event_sender.send(SolutionEvent::new(
                    grid_id,
                    SolutionEventKind::GridEnd,
                )).is_err() {
                    break;
                }
            }
        });

        let swap_join_handle = thread::spawn(move || {
            let mut completed_grid_id = None;

            'thread_loop: loop {
                let (grid_id, grid) = match swap_solver_state.wait(
                    completed_grid_id
                ) {
                    SolverState::Idle => unreachable!(),
                    SolverState::Task { grid_id, grid } => (grid_id, grid),
                    SolverState::Quit => break 'thread_loop,
                };

                completed_grid_id = Some(grid_id);

                let should_cancel = || {
                    swap_solver_state.later_task_is_pending(completed_grid_id)
                };

                if let Some(n_swaps) = minimum_swaps(&grid, should_cancel) {
                    let event = SolutionEvent::new(
                        grid_id,
                        SolutionEventKind::SwapSolution(n_swaps),
                    );
                    if swap_event_sender.send(event).is_err() {
                        break;
                    }
                }
            }
        });

        SolverThread {
            word_join_handle,
            swap_join_handle,
            solver_state,
            event_receiver,
        }
    }

    fn join(self) {
        let SolverThread {
            word_join_handle,
            swap_join_handle,
            event_receiver,
            solver_state,
        } = self;

        solver_state.quit();

        // Drop the mpsc so that the thread will quit if it tries to send
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
        WRONG_LETTER_COLOR,
        ncurses::COLOR_RED,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        color_for_state(PuzzleSquareState::Correct),
        ncurses::COLOR_GREEN,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        color_for_state(PuzzleSquareState::WrongPosition),
        ncurses::COLOR_YELLOW,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        color_for_state(PuzzleSquareState::Wrong),
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
        Arc::clone(&solver_thread.solver_state),
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
