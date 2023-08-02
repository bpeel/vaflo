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

use wasm_bindgen::prelude::*;
use web_sys::console;
use super::grid;
use grid::{Grid, WORD_LENGTH, PuzzleSquareState};

fn show_error(message: &str) {
    console::log_1(&message.into());

    let Some(window) = web_sys::window()
    else {
        return;
    };

    let Some(document) = window.document()
    else {
        return;
    };

    let Some(message_elem) = document.get_element_by_id("message")
    else {
        return;
    };

    message_elem.set_text_content(Some("Eraro okazis"));
}

struct Context {
    document: web_sys::Document,
    window: web_sys::Window,
    message: web_sys::HtmlElement,
}

impl Context {
    fn new() -> Result<Context, String> {
        let Some(window) = web_sys::window()
        else {
            return Err("failed to get window".to_string());
        };

        let Some(document) = window.document()
        else {
            return Err("failed to get document".to_string());
        };

        let Some(message) = document.get_element_by_id("message")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get message div".to_string());
        };

        Ok(Context {
            document,
            window,
            message,
        })
    }
}

type PromiseClosure = Closure::<dyn FnMut(JsValue)>;

struct Loader {
    context: Context,

    data_response_closure: Option<PromiseClosure>,
    data_content_closure: Option<PromiseClosure>,
    data_error_closure: Option<PromiseClosure>,

    floating_pointer: Option<*mut Loader>,
}

impl Loader {
    fn new(context: Context) -> Loader {
        Loader {
            context,
            data_response_closure: None,
            data_content_closure: None,
            data_error_closure: None,
            floating_pointer: None,
        }
    }

    fn start_floating(self) -> *mut Loader {
        assert!(self.floating_pointer.is_none());

        let floating_pointer = Box::into_raw(Box::new(self));

        unsafe {
            (*floating_pointer).floating_pointer = Some(floating_pointer);
        }

        floating_pointer
    }

    fn stop_floating(&mut self) -> Loader {
        match self.floating_pointer {
            Some(floating_pointer) => unsafe {
                // This should end up destroying the loader and
                // invalidating any closures that it holds
                *Box::from_raw(floating_pointer)
            },
            None => unreachable!(),
        }
    }

    fn queue_data_load(&mut self) {
        let filename = "puzzles.txt";

        let floating_pointer = self.floating_pointer.unwrap();

        let response_closure = PromiseClosure::new(move |v: JsValue| {
            let (content_closure, error_closure) = unsafe {
                (
                    (*floating_pointer).data_content_closure.as_ref().unwrap(),
                    (*floating_pointer).data_error_closure.as_ref().unwrap(),
                )
            };

            let response: web_sys::Response = v.dyn_into().unwrap();
            let promise = match response.array_buffer() {
                Ok(p) => p,
                Err(_) => {
                    show_error("Error fetching array buffer from data");
                    unsafe {
                        (*floating_pointer).stop_floating();
                    }
                    return;
                },
            };
            let _ = promise.then2(content_closure, error_closure);
        });

        let content_closure = PromiseClosure::new(move |v| {
            let data = js_sys::Uint8Array::new(&v).to_vec();

            unsafe {
                (*floating_pointer).data_loaded(data);
            }
        });

        let error_closure = PromiseClosure::new(move |_| {
            show_error("Error loading data");
            unsafe {
                (*floating_pointer).stop_floating();
            }
        });

        let promise = self.context.window.fetch_with_str(filename);

        let _ = promise.then2(&response_closure, &error_closure);

        self.data_response_closure = Some(response_closure);
        self.data_content_closure = Some(content_closure);
        self.data_error_closure = Some(error_closure);
    }

    fn parse_puzzles(&mut self, data: Vec<u8>) -> Result<Vec<Grid>, ()> {
        let Ok(data) = std::str::from_utf8(&data)
        else {
            show_error("Puzzle data contains invalid UTF-8");
            return Err(());
        };

        let mut puzzles = Vec::new();

        for (line_num, line) in data.lines().enumerate() {
            match line.parse::<Grid>() {
                Ok(puzzle) => puzzles.push(puzzle),
                Err(e) => {
                    show_error(&format!(
                        "puzzles.txt: line {}: {}",
                        line_num + 1,
                        e,
                    ));
                    return Err(());
                },
            }
        }

        if puzzles.is_empty() {
            show_error("puzzles.txt is empty");
            return Err(());
        }

        Ok(puzzles)
    }

    fn data_loaded(&mut self, data: Vec<u8>) {
        match self.parse_puzzles(data) {
            Err(_) => {
                self.stop_floating();
            },
            Ok(puzzles) => self.start_game(puzzles),
        }
    }

    fn start_game(&mut self, puzzles: Vec<Grid>) {
        let Loader { context, .. } = self.stop_floating();

        let vaflo = Vaflo::new(context, puzzles);
        // Leak the main vaflo object so that it will live as
        // long as the web page
        std::mem::forget(vaflo);
    }
}

struct Vaflo {
    context: Context,
    puzzles: Vec<Grid>,
    game_grid: web_sys::HtmlElement,
    letters: Vec<web_sys::HtmlElement>,
    grid: Grid,
}

impl Vaflo {
    fn new(context: Context, puzzles: Vec<Grid>) -> Result<Box<Vaflo>, String> {
        let Some(game_grid) = context.document.get_element_by_id("game-grid")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get game grid".to_string());
        };

        let mut letters = Vec::with_capacity(WORD_LENGTH * WORD_LENGTH);

        for position in 0..WORD_LENGTH * WORD_LENGTH {
            let Some(element) = context.document.create_element("div").ok()
                .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
            else {
                return Err("failed to create letter element".to_string());
            };

            let _ = element.set_attribute(
                "class",
                if grid::is_gap_position(position) {
                    "gap"
                } else {
                    "letter"
                }
            );

            let style = element.style();

            let _ = style.set_property(
                "grid-area",
                &format!(
                    "{} / {} / {} / {}",
                    position / WORD_LENGTH + 1,
                    position % WORD_LENGTH + 1,
                    position / WORD_LENGTH + 2,
                    position % WORD_LENGTH + 2,
                ),
            );

            let _ = game_grid.append_with_node_1(&element);

            letters.push(element);
        }

        let _ = context.message.style().set_property("display", "none");
        let _ = game_grid.style().set_property("display", "grid");

        let grid = puzzles[0].clone();

        let vaflo = Vaflo {
            context,
            puzzles,
            game_grid,
            letters,
            grid,
        };

        vaflo.update_square_letters();
        vaflo.update_square_states();

        Ok(Box::new(vaflo))
    }

    fn update_square_letters(&self) {
        for (position, letter) in self
            .grid
            .solution
            .letters
            .iter()
            .enumerate()
        {
            let element = &self.letters[position];

            while let Some(child) = element.first_child() {
                let _ = element.remove_child(&child);
            }

            let mut letter_text = [0u8; 4];

            let text = self.context.document.create_text_node(
                letter.encode_utf8(&mut letter_text)
            );
            let _ = element.append_with_node_1(&text);
        }
    }

    fn update_square_states(&self) {
        for (position, square) in self
            .grid
            .puzzle
            .squares
            .iter()
            .enumerate()
        {
            if grid::is_gap_position(position) {
                continue;
            }

            let element = &self.letters[position];

            let _ = element.set_attribute(
                "class",
                match square.state {
                    PuzzleSquareState::Correct => "letter correct",
                    PuzzleSquareState::WrongPosition => "letter wrong-position",
                    PuzzleSquareState::Wrong => "letter wrong",
                }
            );
        }
    }
}

#[wasm_bindgen]
pub fn init_vaflo() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let context = match Context::new() {
        Ok(c) => c,
        Err(e) => {
            show_error(&e);
            return;
        }
    };

    let loader = Loader::new(context);

    let floating_pointer = loader.start_floating();

    unsafe {
        (*floating_pointer).queue_data_load();
    }
}
