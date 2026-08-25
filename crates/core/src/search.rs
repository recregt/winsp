use crate::evaluator::Evaluator;
use crate::models::{AppItem, SearchResult};
use std::sync::Arc;

/// In-memory search index holding pre-indexed application items.
#[derive(Debug, Default, Clone)]
pub struct SearchIndex {
    items: Vec<Arc<AppItem>>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self { items: Vec::new() }
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
            // Return top items by frecency / launch count
            let mut top_results: Vec<SearchResult> = self
                .items
                .iter()
                .map(|item| {
                    let score = (item.launch_count as i32) * 10;
                    SearchResult::from_app(Arc::clone(item), score, Vec::new())
                })
                .collect();
            top_results.sort_by_key(|b| std::cmp::Reverse(b.score));
            top_results.truncate(limit);
            return top_results;
        }

        let mut results = Vec::new();

        // 1. Check if the query is an instant mathematical expression
        if let Some(calc_res) = Evaluator::try_eval(trimmed_query) {
            results.push(SearchResult::calculation(
                trimmed_query.to_string(),
                calc_res,
            ));
        }

        // 2. Fuzzy match across all indexed items
        let query_lower = trimmed_query.to_lowercase();
        let query_chars: Vec<char> = query_lower.chars().collect();

        for item in &self.items {
            if let Some((score, indices)) = match_item(item, &query_lower, &query_chars) {
                // Apply frecency boost
                let frecency_boost = (item.launch_count as i32) * 50;
                let total_score = score + frecency_boost;
                results.push(SearchResult::from_app(
                    Arc::clone(item),
                    total_score,
                    indices,
                ));
            }
        }

        // Sort descending by score
        results.sort_by_key(|b| std::cmp::Reverse(b.score));
        results.truncate(limit);
        results
    }
}

/// Matches a query against an AppItem's name and keywords.
/// Returns Some((score, matched_indices_in_name)) on match.
fn match_item(
    item: &AppItem,
    query_lower: &str,
    query_chars: &[char],
) -> Option<(i32, Vec<usize>)> {
    let name_lower = item.name.to_lowercase();
    let name_chars: Vec<char> = name_lower.chars().collect();

    // 1. Exact Match
    if name_lower == query_lower {
        let indices = (0..name_chars.len()).collect();
        return Some((100_000, indices));
    }

    // 2. Exact Prefix Match
    if name_lower.starts_with(query_lower) {
        let indices = (0..query_chars.len()).collect();
        let score = 50_000 - (name_chars.len() as i32 * 10);
        return Some((score, indices));
    }

    // 3. Word Boundary / Acronym Match (e.g. "vs" -> "Visual Studio Code")
    if let Some((acronym_score, indices)) = match_acronym(&name_chars, query_chars) {
        return Some((acronym_score, indices));
    }

    // 4. Exact Substring Match
    if let Some(pos) = name_lower.find(query_lower) {
        let char_pos = name_lower[..pos].chars().count();
        let indices = (char_pos..char_pos + query_chars.len()).collect();
        let score = 20_000 - (char_pos as i32 * 50) - (name_chars.len() as i32 * 10);
        return Some((score, indices));
    }

    // 5. Fuzzy Subsequence Match
    if let Some((fuzzy_score, indices)) = fuzzy_match(&name_chars, query_chars) {
        return Some((fuzzy_score, indices));
    }

    // 6. Match against keywords as fallback
    for kw in &item.keywords {
        let kw_lower = kw.to_lowercase();
        if kw_lower.starts_with(query_lower) || kw_lower.contains(query_lower) {
            return Some((5_000, Vec::new()));
        }
    }

    None
}

/// Matches query against word boundaries / camelCase initials.
fn match_acronym(name_chars: &[char], query_chars: &[char]) -> Option<(i32, Vec<usize>)> {
    let mut q_idx = 0;
    let mut matched_indices = Vec::new();

    let mut is_boundary = true;
    for (i, &c) in name_chars.iter().enumerate() {
        if c.is_whitespace() || c == '-' || c == '_' || c == '.' {
            is_boundary = true;
            continue;
        }

        if is_boundary {
            if q_idx < query_chars.len() && c == query_chars[q_idx] {
                matched_indices.push(i);
                q_idx += 1;
                if q_idx == query_chars.len() {
                    let score = 30_000 - (i as i32 * 20);
                    return Some((score, matched_indices));
                }
            }
            is_boundary = false;
        }
    }

    None
}

/// Computes fuzzy subsequence match with distance penalty and consecutive match bonuses.
fn fuzzy_match(name_chars: &[char], query_chars: &[char]) -> Option<(i32, Vec<usize>)> {
    if query_chars.is_empty() || name_chars.is_empty() {
        return None;
    }

    let mut matched_indices = Vec::with_capacity(query_chars.len());
    let mut name_idx = 0;
    let mut score = 1_000;
    let mut prev_matched_idx: Option<usize> = None;

    for &q_char in query_chars {
        let mut found = false;
        while name_idx < name_chars.len() {
            let n_char = name_chars[name_idx];
            if n_char == q_char {
                matched_indices.push(name_idx);

                // Consecutive match bonus
                if let Some(prev) = prev_matched_idx {
                    if name_idx == prev + 1 {
                        score += 200; // Consecutive bonus
                    } else {
                        score -= ((name_idx - prev) as i32) * 20; // Distance penalty
                    }
                }

                // Word start bonus
                if name_idx == 0
                    || name_chars[name_idx - 1].is_whitespace()
                    || name_chars[name_idx - 1] == '-'
                {
                    score += 150;
                }

                prev_matched_idx = Some(name_idx);
                name_idx += 1;
                found = true;
                break;
            }
            name_idx += 1;
        }

        if !found {
            return None;
        }
    }

    // Penalize longer names for identical matches
    score -= (name_chars.len() as i32) * 5;

    Some((score.max(10), matched_indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppTarget;

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
        assert_eq!(results[0].title, "Calculator");

        let results = index.search("not", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Notepad");
    }

    #[test]
    fn test_acronym_search() {
        let index = sample_index();

        // "vsc" -> Visual Studio Code
        let results = index.search("vsc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Visual Studio Code");

        // "gc" -> Google Chrome
        let results = index.search("gc", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Google Chrome");
    }

    #[test]
    fn test_keyword_search() {
        let index = sample_index();

        let results = index.search("browser", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "Google Chrome");
    }

    #[test]
    fn test_math_in_search() {
        let index = sample_index();

        let results = index.search("25 * 4", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].title, "100");
    }
}
