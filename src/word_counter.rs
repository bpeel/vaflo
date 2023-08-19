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
}

pub struct WordCounter {
    words: HashMap<String, Vec<WordEntry>>,
}

fn add_entry_to_vec<I>(words: &mut Vec<WordEntry>, word: I)
where
    I: IntoIterator<Item = char>
{
    words.push(WordEntry {
        word: word.into_iter().collect::<String>(),
        count: 1,
    });
}

impl WordCounter {
    pub fn new() -> WordCounter {
        WordCounter { words: HashMap::new() }
    }

    pub fn push<I>(&mut self, word: I)
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
                    Some(stored_word) => stored_word.count += 1,
                    None => add_entry_to_vec(words, word),
                }
            })
            .or_insert_with(|| {
                let mut words = Vec::new();
                add_entry_to_vec(&mut words, insert_word);
                words
            });
    }

    pub fn counts(&self, word: &str) -> WordCounts {
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
    type Item = (&'a str, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(|entry| (entry.word.as_str(), entry.count))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn counts() {
        let mut counter = WordCounter::new();

        counter.push("MELONO".chars());
        counter.push("MELONOJ".chars());
        counter.push("MELONOJN".chars());
        counter.push("MELONO".chars());
        counter.push("MELKI".chars());

        let mut melons = counter.counts("MELONA");
        assert_eq!(melons.next(), Some(("MELONO", 2)));
        assert_eq!(melons.next(), Some(("MELONOJ", 1)));
        assert_eq!(melons.next(), Some(("MELONOJN", 1)));
        assert!(melons.next().is_none());

        let mut milkings = counter.counts("MELKIS");
        assert_eq!(milkings.next(), Some(("MELKI", 1)));
        assert!(milkings.next().is_none());

        assert!(counter.counts("BANANOJ").next().is_none());
        assert!(counter.counts("ENGLISH").next().is_none());

        counter.clear();

        assert!(counter.counts("MELONO").next().is_none());
    }
}
