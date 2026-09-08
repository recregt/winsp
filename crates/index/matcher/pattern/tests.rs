// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Portions Copyright (c) 2023-2024 Pascal Kuthe <pascalkuthe@pm.me>
// Portions Copyright (c) 2026 Ömer Tekin

use crate::matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};

#[test]
fn negative() {
    let pat = Atom::parse("!foo", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.negative);
    assert_eq!(pat.kind, AtomKind::Substring);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("!^foo", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.negative);
    assert_eq!(pat.kind, AtomKind::Prefix);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("!foo$", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.negative);
    assert_eq!(pat.kind, AtomKind::Postfix);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("!^foo$", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.negative);
    assert_eq!(pat.kind, AtomKind::Exact);
    assert_eq!(pat.needle.to_string(), "foo");
}

#[test]
fn pattern_kinds() {
    let pat = Atom::parse("foo", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.negative);
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("'foo", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.negative);
    assert_eq!(pat.kind, AtomKind::Substring);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("^foo", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.negative);
    assert_eq!(pat.kind, AtomKind::Prefix);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("foo$", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.negative);
    assert_eq!(pat.kind, AtomKind::Postfix);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("^foo$", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.negative);
    assert_eq!(pat.kind, AtomKind::Exact);
    assert_eq!(pat.needle.to_string(), "foo");
}

#[test]
fn case_matching() {
    let pat = Atom::parse("foo", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("Foo", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "Foo");
    let pat = Atom::parse("Foo", CaseMatching::Ignore, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "foo");
    let pat = Atom::parse("Foo", CaseMatching::Respect, Normalization::Smart);
    assert!(!pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "Foo");
    let pat = Atom::parse("Foo", CaseMatching::Respect, Normalization::Smart);
    assert!(!pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "Foo");
    let pat = Atom::parse("Äxx", CaseMatching::Ignore, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "äxx");
    let pat = Atom::parse("Äxx", CaseMatching::Respect, Normalization::Smart);
    assert!(!pat.ignore_case);
    let pat = Atom::parse("Axx", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "Axx");
    let pat = Atom::parse("你xx", CaseMatching::Smart, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "你xx");
    let pat = Atom::parse("你xx", CaseMatching::Ignore, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "你xx");
    let pat = Atom::parse("Ⲽxx", CaseMatching::Smart, Normalization::Smart);
    assert!(!pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "Ⲽxx");
    let pat = Atom::parse("Ⲽxx", CaseMatching::Ignore, Normalization::Smart);
    assert!(pat.ignore_case);
    assert_eq!(pat.needle.to_string(), "ⲽxx");
}

#[test]
fn escape() {
    let pat = Atom::parse("foo\\ bar", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "foo bar");
    let pat = Atom::parse("\\!foo", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "!foo");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("\\'foo", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "'foo");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("\\^foo", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "^foo");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("foo\\$", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "foo$");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("^foo\\$", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "foo$");
    assert_eq!(pat.kind, AtomKind::Prefix);
    let pat = Atom::parse("\\^foo\\$", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "^foo$");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("\\!^foo\\$", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "!^foo$");
    assert_eq!(pat.kind, AtomKind::Fuzzy);
    let pat = Atom::parse("!\\^foo\\$", CaseMatching::Smart, Normalization::Smart);
    assert_eq!(pat.needle.to_string(), "^foo$");
    assert_eq!(pat.kind, AtomKind::Substring);
}
