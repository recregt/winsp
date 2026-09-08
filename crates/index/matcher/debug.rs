// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Portions Copyright (c) 2023-2024 Pascal Kuthe <pascalkuthe@pm.me>
// Portions Copyright (c) 2026 Ömer Tekin

use crate::matcher::matrix::{MatrixCell, ScoreCell};
use std::fmt::{Debug, Formatter, Result};

// impl<C: Char> MatcherData<'_, C> {
//     pub fn rows(&self) -> impl Iterator<Item = MatrixRow> + ExactSizeIterator + Clone + Sized {
//         let mut cells = &*self.cells;
//         self.row_offs.iter().map(move |&off| {
//             let len = self.haystack.len() - off as usize;
//             let (row, tmp) = cells.split_at(len);
//             cells = tmp;
//             MatrixRow { off, cells: row }
//         })
//     }

//     pub fn haystack(
//         &self,
//     ) -> impl Iterator<Item = HaystackChar<C>> + ExactSizeIterator + '_ + Clone {
//         haystack(self.haystack, self.bonus, 0)
//     }
// }

impl Debug for ScoreCell {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "({}, {})", self.score, self.matched)
    }
}

impl Debug for MatrixCell {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "({}, {})", (self.0 & 1) != 0, (self.0 & 2) != 0)
    }
}
