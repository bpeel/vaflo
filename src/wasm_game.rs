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
use std::fmt::Write;

const STOP_ANIMATIONS_DELAY: i32 = 250;
const MAXIMUM_SWAPS: u32 = 15;
const N_STARS: u32 = 5;

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

struct Drag {
    position: usize,
    start_x: i32,
    start_y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Playing,
    Won,
    Lost,
}

struct Vaflo {
    context: Context,
    pointerdown_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointerup_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointermove_closure: Option<Closure::<dyn Fn(JsValue)>>,
    pointercancel_closure: Option<Closure::<dyn Fn(JsValue)>>,
    puzzles: Vec<Grid>,
    game_contents: web_sys::HtmlElement,
    game_grid: web_sys::HtmlElement,
    letters: Vec<web_sys::HtmlElement>,
    swaps_remaining_message: web_sys::HtmlElement,
    game_state: GameState,
    grid: Grid,
    drag: Option<Drag>,
    stop_animations_closure: Option<Closure::<dyn Fn()>>,
    stop_animations_queued: bool,
    animated_letters: Vec<usize>,
    swaps_remaining: u32,
}

impl Vaflo {
    fn new(context: Context, puzzles: Vec<Grid>) -> Result<Box<Vaflo>, String> {
        let Some(game_contents) =
            context.document.get_element_by_id("game-contents")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get game contents".to_string());
        };

        let Some(game_grid) = context.document.get_element_by_id("game-grid")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get game grid".to_string());
        };

        let Some(swaps_remaining_message) =
            context.document.get_element_by_id("swaps-remaining")
            .and_then(|c| c.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            return Err("failed to get swaps remaining message".to_string());
        };

        let grid = puzzles[0].clone();

        let mut vaflo = Box::new(Vaflo {
            context,
            pointerdown_closure: None,
            pointerup_closure: None,
            pointermove_closure: None,
            pointercancel_closure: None,
            puzzles,
            game_contents,
            game_grid,
            swaps_remaining_message,
            letters: Vec::with_capacity(WORD_LENGTH * WORD_LENGTH),
            game_state: GameState::Playing,
            grid,
            drag: None,
            stop_animations_closure: None,
            stop_animations_queued: false,
            animated_letters: Vec::new(),
            swaps_remaining: MAXIMUM_SWAPS,
        });

        vaflo.create_closures();
        vaflo.create_letters()?;
        vaflo.update_game_state();
        vaflo.update_square_letters();
        vaflo.update_square_states();
        vaflo.update_swaps_remaining();
        vaflo.show_game_contents();

        Ok(vaflo)
    }

    fn create_closures(&mut self) {
        let vaflo_pointer = self as *mut Vaflo;

        let pointerdown_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let vaflo = unsafe { &mut *vaflo_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                vaflo.handle_pointerdown_event(event);
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "pointerdown",
            pointerdown_closure.as_ref().unchecked_ref(),
        );

        self.pointerdown_closure = Some(pointerdown_closure);

        let pointerup_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let vaflo = unsafe { &mut *vaflo_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                vaflo.handle_pointerup_event(event);
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "pointerup",
            pointerup_closure.as_ref().unchecked_ref(),
        );

        self.pointerup_closure = Some(pointerup_closure);

        let pointermove_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let vaflo = unsafe { &mut *vaflo_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                vaflo.handle_pointermove_event(event);
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "pointermove",
            pointermove_closure.as_ref().unchecked_ref(),
        );

        self.pointermove_closure = Some(pointermove_closure);

        let pointercancel_closure = Closure::<dyn Fn(JsValue)>::new(
            move |event: JsValue| {
                let vaflo = unsafe { &mut *vaflo_pointer };
                let event: web_sys::PointerEvent = event.dyn_into().unwrap();
                vaflo.handle_pointercancel_event(event);
            }
        );

        let _ = self.context.document.add_event_listener_with_callback(
            "pointercancel",
            pointercancel_closure.as_ref().unchecked_ref(),
        );

        self.pointercancel_closure = Some(pointercancel_closure);
    }

    fn create_letters(&mut self) -> Result<(), String> {
        let letters = &mut self.letters;

        for position in 0..WORD_LENGTH * WORD_LENGTH {
            let Some(element) = self.context.document.create_element("div").ok()
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

            let _ = self.game_grid.append_with_node_1(&element);

            letters.push(element);
        }

        Ok(())
    }

    fn show_game_contents(&self) {
        let _ = self.context.message.style().set_property("display", "none");
        let _ = self.game_contents.style().set_property("display", "block");
    }

    fn position_for_event(&self, event: &web_sys::Event) -> Option<usize> {
        let Some(target) = event.target()
        else {
            return None;
        };

        let Ok(element) = target.dyn_into::<web_sys::HtmlElement>()
        else {
            return None;
        };

        for (position, letter_element) in self.letters.iter().enumerate() {
            if grid::is_gap_position(position) {
                continue;
            }

            if letter_element == &element {
                return Some(position);
            }
        }

        None
    }

    fn set_letter_translation(&self, position: usize, x: f64, y: f64) {
        let translation = format!("translate({}px, {}px)", x, y);
        let style = self.letters[position].style();
        let _ = style.set_property("transform", &translation);
    }

    fn update_drag_position(&self, event: &web_sys::PointerEvent) {
        let drag = self.drag.as_ref().unwrap();

        self.set_letter_translation(
            drag.position,
            (event.client_x() - drag.start_x) as f64,
            (event.client_y() - drag.start_y) as f64,
        );
    }

    fn find_letter_for_position(
        &self,
        skip: usize,
        x: f64,
        y: f64,
    ) -> Option<usize> {
        for position in 0..WORD_LENGTH * WORD_LENGTH {
            if skip == position || grid::is_gap_position(position) {
                continue;
            }

            let client_rect = self.letters[position].get_bounding_client_rect();
            let client_x = client_rect.x();
            let client_y = client_rect.y();

            if x >= client_x
                && y >= client_y
                && x < client_x + client_rect.width()
                && y < client_y + client_rect.height()
            {
                return Some(position);
            }
        }

        None
    }

    fn stop_animations(&mut self) {
        for &position in self.animated_letters.iter() {
            let style = self.letters[position].style();
            let _ = style.set_property("transform", "none");
            self.set_square_class(position, None);
        }
        self.animated_letters.clear();

        if self.grid.puzzle.is_solved() {
            self.set_won_state();
        } else if self.swaps_remaining == 0 {
            self.set_lost_state();
        }
    }

    fn slide_letter(&mut self, position: usize) {
        self.set_square_class(position, Some("sliding"));
        self.animated_letters.push(position);

        if !self.stop_animations_queued {
            let vaflo_pointer = self as *mut Vaflo;

            let closure = self.stop_animations_closure.get_or_insert_with(|| {
                Closure::<dyn Fn()>::new(move || {
                    let vaflo = unsafe { &mut *vaflo_pointer };
                    vaflo.stop_animations_queued = false;
                    vaflo.stop_animations();
                })
            });

            match self
                .context
                .window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    STOP_ANIMATIONS_DELAY,
                )
            {
                Ok(_) => {
                    self.stop_animations_queued = true;
                },
                Err(_) => {
                    console::log_1(&"Error setting timeout".into());
                },
            }
        }
    }

    fn swap_letters(&mut self, position_a: usize, position_b: usize) {
        self.grid.puzzle.squares.swap(position_a, position_b);
        self.grid.update_square_states();
        self.update_square_states();
        self.update_square_letter(position_a);
        self.update_square_letter(position_b);

        let rect_a = self.letters[position_a].get_bounding_client_rect();
        let rect_b = self.letters[position_b].get_bounding_client_rect();

        // Remove any existing transform so we can set the transform
        // based on the origin
        let a_style = self.letters[position_a].style();
        let _ = a_style.set_property("transform", "none");
        let b_style = self.letters[position_b].style();
        let _ = b_style.set_property("transform", "none");
        let base_rect_a = self.letters[position_a].get_bounding_client_rect();
        let base_rect_b = self.letters[position_b].get_bounding_client_rect();

        self.set_letter_translation(
            position_a,
            rect_b.x() - base_rect_a.x(),
            rect_b.y() - base_rect_a.y(),
        );
        self.set_letter_translation(
            position_b,
            rect_a.x() - base_rect_b.x(),
            rect_a.y() - base_rect_b.y(),
        );

        self.slide_letter(position_a);
        self.slide_letter(position_b);

        self.swaps_remaining = self.swaps_remaining.saturating_sub(1);
        self.update_swaps_remaining();
    }

    fn handle_pointerdown_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary()
            || event.button() != 0
            || self.drag.is_some()
            || self.game_state != GameState::Playing
        {
            return;
        }

        let Some(position) = self.position_for_event(&event)
        else {
            return;
        };

        event.prevent_default();

        if !self.animated_letters.is_empty() {
            return;
        }

        match self.grid.puzzle.squares[position].state {
            PuzzleSquareState::Correct => return,
            PuzzleSquareState::Wrong => (),
            PuzzleSquareState::WrongPosition => (),
        }

        self.set_square_class(position, Some("dragging"));

        self.drag = Some(Drag {
            position,
            start_x: event.client_x(),
            start_y: event.client_y(),
        });

        self.update_drag_position(&event);
    }

    fn handle_pointerup_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() {
            return;
        }

        let Some(drag) = self.drag.take()
        else {
            return;
        };

        event.prevent_default();

        let dragged_element = &self.letters[drag.position];
        let client_rect = dragged_element.get_bounding_client_rect();

        if let Some(target_position) = self.find_letter_for_position(
            drag.position,
            client_rect.x() + client_rect.width() / 2.0,
            client_rect.y() + client_rect.height() / 2.0,
        ).filter(|&target_position| {
            target_position != drag.position
                && self.grid.puzzle.squares[target_position].state
                != PuzzleSquareState::Correct
        }) {
            self.swap_letters(target_position, drag.position);
            // Make sure the dragging letter (which is now the target
            // letter) is on top
            self.move_letter_to_top(target_position);
        } else {
            self.slide_letter(drag.position);
        }
    }

    fn handle_pointermove_event(&mut self, event: web_sys::PointerEvent) {
        if event.is_primary() && self.drag.is_some() {
            event.prevent_default();
            self.update_drag_position(&event);
        }
    }

    fn handle_pointercancel_event(&mut self, event: web_sys::PointerEvent) {
        if !event.is_primary() {
            return;
        }

        let Some(drag) = self.drag.take()
        else {
            return;
        };

        self.slide_letter(drag.position);
    }

    fn set_element_text(&self, element: &web_sys::HtmlElement, text: &str) {
        while let Some(child) = element.first_child() {
            let _ = element.remove_child(&child);
        }

        let text = self.context.document.create_text_node(text);
        let _ = element.append_with_node_1(&text);
    }

    fn update_game_state(&self) {
        let text = match self.game_state {
            GameState::Playing => "playing",
            GameState::Won => "won",
            GameState::Lost => "lost",
        };

        let _ = self.game_grid.set_attribute("class", text);
    }

    fn set_game_state(&mut self, state: GameState) {
        self.game_state = state;
        self.update_game_state();
    }

    fn set_won_state(&mut self) {
        self.set_game_state(GameState::Won);

        let text = match self.swaps_remaining {
            4 => "Bonege!",
            3 => "Tre bone!",
            2 => "Sukceso!",
            1 => "Bone!",
            0 => "Uf! Ĝusteco!",
            _ => "Perfekte!",
        };

        self.set_element_text(&self.swaps_remaining_message, text);

        if let Ok(stars) = self.context.document.create_element("div") {
            let _ = stars.set_attribute("class", "stars");

            for i in 0..N_STARS {
                if let Ok(star) = self.context.document.create_element("span") {
                    let _ = star.set_attribute(
                        "class",
                        if i + 1 <= self.swaps_remaining {
                            "filled"
                        } else {
                            "empty"
                        },
                    );

                    let _ = stars.append_with_node_1(&star);
                }
            }

            let _ = self.swaps_remaining_message.append_with_node_1(&stars);
        }
    }

    fn set_lost_state(&mut self) {
        self.set_game_state(GameState::Lost);

        self.set_element_text(&self.swaps_remaining_message, "Malsukcesis 😔");
    }

    fn update_square_letter(&self, position: usize) {
        let element = &self.letters[position];

        let mut letter_text = [0u8; 4];
        let letter_index = self.grid.puzzle.squares[position].position;
        let letter = self.grid.solution.letters[letter_index];
        self.set_element_text(element, letter.encode_utf8(&mut letter_text));
    }

    fn update_square_letters(&self) {
        for position in 0..WORD_LENGTH * WORD_LENGTH {
            self.update_square_letter(position);
        }
    }

    fn set_square_class(&self, position: usize, extra: Option<&str>) {
        let element = &self.letters[position];

        let square = &self.grid.puzzle.squares[position];

        let class = match square.state {
            PuzzleSquareState::Correct => "letter correct",
            PuzzleSquareState::WrongPosition => "letter wrong-position",
            PuzzleSquareState::Wrong => "letter wrong",
        };

        let mut class_string = class.to_string();

        if let Some(extra) = extra {
            class_string.push(' ');
            class_string.push_str(extra);
        };
        write!(class_string, " col{}", position % WORD_LENGTH).unwrap();

        let _ = element.set_attribute("class", &class_string);
    }

    fn update_square_states(&self) {
        for position in 0..WORD_LENGTH * WORD_LENGTH {
            if grid::is_gap_position(position) {
                continue;
            }

            self.set_square_class(position, None);
        }
    }

    fn update_swaps_remaining(&self) {
        let text = if self.swaps_remaining == 1 {
            "Restas 1 interŝanĝo".to_string()
        } else {
            format!("Restas {} interŝanĝoj", self.swaps_remaining)
        };

        self.set_element_text(&self.swaps_remaining_message, &text);
    }

    fn move_letter_to_top(&self, position: usize) {
        let _ = self.game_grid.append_child(&self.letters[position]);
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
