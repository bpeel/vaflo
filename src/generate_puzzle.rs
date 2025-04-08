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

use super::dictionary::{Dictionary, Node};
use super::grid::{WORD_LENGTH, N_WORDS_ON_AXIS, SolutionGrid};
use rand::prelude::*;

fn list_siblings<'a>(first_node: Node<'a>) -> Vec<Node<'a>> {
    let mut siblings = vec![first_node];

    while let Some(next_sibling) = siblings.last().unwrap().next_sibling() {
        siblings.push(next_sibling);
    }

    siblings.shuffle(&mut rand::thread_rng());

    siblings
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
    mut n: Option<Node<'a>>,
    letter: char,
) -> Option<Node<'a>> {
    while let Some(sibling) = n {
        if sibling.letter() == letter {
            return Some(sibling);
        }

        n = sibling.next_sibling();
    }

    None
}

pub fn generate(dictionary: &Dictionary) -> Option<SolutionGrid> {
    let Some(first_node) = dictionary.first_node()
    else {
        return None;
    };

    let mut horizontal_words =
        std::array::from_fn::<_, { N_WORDS_ON_AXIS * WORD_LENGTH }, _>(|_| {
            first_node.clone()
        });
    let mut vertical_words = horizontal_words.clone();
    let mut stack = vec![list_siblings(first_node.clone())];

    while let Some(mut siblings) = stack.pop() {
        let Some(node) = siblings.pop()
        else {
            continue;
        };

        let pos = stack.len();

        stack.push(siblings);

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
            let letters = std::array::from_fn(|pos| {
                let x = pos % WORD_LENGTH;
                let y = pos / WORD_LENGTH;

                let ch = if y & 1 == 0 {
                    horizontal_words[y / 2 * WORD_LENGTH + x].letter()
                } else if x & 1 == 0 {
                    vertical_words[x / 2 * WORD_LENGTH + y].letter()
                } else {
                    ' '
                };

                ch.to_uppercase().next().unwrap_or(ch)
            });

            return Some(SolutionGrid { letters });
        } else {
            let next_pos = pos + 1;
            let next_group_pos = next_pos % (WORD_LENGTH + N_WORDS_ON_AXIS);

            if next_group_pos == 0 {
                stack.push(list_siblings(first_node.clone()));
            } else {
                let parent = if next_group_pos < WORD_LENGTH {
                    &node
                } else {
                    &vertical_words[
                        next_pos / (WORD_LENGTH + N_WORDS_ON_AXIS) * 2 +
                            (next_group_pos - WORD_LENGTH) * WORD_LENGTH
                    ]
                };

                if let Some(first_child) = parent.first_child() {
                    stack.push(list_siblings(first_child));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_test_dictionary() -> Dictionary {
        // Dictionary that contains the words “abcde”, “fghij”,
        // “klmno”, “pqrst”, “uvwxy”, “afkpu”, “bglqv”, “chmrw”,
        // “dinsx” and “ejoty”
        static DICTIONARY_BYTES: [u8; 150] = [
            0x00, 0x01, 0x2a, 0x01, 0x16, b'a', 0x01, 0x1f, b'b',
            0x01, 0x1f, b'c', 0x01, 0x1f, b'd', 0x01, 0x1f, b'e',
            0x01, 0x10, b'f', 0x01, 0x1c, b'k', 0x04, 0x1c, b'p',
            0x04, 0x1f, b'b', 0x00, 0x19, b'u', 0x00, 0x1f, b'f',
            0x00, 0x19, b'g', 0x00, 0x1c, b'g', 0x00, 0x1f, b'h',
            0x00, 0x1f, b'i', 0x00, 0x1f, b'j', 0x00, 0x13, b'l',
            0x00, 0x1c, b'q', 0x00, 0x1c, b'v', 0x00, 0x1c, b'c',
            0x00, 0x1c, b'h', 0x00, 0x1f, b'k', 0x00, 0x1f, b'l',
            0x00, 0x16, b'm', 0x00, 0x1c, b'm', 0x00, 0x1f, b'n',
            0x00, 0x1f, b'o', 0x00, 0x16, b'r', 0x00, 0x1c, b'w',
            0x00, 0x1c, b'd', 0x00, 0x1c, b'i', 0x00, 0x1c, b'n',
            0x00, 0x1f, b'p', 0x00, 0x1f, b'q', 0x00, 0x1f, b'r',
            0x00, 0x13, b's', 0x00, 0x1c, b's', 0x00, 0x1c, b't',
            0x00, 0x19, b'x', 0x00, 0x19, b'e', 0x00, 0x16, b'j',
            0x00, 0x13, b'o', 0x00, 0x10, b't', 0x00, 0x0d, b'u',
            0x00, 0x0a, b'v', 0x00, 0x07, b'w', 0x00, 0x04, b'x',
            0x00, 0x01, b'y', 0x00, 0x00, 0x00,
        ];

        Dictionary::new(Box::new(DICTIONARY_BYTES.clone()))
    }

    #[test]
    fn test_generate() {
        let grid = generate(&make_test_dictionary()).unwrap();

        // There are two possible solutions and it will randomly pick
        // one of them
        if grid.letters[1] == 'f' {
            assert_eq!(
                &grid.letters.iter().collect::<String>(),
                "AFKPU\
                 B L V\
                 CHMRW\
                 D N X\
                 EJOTY",
            );
        } else {
            assert_eq!(
                &grid.letters.iter().collect::<String>(),
                "ABCDE\
                 F H J\
                 KLMNO\
                 P R T\
                 UVWXY",
            );
        }
    }
}
