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

use std::collections::HashMap;
use super::stem_word;

struct WordEntry {
    word: String,
    count: usize,
    last_use: usize,
}

pub struct WordCounter {
    words: HashMap<String, Vec<WordEntry>>,
}

fn add_entry_to_vec<I>(words: &mut Vec<WordEntry>, word: I, last_use: usize)
where
    I: IntoIterator<Item = char>
{
    words.push(WordEntry {
        word: word.into_iter().collect::<String>(),
        count: 1,
        last_use,
    });
}

impl WordCounter {
    pub fn new() -> WordCounter {
        WordCounter { words: HashMap::new() }
    }

    pub fn push<I>(&mut self, word: I, last_use: usize)
    where
        I: Iterator<Item = char> + Clone
    {
        let mut stem = word.clone().collect::<String>();
        let stem_length = stem_word::stem(&stem).len();
        stem.truncate(stem_length);

        let insert_word = word.clone();

        self.words.entry(stem)
            .and_modify(|words| {
                match words.iter_mut()
                    .find(|stored_word| {
                        stored_word.word.chars().eq(word.clone())
                    })
                {
                    Some(stored_word) => {
                        stored_word.count += 1;
                        stored_word.last_use =
                            last_use.max(stored_word.last_use);
                    },
                    None => add_entry_to_vec(words, word, last_use),
                }
            })
            .or_insert_with(|| {
                let mut words = Vec::new();
                add_entry_to_vec(&mut words, insert_word, last_use);
                words
            });
    }

    pub fn counts<'a>(&'a self, word: &str) -> WordCounts<'a> {
        let entries = self.words.get(stem_word::stem(word))
            .map(|entries| entries.iter())
            .unwrap_or_else(|| [].iter());

        WordCounts { entries }
    }

    pub fn clear(&mut self) {
        self.words.clear();
    }
}

pub struct WordCounts<'a> {
    entries: std::slice::Iter<'a, WordEntry>,
}

impl<'a> Iterator for WordCounts<'a> {
    type Item = (&'a str, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(|entry| {
            (entry.word.as_str(), entry.count, entry.last_use)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn counts() {
        let mut counter = WordCounter::new();

        counter.push("MELONO".chars(), 3);
        counter.push("MELONOJ".chars(), 2);
        counter.push("MELONOJN".chars(), 4);
        counter.push("MELONO".chars(), 4);
        counter.push("MELONO".chars(), 2);
        counter.push("MELKI".chars(), 42);

        let mut melons = counter.counts("MELONA");
        assert_eq!(melons.next(), Some(("MELONO", 3, 4)));
        assert_eq!(melons.next(), Some(("MELONOJ", 1, 2)));
        assert_eq!(melons.next(), Some(("MELONOJN", 1, 4)));
        assert!(melons.next().is_none());

        let mut milkings = counter.counts("MELKIS");
        assert_eq!(milkings.next(), Some(("MELKI", 1, 42)));
        assert!(milkings.next().is_none());

        assert!(counter.counts("BANANOJ").next().is_none());
        assert!(counter.counts("ENGLISH").next().is_none());

        counter.clear();

        assert!(counter.counts("MELONO").next().is_none());
    }
}
