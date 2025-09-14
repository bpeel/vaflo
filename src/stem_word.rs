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

static SUFFIXES: [&'static str; 16] = [
    "AJN",
    "OJN",
    "AN",
    "ON",
    "AJ",
    "OJ",
    "IS",
    "AS",
    "US",
    "OS",
    "EN",
    "U",
    "O",
    "I",
    "A",
    "E",
];

pub fn stem(word: &str) -> &str {
    SUFFIXES.iter().find_map(|suffix| word.strip_suffix(suffix)).unwrap_or(word)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stems() {
        assert_eq!(stem("OPAJN"), "OP");
        assert_eq!(stem("OPOJN"), "OP");
        assert_eq!(stem("KAFAN"), "KAF");
        assert_eq!(stem("KAFON"), "KAF");
        assert_eq!(stem("KAFAJ"), "KAF");
        assert_eq!(stem("KAFOJ"), "KAF");
        assert_eq!(stem("KAFIS"), "KAF");
        assert_eq!(stem("KAFAS"), "KAF");
        assert_eq!(stem("KAFUS"), "KAF");
        assert_eq!(stem("KAFOS"), "KAF");
        assert_eq!(stem("KAFEN"), "KAF");
        assert_eq!(stem("KANTU"), "KANT");
        assert_eq!(stem("KANTO"), "KANT");
        assert_eq!(stem("KANTI"), "KANT");
        assert_eq!(stem("KANTA"), "KANT");
        assert_eq!(stem("KANTE"), "KANT");

        assert_eq!(stem("ANKAŬ"), "ANKAŬ");
    }
}
