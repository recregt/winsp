use compact_str::CompactString;
use winsp_core::calc;
use winsp_core::index::{Index, Match};
use winsp_core::models::{AppItem, SearchResult};

trait LiveSource {
    fn query(&self, input: &str) -> Option<SearchResult>;
}

struct CalcSource;

impl LiveSource for CalcSource {
    fn query(&self, input: &str) -> Option<SearchResult> {
        let result = calc::eval(input)?;
        Some(SearchResult::calculation(CompactString::new(input), result))
    }
}

const LIVE_SOURCES: &[&dyn LiveSource] = &[&CalcSource];

pub fn query(
    index: &Index<AppItem>,
    input: &str,
    limit: usize,
    matches: &mut Vec<Match<AppItem>>,
    out: &mut Vec<SearchResult>,
) {
    index.search_into(input, limit, matches);
    out.clear();
    out.extend(
        matches
            .drain(..)
            .map(|m| SearchResult::from_app(m.item, m.score, m.matched_char_indices)),
    );

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return;
    }

    for source in LIVE_SOURCES {
        if let Some(result) = source.query(trimmed) {
            out.insert(0, result);
            out.truncate(limit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winsp_core::models::{LaunchTarget, SearchResultKind};

    #[test]
    fn a_calculation_takes_the_top_result() {
        let index = Index::new();
        let mut matches = Vec::new();
        let mut results = Vec::new();

        query(&index, "12 * 12", 5, &mut matches, &mut results);

        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_ref(), "144");
        let SearchResultKind::Calculation { expression, result } = &results[0].kind else {
            panic!("expected a Calculation result");
        };
        assert_eq!(expression, "12 * 12");
        assert_eq!(result, "144");
    }

    #[test]
    fn a_non_calculation_query_yields_no_calculation_result() {
        let index = Index::new();
        let mut matches = Vec::new();
        let mut results = Vec::new();

        query(&index, "notepad", 5, &mut matches, &mut results);

        assert!(
            !results
                .iter()
                .any(|r| matches!(r.kind, SearchResultKind::Calculation { .. }))
        );
    }

    /// The window shows `limit` rows, so a calculation costs the index its last
    /// one: the results before it are the ones the index ranked highest, in the
    /// order it ranked them.
    #[test]
    fn a_calculation_fills_the_window_ahead_of_the_index() {
        let mut index = Index::new();
        index.set_items((0..10).map(|i| {
            AppItem::new(
                format!("id-{i}"),
                format!("12 * 12 Manager {i}"),
                LaunchTarget::Path(format!("app{i}.exe")),
            )
        }));

        let mut matches = Vec::new();
        let mut results = Vec::new();
        query(&index, "12 * 12", 3, &mut matches, &mut results);

        assert_eq!(results.len(), 3);
        assert!(matches!(
            results[0].kind,
            SearchResultKind::Calculation { .. }
        ));

        let ranked = index.search("12 * 12", 3);
        assert!(ranked.len() >= 2);
        for (result, expected) in results[1..].iter().zip(&ranked) {
            assert_eq!(result.title, expected.item.name_arc());
            assert_eq!(result.score, expected.score);
        }
    }

    #[test]
    fn an_empty_query_yields_no_calculation_result() {
        let index = Index::new();
        let mut matches = Vec::new();
        let mut results = Vec::new();

        query(&index, "", 5, &mut matches, &mut results);

        assert!(
            !results
                .iter()
                .any(|r| matches!(r.kind, SearchResultKind::Calculation { .. }))
        );
    }
}
