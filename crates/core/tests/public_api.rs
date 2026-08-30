use winsp_core::{AppItem, AppTarget, SearchIndex, SearchResultKind};

#[test]
fn test_full_flow_from_construction_to_search_result() {
    let mut index = SearchIndex::new();
    index.set_items(vec![
        AppItem::new("notepad", "Notepad", AppTarget::Path("notepad.exe".into())),
        AppItem::new(
            "calc",
            "Calculator",
            AppTarget::Aumid("Microsoft.WindowsCalculator".into()),
        ),
        AppItem::new(
            "display",
            "Display Settings",
            AppTarget::SettingUri("ms-settings:display".into()),
        )
        .with_description("Change your display resolution")
        .with_keywords(vec!["screen".into(), "monitor".into()]),
        AppItem::new(
            "shutdown",
            "Shut Down",
            AppTarget::SystemCommand("shutdown /s /t 0".into()),
        ),
    ]);

    let results = index.search("notepad", 5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title.as_ref(), "Notepad");
    assert_eq!(results[0].subtitle.as_deref(), Some("notepad.exe"));
    let SearchResultKind::App(item) = &results[0].kind else {
        panic!("expected an App result");
    };
    assert_eq!(item.id, "notepad");

    let results = index.search("calc", 5);
    assert_eq!(
        results[0].subtitle.as_deref(),
        Some("Store App: Microsoft.WindowsCalculator")
    );

    let results = index.search("display", 5);
    assert_eq!(
        results[0].subtitle.as_deref(),
        Some("Change your display resolution")
    );

    let results = index.search("shutdown", 5);
    assert_eq!(
        results[0].subtitle.as_deref(),
        Some("System: shutdown /s /t 0")
    );
}

#[test]
fn test_math_calculation_reachable_through_public_search() {
    let index = SearchIndex::new();

    let results = index.search("12 * 12", 5);
    assert!(!results.is_empty());
    assert_eq!(results[0].title.as_ref(), "144");
    let SearchResultKind::Calculation { expression, result } = &results[0].kind else {
        panic!("expected a Calculation result");
    };
    assert_eq!(expression, "12 * 12");
    assert_eq!(result, "144");
}

#[test]
fn test_search_result_kind_is_exhaustively_matchable() {
    let index = SearchIndex::new();
    let results = index.search("2+2", 5);
    let kind = &results[0].kind;

    let description = match kind {
        SearchResultKind::App(item) => item.name.to_string(),
        SearchResultKind::Calculation { result, .. } => result.clone(),
        SearchResultKind::WebSearch { query, .. } => query.clone(),
        SearchResultKind::SystemCommand { command, .. } => command.clone(),
    };
    assert_eq!(description, "4");
}
