mod live;

use winsp_core::index::Match;
use winsp_core::models::{AppItem, SearchResult};

use crate::state::Catalog;
use live::{CalcSource, LiveSource};

const LIVE_SOURCES: &[&dyn LiveSource] = &[&CalcSource];

/// `matches` is scratch space reused across calls, the same way `out` is: the
/// index's own results land there first and are converted into `out`, so a
/// keystroke costs no allocation beyond what the index and the live sources
/// themselves need.
pub fn query(
    index: &Catalog,
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
    use winsp_core::models::SearchResultKind;

    #[test]
    fn a_calculation_takes_the_top_result() {
        let index = Catalog::new();
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
        let index = Catalog::new();
        let mut matches = Vec::new();
        let mut results = Vec::new();

        query(&index, "notepad", 5, &mut matches, &mut results);

        assert!(
            !results
                .iter()
                .any(|r| matches!(r.kind, SearchResultKind::Calculation { .. }))
        );
    }

    #[test]
    fn an_empty_query_yields_no_calculation_result() {
        let index = Catalog::new();
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
