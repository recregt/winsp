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
    use winsp_core::models::SearchResultKind;

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
