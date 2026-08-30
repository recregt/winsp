use crate::evaluator::Evaluator;
use crate::models::{AppItem, SearchResult};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::cell::RefCell;
use std::sync::Arc;

/// In-memory search index holding pre-indexed application items.
pub struct SearchIndex {
    items: Vec<Arc<AppItem>>,
    matcher: RefCell<Matcher>,
}

impl std::fmt::Debug for SearchIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchIndex")
            .field("items", &self.items.len())
            .finish()
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.ignore_case = true;
        Self {
            items: Vec::new(),
            matcher: RefCell::new(Matcher::new(config)),
        }
    }

    pub fn set_items(&mut self, items: Vec<AppItem>) {
        self.items = items.into_iter().map(Arc::new).collect();
    }

    pub fn add_item(&mut self, item: AppItem) {
        self.items.push(Arc::new(item));
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Searches the index with the given query.
    /// Returns sorted search results (highest score first).
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            let mut top: Vec<(Arc<AppItem>, i32)> = self
                .items
                .iter()
                .map(|item| (Arc::clone(item), (item.launch_count as i32) * 10))
                .collect();
            top.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
            top.truncate(limit);
            return top
                .into_iter()
                .map(|(item, score)| SearchResult::from_app(item, score, Vec::new()))
                .collect();
        }

        let calc_result = Evaluator::try_eval(trimmed_query)
            .map(|calc_res| SearchResult::calculation(trimmed_query.to_string(), calc_res));

        let mut candidates = match_all_items(&self.items, &self.matcher, trimmed_query);
        candidates.sort_by_key(|(_, score, _)| std::cmp::Reverse(*score));
        candidates.truncate(limit);

        let mut results: Vec<SearchResult> = candidates
            .into_iter()
            .map(|(item, score, indices)| SearchResult::from_app(item, score, indices))
            .collect();
        results.extend(calc_result);

        results.sort_by_key(|r| std::cmp::Reverse(r.score));
        results.truncate(limit);
        results
    }
}

fn match_all_items(
    items: &[Arc<AppItem>],
    matcher: &RefCell<Matcher>,
    query: &str,
) -> Vec<(Arc<AppItem>, i32, Vec<usize>)> {
    let mut matcher = matcher.borrow_mut();
    let query_lower = query.to_lowercase();
    let mut needle_buf = Vec::new();
    let needle = Utf32Str::new(&query_lower, &mut needle_buf);

    let mut candidates = Vec::new();
    let mut hay_buf = Vec::new();
    let mut raw_indices = Vec::new();

    for item in items {
        hay_buf.clear();
        let haystack = Utf32Str::new(&item.name, &mut hay_buf);
        raw_indices.clear();

        let frecency_boost = (item.launch_count as i32) * 50;

        if let Some(score) = matcher.fuzzy_indices(haystack, needle, &mut raw_indices) {
            let indices = raw_indices.iter().map(|&i| i as usize).collect();
            candidates.push((Arc::clone(item), score as i32 + frecency_boost, indices));
        } else if let Some(kw_score) = match_keywords(&item.keywords, &query_lower) {
            candidates.push((Arc::clone(item), kw_score + frecency_boost, Vec::new()));
        }
    }

    candidates
}

fn match_keywords(keywords: &[String], query_lower: &str) -> Option<i32> {
    keywords
        .iter()
        .any(|kw| kw.starts_with(query_lower) || kw.contains(query_lower))
        .then_some(5_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppTarget, SearchResultKind};

    fn sample_index() -> SearchIndex {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new("notepad", "Notepad", AppTarget::Path("notepad.exe".into())),
            AppItem::new(
                "vscode",
                "Visual Studio Code",
                AppTarget::Path("code.exe".into()),
            ),
            AppItem::new(
                "calc",
                "Calculator",
                AppTarget::Aumid("Microsoft.WindowsCalculator".into()),
            ),
            AppItem::new(
                "terminal",
                "Windows Terminal",
                AppTarget::Path("wt.exe".into()),
            ),
            AppItem::new(
                "chrome",
                "Google Chrome",
                AppTarget::Path("chrome.exe".into()),
            )
            .with_keywords(vec!["browser".into(), "web".into(), "internet".into()]),
            AppItem::new(
                "settings",
                "Windows Settings",
                AppTarget::SettingUri("ms-settings:".into()),
            ),
        ]);
        index
    }

    #[test]
    fn test_exact_and_prefix_search() {
        let index = sample_index();

        let results = index.search("calc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Calculator");

        let results = index.search("not", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Notepad");
    }

    #[test]
    fn test_acronym_search() {
        let index = sample_index();

        // "vsc" -> Visual Studio Code
        let results = index.search("vsc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Visual Studio Code");

        // "gc" -> Google Chrome
        let results = index.search("gc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Google Chrome");
    }

    #[test]
    fn test_keyword_search() {
        let index = sample_index();

        let results = index.search("browser", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "Google Chrome");
    }

    #[test]
    fn test_math_in_search() {
        let index = sample_index();

        let results = index.search("25 * 4", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "100");
    }

    fn titles(results: &[SearchResult]) -> Vec<&str> {
        results.iter().map(|r| r.title.as_ref()).collect()
    }

    #[test]
    fn test_no_match_is_excluded_even_with_partial_letters() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new("chrome", "Chrome", AppTarget::Path("chrome.exe".into())),
            AppItem::new(
                "google-chrome",
                "Google Chrome",
                AppTarget::Path("chrome.exe".into()),
            ),
            AppItem::new(
                "chromium",
                "Chromium",
                AppTarget::Path("chromium.exe".into()),
            ),
            AppItem::new(
                "chrome-devtools",
                "Chrome DevTools",
                AppTarget::Path("chrome.exe".into()),
            ),
        ]);

        let results = index.search("chrome", 10);
        assert!(!titles(&results).contains(&"Chromium"));
        assert!(titles(&results).contains(&"Chrome"));
        assert!(titles(&results).contains(&"Google Chrome"));
        assert!(titles(&results).contains(&"Chrome DevTools"));
    }

    #[test]
    fn test_prefix_outranks_acronym() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new("vscode", "VS Code", AppTarget::Path("code.exe".into())),
            AppItem::new(
                "vstudio",
                "Visual Studio",
                AppTarget::Path("devenv.exe".into()),
            ),
        ]);

        let results = index.search("vs", 5);
        assert_eq!(titles(&results), vec!["VS Code", "Visual Studio"]);
    }

    #[test]
    fn test_acronym_outranks_substring() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new(
                "open-office-go",
                "Open Office Go",
                AppTarget::Path("oog.exe".into()),
            ),
            AppItem::new("google", "Google", AppTarget::Path("chrome.exe".into())),
        ]);

        let results = index.search("oog", 5);
        assert_eq!(titles(&results), vec!["Open Office Go", "Google"]);
    }

    #[test]
    fn test_word_start_bonus_can_outrank_a_midword_substring() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new("notepad", "Notepad", AppTarget::Path("notepad.exe".into())),
            AppItem::new(
                "paint-design",
                "Paint Design",
                AppTarget::Path("paint.exe".into()),
            ),
        ]);

        let results = index.search("pad", 5);
        assert_eq!(titles(&results), vec!["Paint Design", "Notepad"]);
    }

    #[test]
    fn test_keyword_match_outranks_a_weak_fuzzy_name_match() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new(
                "rand-setup",
                "Random Windows Setup",
                AppTarget::Path("setup.exe".into()),
            ),
            AppItem::new(
                "chrome",
                "Google Chrome",
                AppTarget::Path("chrome.exe".into()),
            )
            .with_keywords(vec!["browser".into()]),
        ]);

        let results = index.search("rows", 5);
        assert_eq!(
            titles(&results),
            vec!["Google Chrome", "Random Windows Setup"]
        );
    }

    #[test]
    fn test_frecency_breaks_ties_between_identical_names() {
        let mut popular = AppItem::new("a", "Test App", AppTarget::Path("a.exe".into()));
        popular.launch_count = 10;
        let rare = AppItem::new("b", "Test App", AppTarget::Path("b.exe".into()));

        let mut index = SearchIndex::new();
        index.set_items(vec![rare, popular]);

        let results = index.search("test app", 5);
        assert_eq!(results.len(), 2);
        let SearchResultKind::App(item) = &results[0].kind else {
            panic!("expected an App result");
        };
        assert_eq!(item.id, "a");
    }

    #[test]
    fn test_unicode_names_match_case_insensitively_with_correct_indices() {
        let mut index = SearchIndex::new();
        index.set_items(vec![
            AppItem::new("cafe", "Café", AppTarget::Path("cafe.exe".into())),
            AppItem::new(
                "nihongo",
                "日本語アプリ",
                AppTarget::Path("nihongo.exe".into()),
            ),
        ]);

        let results = index.search("CAF", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "Café");
        assert_eq!(results[0].matched_indices, vec![0, 1, 2]);

        let results = index.search("アプリ", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "日本語アプリ");
        assert_eq!(results[0].matched_indices, vec![3, 4, 5]);
    }
}
