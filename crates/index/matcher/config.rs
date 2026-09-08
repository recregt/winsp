// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Portions Copyright (c) 2023-2024 Pascal Kuthe <pascalkuthe@pm.me>
// Portions Copyright (c) 2026 Ömer Tekin

use crate::matcher::chars::{CharClass, ascii_char_class};
use crate::matcher::score::BONUS_BOUNDARY;

/// Configuration data that controls how a matcher behaves
#[non_exhaustive]
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Config {
    /// Characters that act as delimiters and provide bonus
    /// for matching the following char
    pub(crate) delimiter_chars: &'static [u8],
    /// Extra bonus for word boundary after whitespace character or beginning of the string
    pub(crate) bonus_boundary_white: u16,
    /// Extra bonus for word boundary after slash, colon, semi-colon, and comma
    pub(crate) bonus_boundary_delimiter: u16,
    pub(crate) initial_char_class: CharClass,
    /// The class of every ASCII byte, as [`ascii_char_class`] decides it for
    /// the delimiters above. Held here because both answers a character's class
    /// depends on are fixed once a config is built, while scoring asks for one
    /// per haystack character.
    pub(crate) ascii_char_classes: [CharClass; 256],
    /// The bonus of every ordered pair of classes, as
    /// [`Config::compute_bonus`] decides it, indexed by
    /// `prev_class * CharClass::COUNT + class`. Same reason: the bonuses above
    /// do not change while a haystack is scored, and this is the other value
    /// the scoring loops ask for per character.
    pub(crate) bonuses: [u16; CharClass::COUNT * CharClass::COUNT],

    /// Whether to normalize latin script characters to ASCII (enabled by default)
    pub normalize: bool,
    /// whether to ignore casing
    pub ignore_case: bool,
    /// Whether to provide a bonus to matches by their distance from the start
    /// of the haystack. The bonus is fairly small compared to the normal gap
    /// penalty to avoid messing with the normal score heuristic. This setting
    /// is not turned on by default and only recommended for autocompletion
    /// usecases where the expectation is that the user is typing the entire
    /// match. For a full fzf-like fuzzy matcher/picker word segmentation and
    /// explicit prefix literals should be used instead.
    pub prefer_prefix: bool,
}

impl Config {
    /// The default config for nucleo, implemented as a constant since
    /// Default::default can not be called in a const context
    pub const DEFAULT: Self = {
        let mut config = Config {
            delimiter_chars: b"/,:;|",
            bonus_boundary_white: BONUS_BOUNDARY + 2,
            bonus_boundary_delimiter: BONUS_BOUNDARY + 1,
            initial_char_class: CharClass::Whitespace,
            ascii_char_classes: [CharClass::NonWord; 256],
            bonuses: [0; CharClass::COUNT * CharClass::COUNT],
            normalize: true,
            ignore_case: true,
            prefer_prefix: false,
        };
        config.rebuild_tables();
        config
    };

    /// Fills the class and bonus tables in from the fields they are derived
    /// from. Called by every constructor, and by everything that changes one of
    /// those fields.
    const fn rebuild_tables(&mut self) {
        let mut byte = 0;
        while byte < self.ascii_char_classes.len() {
            self.ascii_char_classes[byte] = ascii_char_class(byte as u8, self.delimiter_chars);
            byte += 1;
        }

        let mut prev_class = 0;
        while prev_class < CharClass::COUNT {
            let mut class = 0;
            while class < CharClass::COUNT {
                self.bonuses[prev_class * CharClass::COUNT + class] = self.compute_bonus(
                    CharClass::from_index(prev_class),
                    CharClass::from_index(class),
                );
                class += 1;
            }
            prev_class += 1;
        }
    }
}

impl Config {
    /// Configures the matcher with bonuses appropriate for matching file paths.
    pub fn set_match_paths(&mut self) {
        if cfg!(windows) {
            self.delimiter_chars = b"/:\\";
        } else {
            self.delimiter_chars = b"/:";
        }
        self.bonus_boundary_white = BONUS_BOUNDARY;
        self.initial_char_class = CharClass::Delimiter;
        self.rebuild_tables();
    }

    /// Configures the matcher with bonuses appropriate for matching file paths.
    pub const fn match_paths(mut self) -> Self {
        if cfg!(windows) {
            self.delimiter_chars = b"/\\";
        } else {
            self.delimiter_chars = b"/";
        }
        self.bonus_boundary_white = BONUS_BOUNDARY;
        self.initial_char_class = CharClass::Delimiter;
        self.rebuild_tables();
        self
    }
}
