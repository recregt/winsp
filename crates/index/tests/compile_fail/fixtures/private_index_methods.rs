// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

struct Item;

impl winsp_index::IndexableItem for Item {
    fn name(&self) -> &str {
        ""
    }

    fn keywords(&self) -> &[String] {
        &[]
    }

    fn launch_count(&self) -> u32 {
        0
    }
}

fn main() {
    let index = winsp_index::Index::<Item>::new();
    let mut out = Vec::new();
    index.find_into("test", 5, &mut out);
}
