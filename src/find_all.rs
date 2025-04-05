// Vaflo – A word game in Esperanto
// Copyright (C) 2025  Neil Roberts
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

mod dictionary;

use dictionary::Dictionary;
use std::process::ExitCode;

const WORD_LENGTH: usize = 5;
const N_WORDS_ON_AXIS: usize = (WORD_LENGTH + 1) / 2;

fn load_dictionary() -> Result<Dictionary, ()> {
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
        Ok(d) => Ok(Dictionary::new(d.into_boxed_slice())),
    }
}

fn vertical_word_pos(pos: usize) -> Option<(usize, usize)> {
    // The position within the group, where a group is a horizontal
    // word followed by a row of letters used only in the vertical
    // words
    let group_pos = pos % (WORD_LENGTH + N_WORDS_ON_AXIS);

    if group_pos < WORD_LENGTH {
        ((group_pos & 1) == 0).then(|| {
            (
                group_pos / 2,
                pos / (WORD_LENGTH + N_WORDS_ON_AXIS) * 2,
            )
        })
    } else {
        Some((
            group_pos - WORD_LENGTH,
            pos / (WORD_LENGTH + N_WORDS_ON_AXIS) * 2 + 1,
        ))
    }
}

fn find_sibling<'a>(
    mut n: Option<dictionary::Node<'a>>,
    letter: char,
) -> Option<dictionary::Node<'a>> {
    while let Some(sibling) = n {
        if sibling.letter() == letter {
            return Some(sibling);
        }

        n = sibling.next_sibling();
    }

    None
}

fn print_solution<'a>(
    horizontal_words: &[dictionary::Node<'a>],
    vertical_words: &[dictionary::Node<'a>],
) {
    for y in 0..N_WORDS_ON_AXIS {
        for x in 0..WORD_LENGTH {
            print!(
                "{}",
                horizontal_words[y * WORD_LENGTH + x].letter(),
            );
        }

        println!();

        if y < N_WORDS_ON_AXIS - 1 {
            for x in 0..N_WORDS_ON_AXIS {
                print!(
                    "{} ",
                    vertical_words[x * WORD_LENGTH + y * 2 + 1]
                        .letter(),
                );
            }
        }

        println!();
    }
}

fn count_puzzles(dictionary: &Dictionary) -> u128 {
    let Some(first_node) = dictionary.first_node()
    else {
        return 0;
    };

    let mut horizontal_words =
        std::array::from_fn::<_, { N_WORDS_ON_AXIS * WORD_LENGTH }, _>(|_| {
            first_node.clone()
        });
    let mut vertical_words = horizontal_words.clone();
    let mut stack = vec![Some(first_node.clone())];

    let mut count = 0;

    while let Some(node) = stack.pop() {
        let Some(node) = node
        else {
            continue;
        };

        let pos = stack.len();

        stack.push(node.next_sibling());

        // The position within the group, where a group is a
        // horizontal word followed by a row of letters used only in
        // the vertical words
        let group_pos = pos % (WORD_LENGTH + N_WORDS_ON_AXIS);

        // Does the pos intersect with a horizontal word?
        if group_pos < WORD_LENGTH {
            let word_num = pos / (WORD_LENGTH + N_WORDS_ON_AXIS);
            let word_start = word_num * WORD_LENGTH;
            let letter_pos = word_start + group_pos;

            horizontal_words[letter_pos] = node.clone();
        }

        // Does the pos intersect with a vertical word?
        if let Some((word_num, word_pos)) = vertical_word_pos(pos) {
            let word_start = word_num * WORD_LENGTH;
            let letter_pos = word_start + word_pos;

            let sibling = if word_pos == 0 {
                Some(first_node.clone())
            } else {
                vertical_words[letter_pos - 1].first_child()
            };

            // Make sure there this letter can follow the previous one
            // in the vertical word
            match find_sibling(sibling, node.letter()) {
                Some(sibling) => vertical_words[letter_pos] = sibling,
                None => continue,
            }
        }

        // Have we filled the grid?
        if pos >= WORD_LENGTH * N_WORDS_ON_AXIS +
            (WORD_LENGTH - N_WORDS_ON_AXIS) * N_WORDS_ON_AXIS -
            1
        {
            count += 1;
            if count % 1_000_000 == 0 {
                print_solution(&horizontal_words, &vertical_words);
                println!("{}", count);
            }
        } else {
            let next_pos = pos + 1;
            let next_group_pos = next_pos % (WORD_LENGTH + N_WORDS_ON_AXIS);

            if next_group_pos == 0 {
                stack.push(Some(first_node.clone()));
            } else if next_group_pos < WORD_LENGTH {
                stack.push(node.first_child());
            } else {
                let previous_letter = &vertical_words[
                    next_pos / (WORD_LENGTH + N_WORDS_ON_AXIS) * 2 +
                        (next_group_pos - WORD_LENGTH) * WORD_LENGTH
                ];
                stack.push(previous_letter.first_child());
            }
        }
    }

    count
}

fn main() -> ExitCode {
    let Ok(dictionary) = load_dictionary()
    else {
        return ExitCode::FAILURE;
    };

    println!("{}", count_puzzles(&dictionary));

    ExitCode::SUCCESS
}
