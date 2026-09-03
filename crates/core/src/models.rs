use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppTarget {
    Path(String),
    Aumid(String),
    Uri(String),
    SystemCommand(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconSource {
    Path(String),
    Glyph(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppItem {
    pub id: String,
    pub name: Arc<str>,
    pub description: Option<Arc<str>>,
    pub target: AppTarget,
    pub icon: Option<IconSource>,
    pub keywords: Vec<String>,
    pub launch_count: u32,
    pub last_launched_timestamp: u64,
}

impl AppItem {
    pub fn new(id: impl Into<String>, name: impl Into<Arc<str>>, target: AppTarget) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            target,
            icon: None,
            keywords: Vec::new(),
            launch_count: 0,
            last_launched_timestamp: 0,
        }
    }

    pub fn with_description(mut self, desc: impl Into<Arc<str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_icon(mut self, path: impl Into<String>) -> Self {
        self.icon = Some(IconSource::Path(path.into()));
        self
    }

    pub fn with_icon_glyph(mut self, glyph: char) -> Self {
        self.icon = Some(IconSource::Glyph(glyph));
        self
    }

    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords.into_iter().map(|kw| kw.to_lowercase()).collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchResultKind {
    App(Arc<AppItem>),
    Calculation {
        expression: String,
        result: String,
    },
    WebSearch {
        query: String,
        url: String,
    },
    SystemCommand {
        command: String,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub title: Arc<str>,
    pub subtitle: Option<Arc<str>>,
    pub score: i32,
    pub matched_indices: Vec<usize>,
    pub kind: SearchResultKind,
}

impl SearchResult {
    pub fn from_app(item: Arc<AppItem>, score: i32, matched_indices: Vec<usize>) -> Self {
        let subtitle = item.description.clone().or_else(|| match &item.target {
            AppTarget::Path(p) => Some(p.clone().into()),
            AppTarget::Aumid(a) => Some(format!("Store App: {a}").into()),
            AppTarget::Uri(u) => Some(format!("Settings: {u}").into()),
            AppTarget::SystemCommand(c) => Some(format!("System: {c}").into()),
        });

        Self {
            title: Arc::clone(&item.name),
            subtitle,
            score,
            matched_indices,
            kind: SearchResultKind::App(item),
        }
    }

    pub fn calculation(expression: String, result: String) -> Self {
        Self {
            title: Arc::from(result.as_str()),
            subtitle: Some(format!("= {expression}").into()),
            score: 100_000,
            matched_indices: Vec::new(),
            kind: SearchResultKind::Calculation { expression, result },
        }
    }
}
