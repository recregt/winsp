mod live;

use winsp_core::engine::Engine;
use winsp_core::models::SearchResult;

use live::{CalcSource, LiveSource};

const LIVE_SOURCES: &[&dyn LiveSource] = &[&CalcSource];

pub fn query(engine: &Engine, input: &str, limit: usize, out: &mut Vec<SearchResult>) {
    engine.search_into(input, limit, out);

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
        let engine = Engine::new();
        let mut results = Vec::new();

        query(&engine, "12 * 12", 5, &mut results);

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
        let engine = Engine::new();
        let mut results = Vec::new();

        query(&engine, "notepad", 5, &mut results);

        assert!(
            !results
                .iter()
                .any(|r| matches!(r.kind, SearchResultKind::Calculation { .. }))
        );
    }

    #[test]
    fn an_empty_query_yields_no_calculation_result() {
        let engine = Engine::new();
        let mut results = Vec::new();

        query(&engine, "", 5, &mut results);

        assert!(
            !results
                .iter()
                .any(|r| matches!(r.kind, SearchResultKind::Calculation { .. }))
        );
    }
}
