mod index;
mod math;

use crate::models::SearchResult;

pub use index::Engine;

impl Engine {
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.top_items(limit);
        }

        let calc_result =
            math::try_eval(trimmed).map(|res| SearchResult::calculation(trimmed.to_string(), res));

        let mut results = self.find(trimmed, limit);
        results.extend(calc_result);

        results.sort_by_key(|r| std::cmp::Reverse(r.score));
        results.truncate(limit);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppItem, LaunchTarget};

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
        let mut popular = AppItem::new("a", "Popular App", LaunchTarget::Path("a.exe".into()));
        popular.launch_count = 10;
        index.set_items(vec![popular]);

        let results = index.search("", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_ref(), "Popular App");
    }
}
