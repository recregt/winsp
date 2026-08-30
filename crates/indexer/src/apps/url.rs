pub(super) fn resolve_url_target(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim().eq_ignore_ascii_case("URL") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}
