use nucleo_matcher::chars::{normalize, to_lower_case};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::borrow::Cow;
use std::cell::{RefCell, RefMut};
use std::sync::Arc;

const KEYWORD_MATCH_SCORE: i32 = 5_000;
const SEARCH_FRECENCY_MULTIPLIER: i64 = 50;
const TOP_ITEMS_FRECENCY_MULTIPLIER: i64 = 10;

/// What [`Index`] needs from an item to search it: a name and keywords to
/// match against, and a launch count to break ties in the item's favor.
pub trait IndexableItem {
    fn name(&self) -> &str;
    fn keywords(&self) -> &[String];
    fn launch_count(&self) -> u32;
}

/// One item `Index::search` returned: the item itself, its score, and which of
/// its name's characters the query matched, for highlighting.
#[derive(Debug, Clone, PartialEq)]
pub struct Match<T> {
    pub item: Arc<T>,
    pub score: i32,
    pub matched_char_indices: Vec<usize>,
}

pub struct Index<T: IndexableItem> {
    items: Vec<Arc<T>>,
    ranking: Ranking,
}

impl<T: IndexableItem> std::fmt::Debug for Index<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Index")
            .field("items", &self.items.len())
            .finish()
    }
}

impl<T: IndexableItem> Default for Index<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IndexableItem> Index<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            ranking: Ranking::new(),
        }
    }

    pub fn set_items(&mut self, items: impl IntoIterator<Item = T>) {
        self.items = items.into_iter().map(Arc::new).collect();
        self.ranking.rebuild(&self.items);
    }

    pub fn add_item(&mut self, item: T) {
        let item = Arc::new(item);
        self.ranking.push(item.as_ref());
        self.items.push(item);
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.ranking.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[cfg(test)]
    fn top_items(&self, limit: usize) -> Vec<Match<T>> {
        let mut out = Vec::new();
        self.top_items_into(limit, &mut out);
        out
    }

    fn top_items_into(&self, limit: usize, out: &mut Vec<Match<T>>) {
        self.collect(self.ranking.top(limit), out);
    }

    #[cfg(test)]
    fn find(&self, query: &str, limit: usize) -> Vec<Match<T>> {
        let mut out = Vec::new();
        self.find_into(query, limit, &mut out);
        out
    }

    fn find_into(&self, query: &str, limit: usize, out: &mut Vec<Match<T>>) {
        self.collect(self.ranking.find(query, limit), out);
    }

    /// Turns the item indices a ranking picked into the items themselves. The
    /// only step of a search that knows what an item is, and the only one that
    /// touches item memory: everything before it reads the columns of
    /// [`ScanTable`], which hold no `T`.
    fn collect(&self, mut ranked: RefMut<'_, Vec<Ranked>>, out: &mut Vec<Match<T>>) {
        out.clear();
        out.extend(ranked.drain(..).map(|ranked| Match {
            item: Arc::clone(&self.items[ranked.item as usize]),
            score: ranked.score,
            matched_char_indices: ranked.matched_char_indices,
        }));
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Match<T>> {
        let mut out = Vec::new();
        self.search_into(query, limit, &mut out);
        out
    }

    /// Same as [`Index::search`], but reuses `out`'s existing allocation
    /// instead of returning a freshly allocated `Vec` on every call, which
    /// matters on a UI thread re-searching once per keystroke.
    pub fn search_into(&self, query: &str, limit: usize, out: &mut Vec<Match<T>>) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            self.top_items_into(limit, out);
        } else {
            self.find_into(trimmed, limit, out);
        }
    }
}

/// One item a query picked, as an index into the item list. What a search
/// produces before it is given an item type.
struct Ranked {
    item: u32,
    score: i32,
    matched_char_indices: Vec<usize>,
}

/// Everything a search reads that is not an item: the columnar projection of
/// the index, the matcher, and the match set kept for the next keystroke.
///
/// Deliberately not generic, and the reason [`Index`]'s own body is as thin as
/// it is. A generic body is compiled again in every crate that names an item
/// type, under that crate's code generation rather than this one's, which is
/// enough to change how a loop over an index-sized column comes out. Nothing a
/// search reads is a `T` — names, keywords and launch counts all live in
/// columns — so the whole of it is ranked here, once, and handed back as item
/// indices for [`Index::collect`] to look up.
struct Ranking {
    scan: ScanTable,
    matcher: RefCell<Matcher>,
    narrowing: RefCell<Option<Narrowing>>,
    /// Where a ranking puts its results, kept between queries so that a
    /// keystroke reuses the allocation instead of making one.
    ranked: RefCell<Vec<Ranked>>,
    /// The buffers a single search fills and empties again, kept for the same
    /// reason.
    scratch: RefCell<Scratch>,
}

/// What one search needs room for beyond its results: the buffers the matcher
/// decodes into, and the character indices it reports for the items the window
/// shows.
///
/// Kept between queries because a launcher searches once per keystroke, and
/// every one of these was a fresh allocation on every one of them.
#[derive(Default)]
struct Scratch {
    /// Where a query that is not ASCII is decoded to.
    needle_buf: Vec<char>,
    /// Where a name that is not ASCII is decoded to, for highlighting.
    hay_buf: Vec<char>,
    /// Where the matcher reports the characters of a name it matched.
    raw_indices: Vec<u32>,
}

impl Ranking {
    fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        Self {
            scan: ScanTable::default(),
            matcher: RefCell::new(Matcher::new(config)),
            narrowing: RefCell::new(None),
            ranked: RefCell::new(Vec::new()),
            scratch: RefCell::new(Scratch::default()),
        }
    }

    fn rebuild<T: IndexableItem>(&mut self, items: &[Arc<T>]) {
        self.scan = ScanTable::build(items);
        self.narrowing.get_mut().take();
    }

    fn push<T: IndexableItem>(&mut self, item: &T) {
        self.scan.push(item);
        self.narrowing.get_mut().take();
    }

    fn clear(&mut self) {
        self.scan = ScanTable::default();
        self.narrowing.get_mut().take();
    }

    /// The items an empty query answers with: the most launched ones.
    fn top(&self, limit: usize) -> RefMut<'_, Vec<Ranked>> {
        let mut ranked = self.ranked.borrow_mut();
        ranked.clear();
        if limit == 0 {
            return ranked;
        }

        ranked.extend(
            self.scan
                .best_by_launch_count(limit)
                .into_iter()
                .map(|(item, score)| Ranked {
                    item,
                    score,
                    matched_char_indices: Vec::new(),
                }),
        );
        ranked
    }

    /// The best `limit` items for `query`, with the characters of their names
    /// the query matched, for highlighting.
    fn find(&self, query: &str, limit: usize) -> RefMut<'_, Vec<Ranked>> {
        let mut ranked = self.ranked.borrow_mut();
        ranked.clear();
        if limit == 0 {
            return ranked;
        }

        let mut matcher = self.matcher.borrow_mut();
        let mut scratch = self.scratch.borrow_mut();
        // Field by field, since the needle borrows one of them for as long as
        // the matcher is given it while the rest are still written to.
        let Scratch {
            needle_buf,
            hay_buf,
            raw_indices,
        } = &mut *scratch;

        let needles = Needles::new(query);
        let needle = Utf32Str::new(needles.name(), needle_buf);
        let query_lower = needles.keyword();
        let query_masks = needle_masks(needles.name());

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
            query_masks,
            limit,
            narrowed,
        );

        // The previous match set is superseded, so both what it held and the
        // string its query was kept in go back to be filled again.
        let mut query_buf = match previous {
            Some(previous) => {
                self.scan.recycle(previous.items);
                previous.query
            }
            None => String::new(),
        };
        *self.narrowing.borrow_mut() = scan.matched.map(|items| {
            query_buf.clear();
            query_buf.push_str(query);
            Narrowing {
                query: query_buf,
                items,
            }
        });

        ranked.extend(scan.top.into_iter().map(|candidate| {
            let indices = if candidate.matched_by_name {
                raw_indices.clear();
                // The name the scan matched, out of the column it matched it
                // in, which is the item's name as it was indexed.
                let haystack = Utf32Str::new(self.scan.name(candidate.item), hay_buf);
                matcher.fuzzy_indices(haystack, needle, raw_indices);
                raw_indices.iter().map(|&i| i as usize).collect()
            } else {
                Vec::new()
            };
            Ranked {
                item: candidate.item,
                score: candidate.score,
                matched_char_indices: indices,
            }
        }));
        ranked
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
/// `Arc<T>` indirection (plus one `Vec<String>`) per item.
#[derive(Default)]
struct ScanTable {
    /// Every item name, concatenated.
    names: String,
    /// Every item's keywords, lowercased and separated by [`KEYWORD_SEPARATOR`].
    keywords: String,
    /// Everything the prefilter reads, one [`prefilter_word`] per item: the two
    /// character bitmasks and the name length. Packed into a single column so
    /// the item a query rules out costs one 8 byte load, instead of three
    /// buffers walked in step.
    prefilter: Vec<u64>,
    /// Launch count of every item, in item order. Duplicated from
    /// [`ScanRow::launch_count`] so the no-query path scans 4 bytes per item.
    launch_counts: Vec<u32>,
    rows: Vec<ScanRow>,
    /// The match set of an earlier search, emptied, for the next one to fill
    /// again. A scan records every item it matched so the next keystroke can
    /// rescan those alone, and growing that set from nothing was a chain of
    /// reallocations on every keystroke.
    spare_matched: RefCell<Vec<u32>>,
}

/// Bit set over the characters a haystack contains, as [`MASK_BITS`] groups
/// them: one bit per ASCII letter, one for the digits, one for whitespace and
/// one for everything else. A fuzzy name match needs every
/// needle character to appear in the name, and a keyword match needs every query
/// character to appear in a keyword, so a needle whose mask is not a subset of
/// the haystack mask cannot match. Conservative: never rejects a real match.
fn haystack_mask(text: &str) -> u32 {
    let mut mask = 0;
    for (offset, &byte) in text.as_bytes().iter().enumerate() {
        if !byte.is_ascii() {
            return mask | unicode_mask(&text[offset..]);
        }
        mask |= ascii_bit(byte);
    }
    mask
}

/// Everything one pass over an item's name tells the scan: which characters it
/// holds, which of those a match could earn a bonus on, and whether it is pure
/// ASCII.
///
/// The second mask is what bounds a name match tighter than the needle's length
/// alone can — see [`NameCeilings`]. A name that is not ASCII claims every
/// character instead of a real mask: the character classes its bonuses follow
/// are unicode ones the columns do not model, and claiming everything is the
/// bound the engine held every name to before there was a boundary mask.
fn name_masks(name: &str) -> (u32, u32, bool) {
    let mut present = 0;
    let mut boundary = 0;
    let mut previous = WHITESPACE;
    for (offset, &byte) in name.as_bytes().iter().enumerate() {
        if !byte.is_ascii() {
            return (present | unicode_mask(&name[offset..]), ALL_CHARS, false);
        }
        // One lookup for both of the things this pass needs from a byte, since
        // it runs over every character of every item the index is built from.
        let entry = NAME_BYTES[byte as usize];
        let bit = entry & ALL_CHARS;
        let class = entry >> CLASS_SHIFT;
        present |= bit;
        if EARNS_A_BONUS[previous as usize] >> class & 1 != 0 {
            boundary |= bit;
        }
        previous = class as u8;
    }
    (present, boundary, true)
}

/// Character classes `nucleo_matcher` sorts a haystack character into, for the
/// ASCII characters a name column can hold. Its `Letter` class is for
/// characters that are alphabetic without having a case, which no ASCII
/// character is, so it is left out.
const WHITESPACE: u8 = 0;
const NON_WORD: u8 = 1;
const DELIMITER: u8 = 2;
const LOWER: u8 = 3;
const UPPER: u8 = 4;
const NUMBER: u8 = 5;
const CLASS_COUNT: usize = 6;

/// Characters `Config::DEFAULT` treats as delimiters.
const DELIMITER_CHARS: &[u8] = b"/,:;|";

const fn char_class(byte: u8) -> u8 {
    match byte {
        b'a'..=b'z' => LOWER,
        b'A'..=b'Z' => UPPER,
        b'0'..=b'9' => NUMBER,
        b' ' | b'\t' | b'\n' | b'\x0c' | b'\r' => WHITESPACE,
        _ => {
            let mut nth = 0;
            while nth < DELIMITER_CHARS.len() {
                if DELIMITER_CHARS[nth] == byte {
                    return DELIMITER;
                }
                nth += 1;
            }
            NON_WORD
        }
    }
}

/// Whether a character of `class` preceded by one of `previous` earns any bonus
/// at all, which is the question [`name_masks`] asks of every position of a
/// name. Mirrors `nucleo_matcher`'s `Config::bonus_for` under the configuration
/// this engine builds its matcher with, reduced to whether the bonus is
/// non-zero; `test_boundary_mask_agrees_with_the_matcher` keeps the two
/// together.
const fn earns_a_bonus(previous: u8, class: u8) -> bool {
    let is_word = matches!(class, LOWER | UPPER | NUMBER);
    if is_word && matches!(previous, WHITESPACE | DELIMITER | NON_WORD) {
        // A word starting after whitespace, a delimiter or a non-word
        // character.
        return true;
    }
    if previous == LOWER && class == UPPER || previous != NUMBER && class == NUMBER {
        // camelCase, or letter123.
        return true;
    }
    matches!(class, WHITESPACE | NON_WORD)
}

/// Where a [`NAME_BYTES`] entry keeps the character class, above the mask bit.
const CLASS_SHIFT: u32 = MASK_BITS;

/// What one pass over a name needs to know about a byte: the mask bit it
/// contributes and the class it belongs to, in one word, so that the pass costs
/// a load per byte rather than two chains of comparisons.
const NAME_BYTES: [u32; 128] = {
    let mut table = [0; 128];
    let mut byte = 0;
    while byte < table.len() {
        table[byte] = ascii_bit(byte as u8) | (char_class(byte as u8) as u32) << CLASS_SHIFT;
        byte += 1;
    }
    table
};

/// Bit `class` of `EARNS_A_BONUS[previous]` is set exactly when
/// [`earns_a_bonus`] holds for the pair.
const EARNS_A_BONUS: [u32; CLASS_COUNT] = {
    let mut table = [0; CLASS_COUNT];
    let mut previous = 0;
    while previous < CLASS_COUNT {
        let mut class = 0;
        while class < CLASS_COUNT {
            if earns_a_bonus(previous as u8, class as u8) {
                table[previous] |= 1 << class;
            }
            class += 1;
        }
        previous += 1;
    }
    table
};

fn unicode_mask(text: &str) -> u32 {
    let mut mask = 0;
    for ch in text.chars() {
        // Keyword matching compares characters as they are while name matching
        // normalizes them first, so account for both spellings.
        mask |= char_bit(ch) | char_bit(normalize(to_lower_case(ch)));
    }
    mask
}

/// Masks of the characters a match requires. The needle is already normalized
/// and lowercased, so every character maps to exactly the bit it needs.
///
/// The first character is kept apart from the rest because the matcher scores
/// its bonus twice, so it is worth bounding on its own — see [`NameCeilings`].
#[derive(Clone, Copy)]
struct QueryMasks {
    all_chars: u32,
    first_char: u32,
}

fn needle_masks(needle: &str) -> QueryMasks {
    let mut chars = needle.chars().map(char_bit);
    let first_char = chars.next().unwrap_or(0);
    QueryMasks {
        all_chars: chars.fold(first_char, |mask, bit| mask | bit),
        first_char,
    }
}

/// Bits a character mask occupies: one per ASCII letter, one for the digits,
/// one for whitespace, and the catch-all for everything else.
///
/// Digits and whitespace are kept out of the catch-all because a name holds
/// both and an expression holds neither on its own: with one bit for every
/// character that is not a letter, `12 * 12` claimed the same bit as the space
/// in `Visual Studio 2022`, so the mask ruled nothing out and every item of the
/// index was handed to the matcher. Told apart, the `*` is a character no
/// ordinary name contains and the item is ruled out by the word the prefilter
/// already reads.
const MASK_BITS: u32 = 29;
/// Any ASCII digit. One bit for all ten: a needle digit only asks whether the
/// haystack holds a digit at all, which is what keeps the mask conservative.
const DIGIT_CHAR_BIT: u32 = 1 << 26;
/// Any ASCII whitespace, which is what separates the words of a name.
const SPACE_CHAR_BIT: u32 = 1 << 27;
/// Everything else: ASCII punctuation, and every character that is not ASCII.
const OTHER_CHAR_BIT: u32 = 1 << (MASK_BITS - 1);
/// Every bit a character mask can hold: the mask of a haystack that has to be
/// assumed to contain anything.
const ALL_CHARS: u32 = (1 << MASK_BITS) - 1;

const fn ascii_bit(byte: u8) -> u32 {
    match byte.to_ascii_lowercase() {
        lower @ b'a'..=b'z' => 1 << (lower - b'a'),
        b'0'..=b'9' => DIGIT_CHAR_BIT,
        // Every spelling of whitespace claims the one bit, so a needle written
        // with a tab still matches the name that spells it with a space.
        b' ' | b'\t' | b'\n' | b'\x0c' | b'\r' => SPACE_CHAR_BIT,
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

/// Where the keyword mask sits in a [`prefilter_word`].
const KEYWORD_MASK_SHIFT: u32 = MASK_BITS;
/// Where the name length sits in a [`prefilter_word`]: the high bits, so that
/// comparing whole words compares name lengths first.
const NAME_LEN_SHIFT: u32 = 2 * MASK_BITS;
/// Longest name a [`prefilter_word`] can hold the length of: what the two masks
/// leave of the word. A longer name saturates, which only makes the length test
/// let more items through.
const MAX_PACKED_NAME_LEN: u64 = (1 << (u64::BITS - NAME_LEN_SHIFT)) - 1;

/// Everything one item contributes to the prefilter, in one word: its name mask
/// in the low bits, its keyword mask above that, and its name length in the
/// high bits.
fn prefilter_word(name_mask: u32, keyword_mask: u32, name_len: usize) -> u64 {
    let name_len = (name_len as u64).min(MAX_PACKED_NAME_LEN);
    u64::from(name_mask)
        | (u64::from(keyword_mask) << KEYWORD_MASK_SHIFT)
        | (name_len << NAME_LEN_SHIFT)
}

/// Smallest prefilter word whose name is long enough to hold a needle of
/// `chars` characters, which is what a name has to be for the matcher to have
/// anything to look at. The name length occupies the high bits of a word, so a
/// word compares greater than this exactly when its name is long enough — the
/// same comparison the name length would make on its own, as one instruction
/// on the word the masks were loaded with.
///
/// The lengths are byte lengths, as they were when they were compared field by
/// field: a name that is not ASCII holds more bytes than characters, so the
/// test stays on the conservative side of the character count.
///
/// A needle longer than a packed name length can be is not compared at all,
/// since the saturated lengths no longer order against it.
fn name_len_floor(chars: usize) -> u64 {
    if chars as u64 > MAX_PACKED_NAME_LEN {
        return 0;
    }
    (chars as u64) << NAME_LEN_SHIFT
}

/// Bit of [`ScanRow::name_bonuses`] that marks a name as pure ASCII, above the
/// bits the boundary mask itself occupies.
const NAME_IS_ASCII_BIT: u32 = 1 << MASK_BITS;

#[derive(Clone, Copy)]
struct ScanRow {
    name_start: u32,
    name_end: u32,
    keywords_start: u32,
    keywords_end: u32,
    launch_count: u32,
    /// The characters of the name a match could earn a bonus on, as
    /// [`name_masks`] builds them, plus [`NAME_IS_ASCII_BIT`]. Both ride in the
    /// padding the row had anyway, so the boundary mask needs no column of its
    /// own and arrives with the row the pruning path already reads.
    name_bonuses: u32,
}

impl ScanRow {
    /// Characters of the name a match could earn a bonus on.
    fn boundary_mask(&self) -> u32 {
        self.name_bonuses & ALL_CHARS
    }

    /// Whether the name is pure ASCII, and can therefore be matched byte-wise.
    fn name_is_ascii(&self) -> bool {
        self.name_bonuses & NAME_IS_ASCII_BIT != 0
    }
}

impl ScanTable {
    fn build<T: IndexableItem>(items: &[Arc<T>]) -> Self {
        let names_len: usize = items.iter().map(|item| item.name().len()).sum();
        let keywords_len: usize = items
            .iter()
            .flat_map(|item| item.keywords())
            .map(|keyword| keyword.len() + 1)
            .sum();
        let mut table = Self {
            names: String::with_capacity(names_len),
            keywords: String::with_capacity(keywords_len),
            prefilter: Vec::with_capacity(items.len()),
            launch_counts: Vec::with_capacity(items.len()),
            rows: Vec::with_capacity(items.len()),
            spare_matched: RefCell::new(Vec::new()),
        };
        for item in items {
            table.push(item.as_ref());
        }
        table
    }

    fn push<T: IndexableItem>(&mut self, item: &T) {
        let name_start = self.names.len() as u32;
        self.names.push_str(item.name());
        let keywords_start = self.keywords.len() as u32;
        // Masked one keyword at a time rather than over the blob they are
        // written to: a match stays inside one keyword, so the separators
        // between them are characters no match can consume. Folding them in
        // would set the catch-all bit on every item that has a keyword at all,
        // and let every query holding a digit, a space or any punctuation past
        // the keyword test — most of them, since a query with two words holds a
        // space.
        let mut keyword_mask = 0;
        for keyword in item.keywords() {
            keyword_mask |= haystack_mask(keyword);
            self.keywords.push_str(keyword);
            self.keywords.push(KEYWORD_SEPARATOR);
        }
        let (name_mask, boundary_mask, name_is_ascii) = name_masks(item.name());
        self.prefilter
            .push(prefilter_word(name_mask, keyword_mask, item.name().len()));
        self.launch_counts.push(item.launch_count());
        self.rows.push(ScanRow {
            name_start,
            name_end: self.names.len() as u32,
            keywords_start,
            keywords_end: self.keywords.len() as u32,
            launch_count: item.launch_count(),
            name_bonuses: if name_is_ascii {
                boundary_mask | NAME_IS_ASCII_BIT
            } else {
                boundary_mask
            },
        });
    }

    /// The buffer a scan records its matches in: whatever an earlier search
    /// left behind, emptied, and room for as many items as a match set is
    /// allowed to hold when there is nothing to reuse.
    fn take_spare(&self) -> Vec<u32> {
        let mut spare = std::mem::take(&mut *self.spare_matched.borrow_mut());
        spare.clear();
        if spare.capacity() == 0 {
            spare = Vec::with_capacity(narrowing_capacity(self.rows.len()));
        }
        spare
    }

    /// Takes a match set nobody reads any more, so the next scan can record
    /// itself in it rather than in a fresh allocation.
    fn recycle(&self, items: Vec<u32>) {
        let mut spare = self.spare_matched.borrow_mut();
        if spare.capacity() < items.capacity() {
            *spare = items;
        }
    }

    /// Name of an item, as it was indexed.
    fn name(&self, idx: u32) -> &str {
        let row = &self.rows[idx as usize];
        &self.names[row.name_start as usize..row.name_end as usize]
    }

    /// Best `limit` items of the index by launch count alone, highest score
    /// first, ties going to the lower item index: what an empty query answers
    /// with. Bounded selection over the launch count column, so ranking the
    /// whole index does not require touching a single item.
    fn best_by_launch_count(&self, limit: usize) -> Vec<(u32, i32)> {
        let mut top: Vec<(u32, i32)> = Vec::with_capacity(limit.min(self.launch_counts.len()));
        for (idx, &launch_count) in self.launch_counts.iter().enumerate() {
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
        top
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
        masks: QueryMasks,
        limit: usize,
        narrowed: Option<&[u32]>,
    ) -> Scan {
        match narrowed {
            Some(items) => {
                let words = items.iter().map(|&idx| (idx, self.prefilter[idx as usize]));
                self.scan(words, matcher, needle, query_lower, masks, limit)
            }
            None => {
                // The whole column is read in order, so the prefilter word of
                // an item needs no bounds check.
                let words = self
                    .prefilter
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(idx, word)| (idx as u32, word));
                self.scan(words, matcher, needle, query_lower, masks, limit)
            }
        }
    }

    fn scan<I: Iterator<Item = (u32, u64)>>(
        &self,
        mut source: I,
        matcher: &mut Matcher,
        needle: Utf32Str<'_>,
        query_lower: &str,
        masks: QueryMasks,
        limit: usize,
    ) -> Scan {
        let query_mask = masks.all_chars;
        // The prefilter reads one word per item and tests it against these
        // three: the query characters a name must hold, the same characters in
        // the keyword field, and the shortest name the needle fits in.
        let name_probe = u64::from(query_mask);
        let keyword_probe = name_probe << KEYWORD_MASK_SHIFT;
        let name_len_floor = name_len_floor(needle.len());
        let name_ceilings = NameCeilings::new(needle.len(), masks);
        let query = Query {
            needle,
            lowercased: query_lower,
            name_ceilings: &name_ceilings,
            ascii_needle: match needle {
                Utf32Str::Ascii(needle) => Some(needle),
                Utf32Str::Unicode(_) => None,
            },
        };
        let mut state = ScanState {
            matcher,
            top: Vec::with_capacity(limit.min(self.rows.len())),
            matched: Some(self.take_spare()),
            capacity: narrowing_capacity(self.rows.len()),
            limit,
            hay_buf: Vec::new(),
        };

        // Nothing can be ruled out before the shortlist is full, so the scan
        // starts in a loop that knows nothing about the ceiling. A query that
        // never fills the shortlist — one matching fewer items than the limit,
        // or nothing at all — is scanned by this loop alone, and pays the mask
        // test and nothing else for an item the mask rejects.
        for (idx, word) in source.by_ref() {
            // A name shorter than the needle cannot hold it, which the matcher
            // would have to be called to find out.
            let missing = !word;
            let name_possible = name_probe & missing == 0 && word >= name_len_floor;
            let keyword_possible = keyword_probe & missing == 0;
            if !name_possible && !keyword_possible {
                continue;
            }

            let row = &self.rows[idx as usize];
            self.score_row(&mut state, query, idx, row, name_possible, keyword_possible);
            if state.top.len() == limit {
                break;
            }
        }

        // The shortlist is full from here on, which is what gives the ceiling
        // something to beat: the highest score an item could come out with is
        // known before it is scored, and once that ceiling cannot beat the
        // lowest score on the shortlist, scoring the item would only confirm
        // what the bound already says. This is the comparison the scored path
        // makes below, on a value that can only be larger, so the ranking stays
        // the one an unpruned scan produces.
        //
        // A dropped item still has to go into the match set the next keystroke
        // narrows its scan with, which takes deciding whether it matches at all
        // — much less work than scoring it, but not always possible without the
        // matcher, in which case it is scored after all.
        for (idx, word) in source {
            let missing = !word;
            let name_possible = name_probe & missing == 0 && word >= name_len_floor;
            let keyword_possible = keyword_probe & missing == 0;
            if !name_possible && !keyword_possible {
                continue;
            }

            if let Some(is_match) = ruled_out_unscored(
                self,
                idx,
                name_possible,
                keyword_possible,
                &query,
                state.top[limit - 1].score,
                state.matched.is_some(),
            ) {
                if is_match {
                    record_match(&mut state.matched, state.capacity, idx);
                }
                continue;
            }

            // Only an item the bound could not rule out is worth its row.
            let row = &self.rows[idx as usize];
            self.score_row(&mut state, query, idx, row, name_possible, keyword_possible);
        }

        Scan {
            top: state.top,
            matched: state.matched,
        }
    }

    /// Scores a row the mask prefilter let through, and folds it into the
    /// shortlist and the match set.
    ///
    /// Inlined into both scan loops, which is what lets the first one compile to
    /// the loop a scan without a ceiling compiles to: the shortlist is not full
    /// there, so nothing of the pruning above belongs in it.
    #[inline(always)]
    fn score_row(
        &self,
        state: &mut ScanState<'_>,
        query: Query<'_>,
        idx: u32,
        row: &ScanRow,
        name_possible: bool,
        keyword_possible: bool,
    ) {
        let name_score = if name_possible {
            let name_range = row.name_start as usize..row.name_end as usize;
            let score = if row.name_is_ascii() {
                // Byte indexing skips the UTF-8 boundary checks of `str`.
                let haystack = Utf32Str::Ascii(&self.names.as_bytes()[name_range]);
                state.matcher.fuzzy_match(haystack, query.needle)
            } else {
                match_unicode_name(
                    state.matcher,
                    &self.names[name_range],
                    query.needle,
                    &mut state.hay_buf,
                )
            };
            score.map(|score| score as i32)
        } else {
            None
        };

        let keyword_score = if keyword_possible {
            let keywords = &self.keywords[row.keywords_start as usize..row.keywords_end as usize];
            keywords
                .contains(query.lowercased)
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
            record_match(&mut state.matched, state.capacity, idx);

            let frecency_boost = launch_score(row.launch_count, SEARCH_FRECENCY_MULTIPLIER);
            let score = score.saturating_add(frecency_boost);
            if state.top.len() == state.limit && score <= state.top[state.limit - 1].score {
                return;
            }
            keep_best(
                &mut state.top,
                state.limit,
                Candidate {
                    item: idx,
                    score,
                    matched_by_name,
                },
            );
        }
    }
}

/// What a scan builds up as it walks the items it was given.
struct ScanState<'m> {
    matcher: &'m mut Matcher,
    /// The best candidates seen so far, highest score first.
    top: Vec<Candidate>,
    /// Every item matched so far, until there are more of them than narrowing
    /// the next scan with is worth.
    matched: Option<Vec<u32>>,
    /// How many items the match set is willing to hold.
    capacity: usize,
    /// How many candidates the shortlist keeps.
    limit: usize,
    /// Scratch the matcher decodes a name that is not ASCII into.
    hay_buf: Vec<char>,
}

/// The query in the spellings a scan compares against.
#[derive(Clone, Copy)]
struct Query<'a> {
    /// The needle as the matcher takes it: normalized and lowercased.
    needle: Utf32Str<'a>,
    /// The query as keyword matching compares it.
    lowercased: &'a str,
    /// What a name match is bounded by. Behind a reference: only the pruning
    /// path reads it, and the scan loop carries the rest of this by value.
    name_ceilings: &'a NameCeilings,
    /// The needle, when it is ASCII and can therefore be compared byte-wise.
    ascii_needle: Option<&'a [u8]>,
}

/// Whether the score ceiling rules `row` out, and if so whether the match set
/// still being built wants it, which is all a ruled out item is still asked for.
/// `None` leaves the item to be scored: either the ceiling does not rule it out,
/// or deciding the match needs the matcher after all.
///
/// `collecting` says whether the match set is still alive. Once it has been
/// given up, a ruled out item is not going anywhere, so whether it matches is
/// nothing anyone reads and deciding it is work the scan can skip — which is
/// what the rest of a scan over an index that matches the query widely does.
///
/// Kept out of line, like the rest of what only some items reach: the scan loop
/// calls this once the shortlist is full, and a scan whose shortlist never fills
/// does not call it at all, so none of this belongs in the loop body.
#[inline(never)]
fn ruled_out_unscored(
    table: &ScanTable,
    idx: u32,
    name_possible: bool,
    keyword_possible: bool,
    query: &Query<'_>,
    cutoff: i32,
    collecting: bool,
) -> Option<bool> {
    // The two things a name match competes with: a keyword match, which scores
    // a flat `KEYWORD_MATCH_SCORE`, and the item's frecency, which is added to
    // either.
    let row = &table.rows[idx as usize];
    let keyword_ceiling = if keyword_possible {
        KEYWORD_MATCH_SCORE
    } else {
        0
    };
    let boost = launch_score(row.launch_count, SEARCH_FRECENCY_MULTIPLIER);
    let ruled_out =
        |name_ceiling: i32| name_ceiling.max(keyword_ceiling).saturating_add(boost) <= cutoff;

    let by_length = if name_possible {
        query.name_ceilings.by_length
    } else {
        0
    };
    if !ruled_out(by_length) {
        // What the needle's length allows is not enough to drop this item. What
        // this name in particular can score often still is, and asking costs
        // nothing beyond the row already read here — which is why the boundary
        // mask lives in the row rather than in the word the prefilter streams
        // for every item of the index.
        if !name_possible || !ruled_out(query.name_ceilings.of(row.boundary_mask())) {
            return None;
        }
    }
    if !collecting {
        return Some(false);
    }

    let by_name = match (name_possible, query.ascii_needle) {
        (false, _) => false,
        // An ASCII name holds an ASCII needle exactly when the needle is a
        // subsequence of it, which is what the matcher's own prefilter decides
        // before it scores anything.
        (true, Some(needle)) if row.name_is_ascii() => {
            let name = &table.names.as_bytes()[row.name_start as usize..row.name_end as usize];
            is_subsequence_ignore_ascii_case(name, needle)
        }
        // A needle that is not ASCII never matches an ASCII name, since
        // normalizing one leaves it as it is.
        (true, None) if row.name_is_ascii() => false,
        // Anything else has to be normalized before it can be compared, which
        // is the matcher's job.
        (true, _) => return None,
    };

    let by_keyword = keyword_possible && {
        let keywords = &table.keywords[row.keywords_start as usize..row.keywords_end as usize];
        keywords.contains(query.lowercased)
    };

    Some(by_name || by_keyword)
}

/// Records `idx` in the match set the next keystroke narrows its scan with, or
/// gives the set up once it holds more items than that is worth.
fn record_match(matched: &mut Option<Vec<u32>>, capacity: usize, idx: u32) {
    if let Some(items) = matched {
        if items.len() == capacity {
            matched.take();
        } else {
            items.push(idx);
        }
    }
}

/// Bit that tells an ASCII letter's two cases apart.
const ASCII_CASE_BIT: u8 = 0b10_0000;

/// The bit a haystack byte is folded with before it is compared to `wanted`:
/// the case bit when `wanted` is a letter, since only that letter's two cases
/// fold onto it, and nothing otherwise, since then only the byte itself equals
/// it. Cheaper than lowercasing every haystack byte, and the same comparison.
fn case_fold_bit(wanted: u8) -> u8 {
    if wanted.is_ascii_lowercase() {
        ASCII_CASE_BIT
    } else {
        0
    }
}

/// Whether `needle`, which is ASCII and already lowercase, appears in
/// `haystack` as a subsequence, ignoring case.
fn is_subsequence_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    let mut needle = needle.iter().copied();
    let Some(mut wanted) = needle.next() else {
        return true;
    };
    let mut fold = case_fold_bit(wanted);
    for &byte in haystack {
        if byte | fold == wanted {
            match needle.next() {
                Some(next) => {
                    wanted = next;
                    fold = case_fold_bit(next);
                }
                None => return true,
            }
        }
    }
    false
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

/// Score `nucleo_matcher` gives a matched character, before its bonus.
const SCORE_MATCH: i64 = 16;
/// Largest bonus a matched character can earn under the configuration this
/// engine builds its matcher with: `Config::DEFAULT`'s `bonus_boundary_white`,
/// the bonus for a word starting after whitespace, which is also the highest
/// bonus a run of consecutive matches can inherit.
const MAX_CHAR_BONUS: i64 = 10;
/// The first matched character counts its bonus twice.
const FIRST_CHAR_BONUS_MULTIPLIER: i64 = 2;

/// Smallest bonus a matched character can be given when the one before it in
/// the needle matched the character right before it in the name: a run of
/// consecutive matches is never scored below this, whatever the characters sit
/// next to.
const MIN_CONSECUTIVE_BONUS: i64 = 4;

/// Highest score [`Matcher::fuzzy_match`] can return for a needle of `chars`
/// characters, when its first character earns at most `first` bonus and every
/// other one at most `rest_bonus`.
///
/// The matcher scores a match as one [`SCORE_MATCH`] per needle character plus
/// that character's bonus, the first character's bonus counting twice, and
/// unmatched characters in between only ever subtract. Bounding the bonuses
/// therefore bounds the score, which is what lets the scan drop an item that
/// cannot reach the shortlist without matching it.
fn name_score_ceiling(chars: usize, first_bonus: i64, rest_bonus: i64) -> i32 {
    if chars == 0 {
        // An empty needle matches everything, and scores nothing.
        return 0;
    }
    let rest = chars as i64 - 1;
    let ceiling = SCORE_MATCH
        + first_bonus * FIRST_CHAR_BONUS_MULTIPLIER
        + rest.saturating_mul(SCORE_MATCH + rest_bonus);
    // The matcher returns a `u16`, so nothing can score past its range.
    ceiling.min(u16::MAX as i64) as i32
}

/// What a name match is bounded by, given what the item's boundary mask says
/// about the needle's characters.
///
/// A character only earns a bonus where it sits next to the right neighbour: at
/// the start of a word, on a case or digit transition, or on a character that is
/// not a word character at all. [`name_masks`] records which of a name's
/// characters ever do, so a needle whose characters never sit anywhere like that
/// in this name is held to a much lower bound than the needle's length alone
/// gives — which is what a one or two character needle needs, since there the
/// length bound is barely above the score a real match comes out with and never
/// rules anything out.
///
/// The three bounds, and why they hold for every path the matcher can take:
///
/// * the first needle character scores [`SCORE_MATCH`] plus twice its own
///   bonus, so a needle whose first character earns nothing loses the whole
///   doubled term;
/// * every later character scores [`SCORE_MATCH`] plus a bonus that is either
///   its own, one inherited from an earlier character of the same needle — the
///   matcher only ever carries a bonus forward within a run — or
///   [`MIN_CONSECUTIVE_BONUS`], so if no needle character earns a bonus in this
///   name, no character of the match is scored above the consecutive floor;
/// * unmatched characters in between only ever subtract.
///
/// `test_name_ceilings_bound_every_match` and
/// `test_boundary_mask_agrees_with_the_matcher` keep this honest against the
/// matcher itself.
#[derive(Clone, Copy)]
struct NameCeilings {
    /// Bit of the needle's first character.
    first_char: u32,
    /// Bits of every character of the needle.
    all_chars: u32,
    /// Bound that holds against any name at all, which is also the one a name
    /// where the first needle character can earn a bonus holds to. What the
    /// engine bounded every match by before the boundary mask existed, and
    /// still the first thing the scan tries, since it needs nothing from the
    /// item.
    by_length: i32,
    /// Bound when only a later needle character can earn a bonus.
    trailing_bonus: i32,
    /// Bound when none of them can.
    no_bonus: i32,
}

impl NameCeilings {
    fn new(chars: usize, masks: QueryMasks) -> Self {
        Self {
            first_char: masks.first_char,
            all_chars: masks.all_chars,
            by_length: name_score_ceiling(chars, MAX_CHAR_BONUS, MAX_CHAR_BONUS),
            trailing_bonus: name_score_ceiling(chars, 0, MAX_CHAR_BONUS),
            no_bonus: name_score_ceiling(chars, 0, MIN_CONSECUTIVE_BONUS),
        }
    }

    /// The bound a name whose characters earn a bonus on `boundary` holds to.
    fn of(&self, boundary: u32) -> i32 {
        if self.first_char & boundary != 0 {
            self.by_length
        } else if self.all_chars & boundary != 0 {
            self.trailing_bonus
        } else {
            self.no_bonus
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppItem, LaunchTarget};

    fn sample_index() -> Index<AppItem> {
        let mut index = Index::new();
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

    fn titles(results: &[Match<AppItem>]) -> Vec<&str> {
        results.iter().map(|r| r.item.name()).collect()
    }

    fn named(results: &[Match<AppItem>]) -> Vec<String> {
        results.iter().map(|r| r.item.name().to_string()).collect()
    }

    #[test]
    fn test_exact_and_prefix_search() {
        let index = sample_index();

        let results = index.find("calc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].item.name(), "Calculator");

        let results = index.find("not", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].item.name(), "Notepad");
    }

    #[test]
    fn test_acronym_search() {
        let index = sample_index();

        let results = index.find("vsc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].item.name(), "Visual Studio Code");

        let results = index.find("gc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].item.name(), "Google Chrome");
    }

    #[test]
    fn test_keyword_score_not_shadowed_by_weak_name_match() {
        let mut index = Index::new();
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
        let mut index = Index::new();
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
        assert_eq!(results[0].item.name(), "Google Chrome");
    }

    #[test]
    fn test_no_match_is_excluded_even_with_partial_letters() {
        let mut index = Index::new();
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
        let mut index = Index::new();
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
        let mut index = Index::new();
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
        let mut index = Index::new();
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
        let mut index = Index::new();
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
        let popular =
            AppItem::new("a", "Test App", LaunchTarget::Path("a.exe".into())).with_launch_count(10);
        let rare = AppItem::new("b", "Test App", LaunchTarget::Path("b.exe".into()));

        let mut index = Index::new();
        index.set_items(vec![rare, popular]);

        let results = index.find("test app", 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].item.id(), "a");
    }

    #[test]
    fn test_frecency_applies_to_keyword_matches_too() {
        let popular = AppItem::new("a", "Aardvark Tool", LaunchTarget::Path("a.exe".into()))
            .with_keywords(vec!["zzzmatch".into()])
            .with_launch_count(10);
        let rare = AppItem::new("b", "Yak Tool", LaunchTarget::Path("b.exe".into()))
            .with_keywords(vec!["zzzmatch".into()]);

        let mut index = Index::new();
        index.set_items(vec![rare, popular]);

        let results = index.find("zzzmatch", 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].item.name(), "Aardvark Tool");
    }

    #[test]
    fn test_keyword_matching_normalizes_case_at_construction_time() {
        let mut index = Index::new();
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
        assert_eq!(results[0].item.name(), "Google Chrome");
    }

    #[test]
    fn test_unicode_names_match_case_insensitively_with_correct_indices() {
        let mut index = Index::new();
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
        assert_eq!(results[0].item.name(), "Café");
        assert_eq!(results[0].matched_char_indices, vec![0, 1, 2]);

        let results = index.find("アプリ", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.name(), "日本語アプリ");
        assert_eq!(results[0].matched_char_indices, vec![3, 4, 5]);
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
        let mut index = Index::new();
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
        let mut index = Index::new();
        index.set_items(vec![AppItem::new(
            "istanbul",
            "İstanbul Maps",
            LaunchTarget::Path("istanbul.exe".into()),
        )]);

        let results = index.find("İ", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.name(), "İstanbul Maps");
        assert_eq!(results[0].matched_char_indices, vec![0]);
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
        fn large_index() -> Index<AppItem> {
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
            let mut index = Index::new();
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

    /// Queries that share no prefix cannot narrow one another, so each one
    /// rescans the index while reusing what the last one left behind: the
    /// matcher's buffers and the set it recorded its matches in.
    #[test]
    fn test_unrelated_queries_in_one_session_match_cold_searches() {
        let session = ["visual", "café", "browser", "qqq", "", "windows t", "c"];
        let reused = sample_index();
        for query in session {
            let cold = sample_index();
            assert_eq!(
                titles(&reused.search(query, 5)),
                titles(&cold.search(query, 5)),
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

        let mut index = Index::new();
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
                .map(|r| r.item.name().to_string())
                .collect();
            got.sort();
            assert_eq!(got, reference_titles(&items, query), "query: {query:?}");
        }
    }

    /// The masks tell letters, digits, whitespace and everything else apart, so
    /// a needle character of one group no longer claims the bit of another. The
    /// groups are what the test above checks by hand; this one checks them
    /// against the matcher over shapes nobody wrote down, since a mask that
    /// rules out a real match is a result the engine loses.
    #[test]
    fn test_masks_never_drop_a_match_over_random_input() {
        let names = random_strings(64, 12, 0x5eed);
        let keywords = random_strings(64, 6, 0xc0ffee);
        let items: Vec<AppItem> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                AppItem::new(
                    format!("id-{i}"),
                    name.as_str(),
                    LaunchTarget::Path(format!("{i}.exe")),
                )
                .with_keywords(vec![
                    keywords[i].clone(),
                    keywords[(i + 1) % keywords.len()].clone(),
                ])
            })
            .collect();

        let mut index = Index::new();
        index.set_items(items.clone());

        for query in random_strings(256, 4, 0xbeef) {
            let mut got: Vec<String> = index
                .find(&query, usize::MAX)
                .into_iter()
                .map(|r| r.item.name().to_string())
                .collect();
            got.sort();
            assert_eq!(got, reference_titles(&items, &query), "query: {query:?}");
        }
    }

    /// Deterministic pseudo random strings, so the bound is checked against
    /// shapes nobody thought to write down: whitespace and delimiter
    /// boundaries, camel case, digits, non-word characters and accents.
    const ALPHABET: [char; 12] = [' ', '/', ';', '|', '-', 'a', 'b', 'A', 'B', '1', '2', 'é'];

    fn random_strings(count: usize, max_len: usize, seed: u64) -> Vec<String> {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (state >> 33) as usize
        };
        (0..count)
            .map(|_| {
                let len = next() % max_len + 1;
                (0..len)
                    .map(|_| ALPHABET[next() % ALPHABET.len()])
                    .collect()
            })
            .collect()
    }

    /// The three fields a prefilter word packs have to stay out of each
    /// other's way: either mask tests on its own, and the name length decides
    /// the ordering against [`name_len_floor`] whatever the masks hold.
    #[test]
    fn test_prefilter_word_fields_do_not_bleed_into_each_other() {
        let full = u32::MAX >> (u32::BITS - MASK_BITS);
        let word = prefilter_word(full, 0, 4);
        assert_eq!(u64::from(full) & !word, 0, "the name mask is not readable");
        assert_ne!(
            u64::from(full) << KEYWORD_MASK_SHIFT & !word,
            0,
            "the name mask leaked into the keyword field"
        );

        let word = prefilter_word(full, full, 4);
        assert!(word >= name_len_floor(4));
        assert!(word < name_len_floor(5), "the masks outweighed the length");

        // A name too long to pack saturates, and a needle that long stops
        // comparing rather than ruling the name out.
        let word = prefilter_word(0, 0, MAX_PACKED_NAME_LEN as usize + 1);
        assert!(word >= name_len_floor(MAX_PACKED_NAME_LEN as usize));
        assert_eq!(name_len_floor(MAX_PACKED_NAME_LEN as usize + 1), 0);
    }

    /// The separator the keywords are written between is not part of any of
    /// them, so it must not reach their mask: it would set the catch-all bit
    /// and let every query holding a digit, a space or any punctuation past the
    /// keyword test.
    #[test]
    fn test_keyword_mask_holds_only_what_a_keyword_can_match() {
        let mut table = ScanTable::default();
        table.push(
            &AppItem::new("id", "Name", LaunchTarget::Path("n.exe".into()))
                .with_keywords(vec!["tool".into(), "utility".into()]),
        );

        let keyword_mask = (table.prefilter[0] >> KEYWORD_MASK_SHIFT) as u32 & ALL_CHARS;
        assert_eq!(
            keyword_mask & OTHER_CHAR_BIT,
            0,
            "the separator reached the keyword mask"
        );
        assert_eq!(keyword_mask & SPACE_CHAR_BIT, 0);
        assert_ne!(keyword_mask & ascii_bit(b't'), 0);
        assert_eq!(keyword_mask & ascii_bit(b'z'), 0);
    }

    /// A character mask groups what a name holds, and the groups have to stay
    /// out of each other's way: an expression is ruled out of a name that has
    /// no punctuation precisely because its `*` claims neither the bit of the
    /// digits it sits between nor the one of the spaces around it.
    #[test]
    fn test_character_groups_claim_distinct_bits() {
        let groups = [
            ascii_bit(b'a'),
            ascii_bit(b'z'),
            DIGIT_CHAR_BIT,
            SPACE_CHAR_BIT,
            OTHER_CHAR_BIT,
        ];
        for (nth, bit) in groups.iter().enumerate() {
            assert_eq!(bit.count_ones(), 1);
            assert_eq!(bit & ALL_CHARS, *bit, "the bit is outside a mask");
            for other in &groups[nth + 1..] {
                assert_eq!(bit & other, 0, "two groups share a bit");
            }
        }

        assert_eq!(ascii_bit(b'A'), ascii_bit(b'a'));
        assert_eq!(ascii_bit(b'7'), DIGIT_CHAR_BIT);
        assert_eq!(ascii_bit(b'\t'), SPACE_CHAR_BIT);
        assert_eq!(ascii_bit(b'*'), OTHER_CHAR_BIT);
        assert_eq!(char_bit('é'), OTHER_CHAR_BIT);

        // What the masks leave of a prefilter word still has to hold the length
        // of the names a Start menu holds.
        const { assert!(MAX_PACKED_NAME_LEN >= 63) };
    }

    /// The subsequence test compares a haystack byte by folding it with the
    /// case bit the needle byte asks for, instead of lowercasing it. Over every
    /// byte pair, that has to be the same comparison.
    #[test]
    fn test_case_folded_comparison_matches_lowercasing() {
        for wanted in 0..=u8::MAX {
            if !wanted.is_ascii() || wanted != wanted.to_ascii_lowercase() {
                // Needles reaching the test are ASCII and already lowercase.
                continue;
            }
            let fold = case_fold_bit(wanted);
            for byte in 0..=u8::MAX {
                assert_eq!(
                    byte | fold == wanted,
                    byte.to_ascii_lowercase() == wanted,
                    "byte {byte:#04x} against needle byte {wanted:#04x}"
                );
            }
        }
    }

    #[test]
    fn test_name_ceilings_bound_every_match() {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        let mut matcher = Matcher::new(config);

        let mut names = random_strings(400, 16, 0x5eed);
        names.extend(random_strings(200, 4, 0xbeef));
        names.push("Visual Studio Code".into());
        let mut needles = random_strings(80, 5, 0xd00d);
        // Every one and two character needle over the alphabet the names are
        // drawn from, which is where the bound has to be tightest and where a
        // single character out of place would show.
        for first in ALPHABET {
            needles.push(first.to_string());
            for second in ALPHABET {
                needles.push(format!("{first}{second}"));
            }
        }
        needles.push("visual studio".into());

        let mut hay_buf = Vec::new();
        let mut needle_buf = Vec::new();
        for needle in &needles {
            let normalized: String = needle.chars().map(normalize).map(to_lower_case).collect();
            let masks = needle_masks(&normalized);
            let ceilings = NameCeilings::new(normalized.chars().count(), masks);
            for name in &names {
                let (_, boundary, _) = name_masks(name);
                let ceiling = ceilings.of(boundary);
                hay_buf.clear();
                needle_buf.clear();
                let haystack = Utf32Str::new(name, &mut hay_buf);
                let needle = Utf32Str::new(&normalized, &mut needle_buf);
                let Some(score) = matcher.fuzzy_match(haystack, needle) else {
                    continue;
                };
                assert!(
                    i32::from(score) <= ceiling,
                    "{score} beats the ceiling {ceiling} for {normalized:?} in {name:?}"
                );
            }
        }
    }

    /// The boundary mask claims to hold exactly the characters of a name that
    /// the matcher gives a bonus to. A one character needle scores
    /// `SCORE_MATCH + 2 * bonus` at its best position, which is how the bonus
    /// the matcher actually gave can be read back out of it.
    #[test]
    fn test_boundary_mask_agrees_with_the_matcher() {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        let mut matcher = Matcher::new(config);

        let mut names = random_strings(400, 16, 0x0b0e);
        names.extend(["A", " a", "aA", "a1", "1a", "a/b", "a-b", "a b", "aa"].map(String::from));

        let mut hay_buf = Vec::new();
        let mut needle_buf = Vec::new();
        for name in &names {
            let (present, boundary, is_ascii) = name_masks(name);
            if !is_ascii {
                continue;
            }
            for byte in 0..=127u8 {
                let ch = byte as char;
                let needle_string = ch.to_lowercase().to_string();
                hay_buf.clear();
                needle_buf.clear();
                let haystack = Utf32Str::new(name, &mut hay_buf);
                let needle = Utf32Str::new(&needle_string, &mut needle_buf);
                let Some(score) = matcher.fuzzy_match(haystack, needle) else {
                    continue;
                };
                let bit = ascii_bit(byte);
                assert_ne!(present & bit, 0, "{ch:?} missing from {name:?}'s mask");
                let bonus = (i64::from(score) - SCORE_MATCH) / FIRST_CHAR_BONUS_MULTIPLIER;
                assert!(bonus <= MAX_CHAR_BONUS, "{bonus} beats the highest bonus");
                if bonus > 0 {
                    assert_ne!(
                        boundary & bit,
                        0,
                        "{ch:?} earns {bonus} in {name:?} but is not a boundary character"
                    );
                }
                // Letters have a bit to themselves, so for them the mask is not
                // just sound but exact.
                if byte.is_ascii_alphabetic() {
                    assert_eq!(
                        boundary & bit != 0,
                        bonus > 0,
                        "{ch:?} in {name:?} scored {score}"
                    );
                }
            }
        }
    }

    /// The boundary mask rides in the padding a row had anyway. If the row ever
    /// grows past that, the scan pays for it on every item it looks at, so the
    /// size is worth pinning.
    #[test]
    fn test_a_scan_row_still_fits_six_words() {
        assert_eq!(size_of::<ScanRow>(), 6 * size_of::<u32>());
        assert_eq!(NAME_IS_ASCII_BIT & ALL_CHARS, 0);
    }

    /// A name that is not ASCII is not held to a tightened bound, so its mask
    /// has to claim every character.
    #[test]
    fn test_non_ascii_names_keep_the_untightened_bound() {
        let (_, boundary, is_ascii) = name_masks("Ünïcöde Viewer");
        assert!(!is_ascii);
        assert_eq!(boundary, ALL_CHARS);

        let masks = needle_masks("uv");
        let ceilings = NameCeilings::new(2, masks);
        assert_eq!(ceilings.of(boundary), ceilings.by_length);
        assert_eq!(
            ceilings.by_length,
            name_score_ceiling(2, MAX_CHAR_BONUS, MAX_CHAR_BONUS)
        );
    }

    #[test]
    fn test_score_ceiling_never_prunes_a_result_a_full_scan_would_keep() {
        // Wider than the match set the engine is willing to keep, so the scan
        // gives up narrowing and the ceiling starts dropping items.
        let items: Vec<AppItem> = (0..800u32)
            .map(|i| {
                let name = match i % 5 {
                    0 => format!("Visual Studio {i}"),
                    1 => format!("Video Ace {i}"),
                    2 => format!("visual-basic-{i}"),
                    3 => format!("Vidéo Aperçu {i}"),
                    _ => format!("aVi{i} Viewer"),
                };
                AppItem::new(
                    format!("id-{i}"),
                    name,
                    LaunchTarget::Path(format!("{i}.exe")),
                )
                .with_keywords(vec!["tool".into()])
                // Frecency climbs with the item index, so the results a full
                // scan keeps are the ones a scan that prunes too eagerly would
                // never reach.
                .with_launch_count(i / 100)
            })
            .collect();

        let mut typed = Index::new();
        typed.set_items(items.clone());

        let queries = [
            "v", "vi", "vis", "visu", "visual", "visual s", "a", "tool", "vidé", "aperçu", "é",
        ];
        for query in queries {
            let expected = reference_ranking(&items, query, 6);
            // Cold every time as well: a narrowed scan is a different code
            // path, and the ranking has to come out the same on both.
            let mut cold = Index::new();
            cold.set_items(items.clone());
            assert_eq!(named(&cold.find(query, 6)), expected, "cold: {query:?}");
            assert_eq!(named(&typed.find(query, 6)), expected, "typed: {query:?}");
        }
    }

    /// The `limit` best matches for `query`, scored the way
    /// [`ScanTable::scan`] scores them but without any prefilter or bound, and
    /// ordered the way [`keep_best`] orders them.
    fn reference_ranking(items: &[AppItem], query: &str, limit: usize) -> Vec<String> {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        let mut matcher = Matcher::new(config);
        let needle_string: String = query.chars().map(normalize).map(to_lower_case).collect();
        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(&needle_string, &mut needle_buf);
        let query_lower = query.to_lowercase();
        let mut hay_buf = Vec::new();

        let mut scored: Vec<(i32, usize, &str)> = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            hay_buf.clear();
            let haystack = Utf32Str::new(item.name(), &mut hay_buf);
            let by_name = matcher.fuzzy_match(haystack, needle).map(i32::from);
            let by_keyword = item
                .keywords()
                .iter()
                .any(|keyword| keyword.contains(&query_lower))
                .then_some(KEYWORD_MATCH_SCORE);
            let Some(score) = by_name.max(by_keyword) else {
                continue;
            };
            let score = score.saturating_add(launch_score(
                item.launch_count(),
                SEARCH_FRECENCY_MULTIPLIER,
            ));
            scored.push((score, idx, item.name()));
        }

        scored.sort_by_key(|&(score, idx, _)| (std::cmp::Reverse(score), idx));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, _, name)| name.to_string())
            .collect()
    }

    #[test]
    fn test_empty_query_lists_top_items() {
        let mut index = Index::new();
        let popular = AppItem::new("a", "Popular App", LaunchTarget::Path("a.exe".into()))
            .with_launch_count(10);
        index.set_items(vec![popular]);

        let results = index.search("", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.name(), "Popular App");
    }

    #[test]
    fn test_zero_limit_yields_no_results_for_top_items_and_a_search() {
        let mut index = Index::new();
        index.set_items(vec![AppItem::new(
            "a",
            "Calculator",
            LaunchTarget::Path("calc.exe".into()),
        )]);

        assert!(index.search("", 0).is_empty());
        assert!(index.search("calc", 0).is_empty());
    }
}
