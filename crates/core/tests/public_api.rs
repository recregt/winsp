use winsp_core::index::{Index, Match};
use winsp_core::models::{AppItem, LaunchTarget, SearchResult, SearchResultKind};

fn to_results(matches: Vec<Match<AppItem>>) -> Vec<SearchResult> {
    matches
        .into_iter()
        .map(|m| SearchResult::from_app(m.item, m.score, m.matched_char_indices))
        .collect()
}

#[test]
fn test_full_flow_from_construction_to_search_result() {
    let mut index = Index::new();
    index.set_items(vec![
        AppItem::new(
            "notepad",
            "Notepad",
            LaunchTarget::Path("notepad.exe".into()),
        ),
        AppItem::new(
            "calc",
            "Calculator",
            LaunchTarget::OsUri("shell:AppsFolder\\Microsoft.WindowsCalculator".into()),
        ),
        AppItem::new(
            "display",
            "Display Settings",
            LaunchTarget::OsUri("ms-settings:display".into()),
        )
        .with_description("Change your display resolution")
        .with_keywords(vec!["screen".into(), "monitor".into()]),
        AppItem::new(
            "shutdown",
            "Shut Down",
            LaunchTarget::Command("shutdown /s /t 0".into()),
        ),
    ]);

    let results = to_results(index.search("notepad", 5));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title.as_ref(), "Notepad");
    assert_eq!(results[0].subtitle.as_deref(), Some("notepad.exe"));
    let SearchResultKind::App(item) = &results[0].kind else {
        panic!("expected an App result");
    };
    assert_eq!(item.id(), "notepad");

    let results = to_results(index.search("calc", 5));
    assert_eq!(results[0].subtitle.as_deref(), None);

    let results = to_results(index.search("display", 5));
    assert_eq!(
        results[0].subtitle.as_deref(),
        Some("Change your display resolution")
    );

    let results = to_results(index.search("shutdown", 5));
    assert_eq!(results[0].subtitle.as_deref(), None);
}

#[test]
fn test_search_result_kind_is_exhaustively_matchable() {
    let result = SearchResult::calculation("2+2".into(), "4".into());
    let kind = &result.kind;

    let description = match kind {
        SearchResultKind::App(item) => item.name().to_string(),
        SearchResultKind::Calculation { result, .. } => result.to_string(),
        SearchResultKind::WebSearch { query, .. } => query.clone(),
        SearchResultKind::SystemCommand { command, .. } => command.clone(),
    };
    assert_eq!(description, "4");
}
