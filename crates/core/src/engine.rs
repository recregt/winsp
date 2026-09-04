use crate::models::{AppItem, MatchedCharIndices, SearchResult};
use compact_str::CompactString;
use nucleo_matcher::chars::{normalize, to_lower_case};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::borrow::Cow;
use std::cell::RefCell;
use std::sync::Arc;

const KEYWORD_MATCH_SCORE: i32 = 5_000;
const SEARCH_FRECENCY_MULTIPLIER: i64 = 50;
const TOP_ITEMS_FRECENCY_MULTIPLIER: i64 = 10;

pub struct Engine {
    items: Vec<Arc<AppItem>>,
    scan: ScanTable,
    matcher: RefCell<Matcher>,
    narrowing: RefCell<Option<Narrowing>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("items", &self.items.len())
            .finish()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        Self {
            items: Vec::new(),
            scan: ScanTable::default(),
            matcher: RefCell::new(Matcher::new(config)),
            narrowing: RefCell::new(None),
        }
    }

    pub fn set_items(&mut self, items: impl IntoIterator<Item = AppItem>) {
        self.items = items.into_iter().map(Arc::new).collect();
        self.scan = ScanTable::build(&self.items);
        self.narrowing.get_mut().take();
    }

    pub fn add_item(&mut self, item: AppItem) {
        let item = Arc::new(item);
        self.scan.push(&item);
        self.items.push(item);
        self.narrowing.get_mut().take();
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.scan = ScanTable::default();
        self.narrowing.get_mut().take();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn top_items(&self, limit: usize) -> Vec<SearchResult> {
        if limit == 0 {
            return Vec::new();
        }

        // Bounded selection over the launch count column: scoring the whole
        // index does not require touching a single `AppItem`.
        let mut top: Vec<(u32, i32)> = Vec::with_capacity(limit.min(self.items.len()));
        for (idx, &launch_count) in self.scan.launch_counts.iter().enumerate() {
            let score = launch_score(launch_count, TOP_ITEMS_FRECENCY_MULTIPLIER);
            if top.len() == limit {
                if score <= top[limit - 1].1 {
                    continue;
                }
                top.pop();
            }
            let pos = top.partition_point(|&(_, kept)| kept >= score);
            top.insert(pos, (idx as u32, score));
        }

        top.into_iter()
            .map(|(idx, score)| {
                SearchResult::from_app(
                    Arc::clone(&self.items[idx as usize]),
                    score,
                    MatchedCharIndices::new(),
                )
            })
            .collect()
    }

    fn find(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if limit == 0 {
            return Vec::new();
        }

        let mut matcher = self.matcher.borrow_mut();
        let needles = Needles::new(query);
        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(needles.name(), &mut needle_buf);
        let query_lower = needles.keyword();
        let query_mask = needle_mask(needles.name());

        // A keystroke only ever shrinks the previous match set, so the scan can
        // start from it instead of from the whole index.
        let previous = self.narrowing.borrow_mut().take();
        let narrowed = previous
            .as_ref()
            .filter(|previous| narrows(&previous.query, query))
            .map(|previous| previous.items.as_slice());

        let scan = self.scan.best_matches(
            &mut matcher,
            needle,
            query_lower,
            query_mask,
            limit,
            narrowed,
        );
        *self.narrowing.borrow_mut() = scan.matched.map(|items| Narrowing {
            query: query.to_owned(),
            items,
        });

        let mut hay_buf = Vec::new();
        let mut raw_indices = Vec::new();

        scan.top
            .into_iter()
            .map(|candidate| {
                let item = Arc::clone(&self.items[candidate.item as usize]);
                let indices: MatchedCharIndices = if candidate.matched_by_name {
                    hay_buf.clear();
                    raw_indices.clear();
                    let haystack = Utf32Str::new(item.name(), &mut hay_buf);
                    matcher.fuzzy_indices(haystack, needle, &mut raw_indices);
                    raw_indices.iter().map(|&i| i as usize).collect()
                } else {
                    MatchedCharIndices::new()
                };
                SearchResult::from_app(item, candidate.score, indices)
            })
            .collect()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.top_items(limit);
        }

        let calc_result = crate::calc::eval(trimmed)
            .map(|res| SearchResult::calculation(CompactString::new(trimmed), res));

        let mut results = self.find(trimmed, limit);
        if let Some(calc) = calc_result {
            results.insert(0, calc);
            results.truncate(limit);
        }
        results
    }
}

struct Candidate {
    item: u32,
    score: i32,
    matched_by_name: bool,
}

/// The query in the two spellings a scan compares against: normalized and
/// lowercased for fuzzy name matching, and lowercased for keyword matching,
/// which compares characters as they are.
///
/// The two only differ for a query that is not ASCII, because normalization
/// leaves ASCII alone and lowercasing it is the same operation either way. An
/// ASCII query therefore needs a single string, and one that is already
/// lowercase — the common keystroke — is that string, so nothing is allocated.
enum Needles<'a> {
    Ascii(Cow<'a, str>),
    Unicode { name: String, keyword: String },
}

impl<'a> Needles<'a> {
    fn new(query: &'a str) -> Self {
        if !query.is_ascii() {
            return Self::Unicode {
                name: query.chars().map(normalize).map(to_lower_case).collect(),
                keyword: query.to_lowercase(),
            };
        }
        if query.bytes().any(|byte| byte.is_ascii_uppercase()) {
            Self::Ascii(Cow::Owned(query.to_ascii_lowercase()))
        } else {
            Self::Ascii(Cow::Borrowed(query))
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Ascii(query) => query,
            Self::Unicode { name, .. } => name,
        }
    }

    fn keyword(&self) -> &str {
        match self {
            Self::Ascii(query) => query,
            Self::Unicode { keyword, .. } => keyword,
        }
    }
}

/// Complete match set of the last query, kept so the next keystroke can rescan
/// those items only.
struct Narrowing {
    query: String,
    items: Vec<u32>,
}

/// Whether the items matching `query` are a subset of the ones that matched
/// `previous`. Appending characters to a query can only shrink the match set:
/// fuzzy name matching needs the needle to be a subsequence of the name, and
/// keyword matching needs the query to be a substring of a keyword, and both
/// still hold for any prefix of the query.
///
/// Restricted to ASCII queries because `str::to_lowercase`, which the keyword
/// comparison uses, is context sensitive: a final sigma lowercases differently
/// once another character follows it, so the lowercased queries would not be
/// prefixes of each other.
fn narrows(previous: &str, query: &str) -> bool {
    query.is_ascii() && query.starts_with(previous)
}

/// Items retained for the next keystroke, as a share of the index. Rescanning a
/// list costs a cache miss per item, where a full scan streams through the
/// columnar buffers, so a barely narrowed set is not worth reusing. Small
/// indexes are scanned quickly either way, hence the floor.
fn narrowing_capacity(items: usize) -> usize {
    (items / 8).max(64)
}

/// What one scan produced: the best `limit` candidates, and the complete match
/// set when it is small enough to narrow the next scan with.
struct Scan {
    top: Vec<Candidate>,
    matched: Option<Vec<u32>>,
}

/// Keyword separator inside [`ScanTable::keywords`]. Queries come from a
/// single-line input, so a hit in the blob stays inside one keyword.
const KEYWORD_SEPARATOR: char = '\n';

/// Search-only projection of the index, laid out for a linear scan: the fields
/// the per-keystroke scan reads live in contiguous buffers instead of behind one
/// `Arc<AppItem>` indirection (plus one `Vec<String>`) per item.
#[derive(Default)]
struct ScanTable {
    /// Every item name, concatenated.
    names: String,
    /// Every item's keywords, lowercased and separated by [`KEYWORD_SEPARATOR`].
    keywords: String,
    /// Character bitmask of every name, in item order. Kept in its own buffer so
    /// the prefilter walks 4 bytes per item instead of a whole row.
    name_masks: Vec<u32>,
    /// Character bitmask of every keyword blob slice, in item order.
    keyword_masks: Vec<u32>,
    /// Launch count of every item, in item order. Duplicated from
    /// [`ScanRow::launch_count`] so the no-query path scans 4 bytes per item.
    launch_counts: Vec<u32>,
    rows: Vec<ScanRow>,
}

/// Bit set over the characters a haystack contains: one bit per ASCII letter
/// plus one catch-all bit for everything else. A fuzzy name match needs every
/// needle character to appear in the name, and a keyword match needs every query
/// character to appear in a keyword, so a needle whose mask is not a subset of
/// the haystack mask cannot match. Conservative: never rejects a real match.
///
/// Returns the mask together with whether the text is pure ASCII, in one pass.
fn haystack_mask(text: &str) -> (u32, bool) {
    let mut mask = 0;
    for (offset, &byte) in text.as_bytes().iter().enumerate() {
        if !byte.is_ascii() {
            return (mask | unicode_mask(&text[offset..]), false);
        }
        mask |= ascii_bit(byte);
    }
    (mask, true)
}

fn unicode_mask(text: &str) -> u32 {
    let mut mask = 0;
    for ch in text.chars() {
        // Keyword matching compares characters as they are while name matching
        // normalizes them first, so account for both spellings.
        mask |= char_bit(ch) | char_bit(normalize(to_lower_case(ch)));
    }
    mask
}

/// Mask of the characters a match requires. The needle is already normalized and
/// lowercased, so every character maps to exactly the bit it needs.
fn needle_mask(needle: &str) -> u32 {
    needle.chars().map(char_bit).fold(0, |mask, bit| mask | bit)
}

const OTHER_CHAR_BIT: u32 = 1 << 26;

fn ascii_bit(byte: u8) -> u32 {
    match byte.to_ascii_lowercase() {
        lower @ b'a'..=b'z' => 1 << (lower - b'a'),
        _ => OTHER_CHAR_BIT,
    }
}

fn char_bit(ch: char) -> u32 {
    if ch.is_ascii() {
        return ascii_bit(ch as u8);
    }
    let lower = to_lower_case(ch);
    if lower.is_ascii_lowercase() {
        1 << (lower as u8 - b'a')
    } else {
        OTHER_CHAR_BIT
    }
}

#[derive(Clone, Copy)]
struct ScanRow {
    name_start: u32,
    name_end: u32,
    keywords_start: u32,
    keywords_end: u32,
    launch_count: u32,
    name_is_ascii: bool,
}

impl ScanTable {
    fn build(items: &[Arc<AppItem>]) -> Self {
        let names_len: usize = items.iter().map(|item| item.name().len()).sum();
        let keywords_len: usize = items
            .iter()
            .flat_map(|item| item.keywords())
            .map(|keyword| keyword.len() + 1)
            .sum();
        let mut table = Self {
            names: String::with_capacity(names_len),
            keywords: String::with_capacity(keywords_len),
            name_masks: Vec::with_capacity(items.len()),
            keyword_masks: Vec::with_capacity(items.len()),
            launch_counts: Vec::with_capacity(items.len()),
            rows: Vec::with_capacity(items.len()),
        };
        for item in items {
            table.push(item);
        }
        table
    }

    fn push(&mut self, item: &AppItem) {
        let name_start = self.names.len() as u32;
        self.names.push_str(item.name());
        let keywords_start = self.keywords.len() as u32;
        for keyword in item.keywords() {
            self.keywords.push_str(keyword);
            self.keywords.push(KEYWORD_SEPARATOR);
        }
        let (name_mask, name_is_ascii) = haystack_mask(item.name());
        let (keyword_mask, _) = haystack_mask(&self.keywords[keywords_start as usize..]);
        self.name_masks.push(name_mask);
        self.keyword_masks.push(keyword_mask);
        self.launch_counts.push(item.launch_count());
        self.rows.push(ScanRow {
            name_start,
            name_end: self.names.len() as u32,
            keywords_start,
            keywords_end: self.keywords.len() as u32,
            launch_count: item.launch_count(),
            name_is_ascii,
        });
    }

    /// Scores every item that survives the mask prefilter and keeps the best
    /// `limit` of them, highest score first, ties going to the lower item index.
    /// The selection is a bounded insertion, which assumes the small result
    /// limits a launcher UI asks for.
    ///
    /// `narrowed`, when given, is the complete match set of a query this one
    /// extends, and replaces the index as the set of items to score.
    fn best_matches(
        &self,
        matcher: &mut Matcher,
        needle: Utf32Str<'_>,
        query_lower: &str,
        query_mask: u32,
        limit: usize,
        narrowed: Option<&[u32]>,
    ) -> Scan {
        match narrowed {
            Some(items) => {
                let rows = items.iter().map(|&idx| {
                    let index = idx as usize;
                    (
                        idx,
                        &self.rows[index],
                        self.name_masks[index],
                        self.keyword_masks[index],
                    )
                });
                self.scan(rows, matcher, needle, query_lower, query_mask, limit)
            }
            None => {
                // Walked as parallel iterators: the whole index is read in
                // order, so none of the columns needs a bounds check.
                let rows = self
                    .rows
                    .iter()
                    .zip(self.name_masks.iter().copied())
                    .zip(self.keyword_masks.iter().copied())
                    .enumerate()
                    .map(|(idx, ((row, name_mask), keyword_mask))| {
                        (idx as u32, row, name_mask, keyword_mask)
                    });
                self.scan(rows, matcher, needle, query_lower, query_mask, limit)
            }
        }
    }

    fn scan<'a, I: Iterator<Item = (u32, &'a ScanRow, u32, u32)>>(
        &self,
        source: I,
        matcher: &mut Matcher,
        needle: Utf32Str<'_>,
        query_lower: &str,
        query_mask: u32,
        limit: usize,
    ) -> Scan {
        let mut top: Vec<Candidate> = Vec::with_capacity(limit.min(self.rows.len()));
        let mut matched: Option<Vec<u32>> = Some(Vec::new());
        let capacity = narrowing_capacity(self.rows.len());
        let mut hay_buf = Vec::new();
        let needle_len = needle.len() as u32;

        for (idx, row, name_mask, keyword_mask) in source {
            // A name shorter than the needle cannot hold it, which the matcher
            // would have to be called to find out.
            let name_possible =
                query_mask & !name_mask == 0 && needle_len <= row.name_end - row.name_start;
            let keyword_possible = query_mask & !keyword_mask == 0;
            if !name_possible && !keyword_possible {
                continue;
            }

            let name_score = if name_possible {
                let name_range = row.name_start as usize..row.name_end as usize;
                let score = if row.name_is_ascii {
                    // Byte indexing skips the UTF-8 boundary checks of `str`.
                    let haystack = Utf32Str::Ascii(&self.names.as_bytes()[name_range]);
                    matcher.fuzzy_match(haystack, needle)
                } else {
                    match_unicode_name(matcher, &self.names[name_range], needle, &mut hay_buf)
                };
                score.map(|score| score as i32)
            } else {
                None
            };

            let keyword_score = if keyword_possible {
                let keywords =
                    &self.keywords[row.keywords_start as usize..row.keywords_end as usize];
                keywords
                    .contains(query_lower)
                    .then_some(KEYWORD_MATCH_SCORE)
            } else {
                None
            };

            let best = match (name_score, keyword_score) {
                (Some(name), Some(kw)) => Some((name.max(kw), true)),
                (Some(name), None) => Some((name, true)),
                (None, Some(kw)) => Some((kw, false)),
                (None, None) => None,
            };

            if let Some((score, matched_by_name)) = best {
                if let Some(items) = &mut matched {
                    if items.len() == capacity {
                        // Too many matches left to narrow the next scan with.
                        matched = None;
                    } else {
                        items.push(idx);
                    }
                }

                let frecency_boost = launch_score(row.launch_count, SEARCH_FRECENCY_MULTIPLIER);
                let score = score.saturating_add(frecency_boost);
                if top.len() == limit && score <= top[limit - 1].score {
                    continue;
                }
                keep_best(
                    &mut top,
                    limit,
                    Candidate {
                        item: idx,
                        score,
                        matched_by_name,
                    },
                );
            }
        }

        Scan { top, matched }
    }
}

/// Matches a name that is not pure ASCII, which has to be decoded into
/// codepoints first. Kept out of line: names are ASCII in the common case, and
/// the decoding would otherwise sit in the middle of the scan loop.
#[inline(never)]
fn match_unicode_name(
    matcher: &mut Matcher,
    name: &str,
    needle: Utf32Str<'_>,
    buf: &mut Vec<char>,
) -> Option<u16> {
    buf.clear();
    let haystack = Utf32Str::new(name, buf);
    matcher.fuzzy_match(haystack, needle)
}

/// Inserts `candidate` into the descending, `limit` long `top` list. Kept out of
/// line so the scan loop that calls it stays tight.
#[inline(never)]
fn keep_best(top: &mut Vec<Candidate>, limit: usize, candidate: Candidate) {
    if top.len() == limit {
        top.pop();
    }
    let pos = top.partition_point(|kept| kept.score >= candidate.score);
    top.insert(pos, candidate);
}

fn launch_score(launch_count: u32, multiplier: i64) -> i32 {
    ((launch_count as i64) * multiplier).min(i32::MAX as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LaunchTarget;

    fn sample_index() -> Engine {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "notepad",
                "Notepad",
                LaunchTarget::Path("notepad.exe".into()),
            ),
            AppItem::new(
                "vscode",
                "Visual Studio Code",
                LaunchTarget::Path("code.exe".into()),
            ),
            AppItem::new(
                "calc",
                "Calculator",
                LaunchTarget::OsUri("shell:AppsFolder\\Microsoft.WindowsCalculator".into()),
            ),
            AppItem::new(
                "terminal",
                "Windows Terminal",
                LaunchTarget::Path("wt.exe".into()),
            ),
            AppItem::new(
                "chrome",
                "Google Chrome",
                LaunchTarget::Path("chrome.exe".into()),
            )
            .with_keywords(vec!["browser".into(), "web".into(), "internet".into()]),
            AppItem::new(
                "settings",
                "Windows Settings",
                LaunchTarget::OsUri("ms-settings:".into()),
            ),
        ]);
        index
    }

    fn titles(results: &[SearchResult]) -> Vec<&str> {
        results.iter().map(|r| r.title.as_ref()).collect()
    }

    #[test]
    fn test_exact_and_prefix_search() {
        let index = sample_index();

        let results = index.find("calc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Calculator");

        let results = index.find("not", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Notepad");
    }

    #[test]
    fn test_acronym_search() {
        let index = sample_index();

        let results = index.find("vsc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Visual Studio Code");

        let results = index.find("gc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Google Chrome");
    }

    #[test]
    fn test_keyword_score_not_shadowed_by_weak_name_match() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "scattered",
                "T z z z z e z z z z r z z z z m z z z z z z z z",
                LaunchTarget::Path("term.exe".into()),
            )
            .with_keywords(vec!["term".into()]),
        ]);

        let results = index.find("term", 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].score >= 5_000);
        assert!(!results[0].matched_char_indices.is_empty());
    }

    #[test]
    fn test_ascii_needles_are_the_unicode_ones() {
        // One string serves both matchers on the ASCII path, which holds only
        // because normalization leaves ASCII alone and the two lowercasings
        // agree on it.
        for byte in 0..=127u8 {
            let query = (byte as char).to_string();
            let name: String = query.chars().map(normalize).map(to_lower_case).collect();
            let needles = Needles::new(&query);
            assert_eq!(needles.name(), name, "name needle for {byte:#04x}");
            assert_eq!(
                needles.keyword(),
                query.to_lowercase(),
                "keyword needle for {byte:#04x}"
            );
        }
    }

    #[test]
    fn test_unicode_needles_keep_both_spellings() {
        let needles = Needles::new("Är");
        assert_eq!(needles.name(), "ar");
        assert_eq!(needles.keyword(), "är");
    }

    #[test]
    fn test_uppercase_and_accented_queries_still_match() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "chrome",
                "Google Chrome",
                LaunchTarget::Path("c.exe".into()),
            )
            .with_keywords(vec!["BROWSER".into()]),
            AppItem::new("uber", "Über Editor", LaunchTarget::Path("u.exe".into())),
        ]);

        assert_eq!(titles(&index.find("CHROME", 5)), vec!["Google Chrome"]);
        assert_eq!(titles(&index.find("BrowSer", 5)), vec!["Google Chrome"]);
        assert_eq!(titles(&index.find("Über", 5)), vec!["Über Editor"]);
        assert_eq!(titles(&index.find("uber", 5)), vec!["Über Editor"]);
    }

    #[test]
    fn test_keyword_search() {
        let index = sample_index();

        let results = index.find("browser", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Google Chrome");
    }

    #[test]
    fn test_no_match_is_excluded_even_with_partial_letters() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new("chrome", "Chrome", LaunchTarget::Path("chrome.exe".into())),
            AppItem::new(
                "google-chrome",
                "Google Chrome",
                LaunchTarget::Path("chrome.exe".into()),
            ),
            AppItem::new(
                "chromium",
                "Chromium",
                LaunchTarget::Path("chromium.exe".into()),
            ),
            AppItem::new(
                "chrome-devtools",
                "Chrome DevTools",
                LaunchTarget::Path("chrome.exe".into()),
            ),
        ]);

        let results = index.find("chrome", 10);
        assert!(!titles(&results).contains(&"Chromium"));
        assert!(titles(&results).contains(&"Chrome"));
        assert!(titles(&results).contains(&"Google Chrome"));
        assert!(titles(&results).contains(&"Chrome DevTools"));
    }

    #[test]
    fn test_prefix_outranks_acronym() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new("vscode", "VS Code", LaunchTarget::Path("code.exe".into())),
            AppItem::new(
                "vstudio",
                "Visual Studio",
                LaunchTarget::Path("devenv.exe".into()),
            ),
        ]);

        let results = index.find("vs", 5);
        assert_eq!(titles(&results), vec!["VS Code", "Visual Studio"]);
    }

    #[test]
    fn test_acronym_outranks_substring() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "open-office-go",
                "Open Office Go",
                LaunchTarget::Path("oog.exe".into()),
            ),
            AppItem::new("google", "Google", LaunchTarget::Path("chrome.exe".into())),
        ]);

        let results = index.find("oog", 5);
        assert_eq!(titles(&results), vec!["Open Office Go", "Google"]);
    }

    #[test]
    fn test_word_start_bonus_can_outrank_a_midword_substring() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "notepad",
                "Notepad",
                LaunchTarget::Path("notepad.exe".into()),
            ),
            AppItem::new(
                "paint-design",
                "Paint Design",
                LaunchTarget::Path("paint.exe".into()),
            ),
        ]);

        let results = index.find("pad", 5);
        assert_eq!(titles(&results), vec!["Paint Design", "Notepad"]);
    }

    #[test]
    fn test_keyword_match_outranks_a_weak_fuzzy_name_match() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "rand-setup",
                "Random Windows Setup",
                LaunchTarget::Path("setup.exe".into()),
            ),
            AppItem::new(
                "chrome",
                "Google Chrome",
                LaunchTarget::Path("chrome.exe".into()),
            )
            .with_keywords(vec!["browser".into()]),
        ]);

        let results = index.find("rows", 5);
        assert_eq!(
            titles(&results),
            vec!["Google Chrome", "Random Windows Setup"]
        );
    }

    #[test]
    fn test_frecency_breaks_ties_between_identical_names() {
        use crate::models::SearchResultKind;

        let popular =
            AppItem::new("a", "Test App", LaunchTarget::Path("a.exe".into())).with_launch_count(10);
        let rare = AppItem::new("b", "Test App", LaunchTarget::Path("b.exe".into()));

        let mut index = Engine::new();
        index.set_items(vec![rare, popular]);

        let results = index.find("test app", 5);
        assert_eq!(results.len(), 2);
        let SearchResultKind::App(item) = &results[0].kind else {
            panic!("expected an App result");
        };
        assert_eq!(item.id(), "a");
    }

    #[test]
    fn test_frecency_applies_to_keyword_matches_too() {
        let popular = AppItem::new("a", "Aardvark Tool", LaunchTarget::Path("a.exe".into()))
            .with_keywords(vec!["zzzmatch".into()])
            .with_launch_count(10);
        let rare = AppItem::new("b", "Yak Tool", LaunchTarget::Path("b.exe".into()))
            .with_keywords(vec!["zzzmatch".into()]);

        let mut index = Engine::new();
        index.set_items(vec![rare, popular]);

        let results = index.find("zzzmatch", 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title.as_ref(), "Aardvark Tool");
    }

    #[test]
    fn test_keyword_matching_normalizes_case_at_construction_time() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new(
                "chrome",
                "Google Chrome",
                LaunchTarget::Path("chrome.exe".into()),
            )
            .with_keywords(vec!["BROWSER".into()]),
        ]);

        let results = index.find("browser", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Google Chrome");
    }

    #[test]
    fn test_unicode_names_match_case_insensitively_with_correct_indices() {
        let mut index = Engine::new();
        index.set_items(vec![
            AppItem::new("cafe", "Café", LaunchTarget::Path("cafe.exe".into())),
            AppItem::new(
                "nihongo",
                "日本語アプリ",
                LaunchTarget::Path("nihongo.exe".into()),
            ),
        ]);

        let results = index.find("CAF", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "Café");
        assert_eq!(
            results[0].matched_char_indices,
            MatchedCharIndices::from_slice(&[0, 1, 2])
        );

        let results = index.find("アプリ", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "日本語アプリ");
        assert_eq!(
            results[0].matched_char_indices,
            MatchedCharIndices::from_slice(&[3, 4, 5])
        );
    }

    #[test]
    fn test_launch_score_saturates_instead_of_overflowing() {
        assert_eq!(launch_score(u32::MAX, 50), i32::MAX);
        assert_eq!(launch_score(0, 50), 0);
    }

    #[test]
    fn test_extreme_launch_count_does_not_panic_or_go_negative() {
        let item = AppItem::new("bulk", "Bulk App", LaunchTarget::Path("bulk.exe".into()))
            .with_launch_count(u32::MAX);
        let mut index = Engine::new();
        index.add_item(item);

        let top = index.top_items(1);
        assert_eq!(top.len(), 1);
        assert!(top[0].score >= 0);

        let found = index.find("bulk", 1);
        assert_eq!(found.len(), 1);
        assert!(found[0].score >= 0);
    }

    #[test]
    fn test_query_with_multi_char_unicode_lowercase_expansion_still_matches() {
        let mut index = Engine::new();
        index.set_items(vec![AppItem::new(
            "istanbul",
            "İstanbul Maps",
            LaunchTarget::Path("istanbul.exe".into()),
        )]);

        let results = index.find("İ", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "İstanbul Maps");
        assert_eq!(
            results[0].matched_char_indices,
            MatchedCharIndices::from_slice(&[0])
        );
    }

    #[test]
    fn test_typing_a_query_matches_searching_it_directly() {
        let sessions = [
            "visual studio code",
            "browser",
            "windows ",
            "café",
            "İst",
            "calc",
            "qqq",
        ];

        for session in sessions {
            let typed = sample_index();
            for (offset, ch) in session.char_indices() {
                let query = &session[..offset + ch.len_utf8()];
                let cold = sample_index();
                assert_eq!(
                    titles(&typed.find(query, 5)),
                    titles(&cold.find(query, 5)),
                    "query: {query:?}"
                );
            }
        }
    }

    #[test]
    fn test_narrowing_survives_a_result_limit_smaller_than_the_match_set() {
        let index = sample_index();
        let cold = sample_index();

        // The first search drops matches it cannot return, the second one still
        // has to see them.
        assert_eq!(index.find("windows", 1).len(), 1);
        let results = index.find("windows t", 5);
        assert!(!results.is_empty());
        assert_eq!(titles(&results), titles(&cold.find("windows t", 5)));
    }

    #[test]
    fn test_typing_matches_a_cold_search_past_the_narrowing_capacity() {
        // Wide enough that early keystrokes match more items than the engine
        // keeps, which forces the following keystroke back to a full scan.
        fn large_index() -> Engine {
            let items: Vec<AppItem> = (0..500)
                .map(|i| {
                    let name = match i % 3 {
                        0 => format!("Visual Studio {i}"),
                        1 => format!("Video Editor {i}"),
                        _ => format!("Notepad {i}"),
                    };
                    AppItem::new(
                        format!("id-{i}"),
                        name,
                        LaunchTarget::Path(format!("{i}.exe")),
                    )
                    .with_keywords(vec!["tool".into()])
                })
                .collect();
            let mut index = Engine::new();
            index.set_items(items);
            index
        }

        let session = "visual studio 42";
        let typed = large_index();
        for (offset, ch) in session.char_indices() {
            let query = &session[..offset + ch.len_utf8()];
            let cold = large_index();
            assert_eq!(
                titles(&typed.find(query, 6)),
                titles(&cold.find(query, 6)),
                "query: {query:?}"
            );
        }
    }

    #[test]
    fn test_editing_the_index_invalidates_the_narrowed_scan() {
        let mut index = sample_index();
        assert!(titles(&index.find("vis", 5)).contains(&"Visual Studio Code"));

        index.add_item(AppItem::new(
            "vim",
            "Vim",
            LaunchTarget::Path("vim.exe".into()),
        ));
        assert!(titles(&index.find("vi", 5)).contains(&"Vim"));

        index.set_items(vec![AppItem::new(
            "gimp",
            "GIMP",
            LaunchTarget::Path("gimp.exe".into()),
        )]);
        assert_eq!(titles(&index.find("gi", 5)), vec!["GIMP"]);

        index.clear();
        assert!(index.find("gi", 5).is_empty());
    }

    /// Same scoring rules as [`ScanTable::best_matches`], without any prefilter.
    fn reference_titles(items: &[AppItem], query: &str) -> Vec<String> {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        let mut matcher = Matcher::new(config);
        let needle_string: String = query.chars().map(normalize).map(to_lower_case).collect();
        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(&needle_string, &mut needle_buf);
        let query_lower = query.to_lowercase();
        let mut hay_buf = Vec::new();

        let mut matched: Vec<String> = Vec::new();
        for item in items {
            hay_buf.clear();
            let haystack = Utf32Str::new(item.name(), &mut hay_buf);
            let by_name = matcher.fuzzy_match(haystack, needle).is_some();
            let by_keyword = item.keywords().iter().any(|kw| kw.contains(&query_lower));
            if by_name || by_keyword {
                matched.push(item.name().to_string());
            }
        }
        matched.sort();
        matched
    }

    #[test]
    fn test_prefilter_never_drops_a_match() {
        let names = [
            "Notepad",
            "Visual Studio Code",
            "Café Manager",
            "CAFE 42",
            "Ünïcöde Viewer",
            "ZIP Extractor 7",
            "system-monitor",
            "Файл Менеджер",
        ];
        let items: Vec<AppItem> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                AppItem::new(
                    format!("id-{i}"),
                    *name,
                    LaunchTarget::Path(format!("{i}.exe")),
                )
                .with_keywords(vec!["Tool".into(), "Café".into(), "42".into()])
            })
            .collect();

        let mut index = Engine::new();
        index.set_items(items.clone());

        let queries = [
            "n",
            "no",
            "cafe",
            "café",
            "CAFÉ",
            "42",
            "7",
            "zip",
            "vsc",
            "ü",
            "u",
            "unicode",
            "файл",
            "system-monitor",
            "tool",
            "café manager",
            "qqq",
            " ",
            "-",
        ];
        for query in queries {
            let mut got: Vec<String> = index
                .find(query, usize::MAX)
                .into_iter()
                .map(|r| r.title.to_string())
                .collect();
            got.sort();
            assert_eq!(got, reference_titles(&items, query), "query: {query:?}");
        }
    }

    #[test]
    fn test_math_expression_is_merged_into_results() {
        let index = Engine::new();

        let results = index.search("25 * 4", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "100");
    }

    #[test]
    fn test_math_result_ranks_above_app_matches() {
        let mut index = Engine::new();
        index.set_items(vec![AppItem::new(
            "calc",
            "2 Calculators",
            LaunchTarget::OsUri("shell:AppsFolder\\Microsoft.WindowsCalculator".into()),
        )]);

        let results = index.search("2 + 2", 5);
        assert_eq!(results[0].title.as_ref(), "4");
    }

    #[test]
    fn test_empty_query_lists_top_items_without_touching_math() {
        let mut index = Engine::new();
        let popular = AppItem::new("a", "Popular App", LaunchTarget::Path("a.exe".into()))
            .with_launch_count(10);
        index.set_items(vec![popular]);

        let results = index.search("", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "Popular App");
    }
}
