use compact_str::CompactString;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LaunchTarget {
    Path(String),
    WebUrl(String),
    OsUri(String),
    Command(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IconSource {
    Path(String),
    Glyph(char),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppItem {
    id: String,
    name: Arc<str>,
    description: Option<Arc<str>>,
    target: LaunchTarget,
    icon: Option<IconSource>,
    keywords: Vec<String>,
    launch_count: u32,
    last_launched_timestamp: u64,
}

impl AppItem {
    pub fn new(id: impl Into<String>, name: impl Into<Arc<str>>, target: LaunchTarget) -> Self {
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

    pub fn with_launch_count(mut self, count: u32) -> Self {
        self.launch_count = count;
        self
    }

    pub fn with_last_launched_timestamp(mut self, timestamp: u64) -> Self {
        self.last_launched_timestamp = timestamp;
        self
    }

    pub fn record_launch(&mut self, timestamp: u64) {
        self.launch_count = self.launch_count.saturating_add(1);
        self.last_launched_timestamp = timestamp;
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn name_arc(&self) -> Arc<str> {
        Arc::clone(&self.name)
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn description_arc(&self) -> Option<Arc<str>> {
        self.description.clone()
    }

    pub fn target(&self) -> &LaunchTarget {
        &self.target
    }

    pub fn icon(&self) -> Option<&IconSource> {
        self.icon.as_ref()
    }

    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    pub fn launch_count(&self) -> u32 {
        self.launch_count
    }

    pub fn last_launched_timestamp(&self) -> u64 {
        self.last_launched_timestamp
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchResultKind {
    App(Arc<AppItem>),
    Calculation {
        expression: CompactString,
        result: CompactString,
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

const CALCULATION_SCORE: i32 = i32::MAX;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub title: Arc<str>,
    pub subtitle: Option<Arc<str>>,
    pub score: i32,
    pub matched_char_indices: Vec<usize>,
    pub kind: SearchResultKind,
}

impl SearchResult {
    pub fn from_app(item: Arc<AppItem>, score: i32, matched_char_indices: Vec<usize>) -> Self {
        // Copied straight into the `Arc<str>`: cloning the `String` first would
        // allocate a buffer that the conversion only reads and then frees.
        let subtitle = item.description_arc().or_else(|| match item.target() {
            LaunchTarget::Path(p) => Some(Arc::from(p.as_str())),
            _ => None,
        });

        Self {
            title: item.name_arc(),
            subtitle,
            score,
            matched_char_indices,
            kind: SearchResultKind::App(item),
        }
    }

    pub fn calculation(expression: CompactString, result: CompactString) -> Self {
        // Built by hand rather than with `format!`: this runs on the keystroke
        // that types the expression, and the formatting machinery would grow a
        // string of a length that is already known.
        let mut subtitle = CompactString::with_capacity(expression.len() + 2);
        subtitle.push_str("= ");
        subtitle.push_str(&expression);

        Self {
            title: Arc::from(result.as_str()),
            subtitle: Some(Arc::from(subtitle.as_str())),
            score: CALCULATION_SCORE,
            matched_char_indices: Vec::new(),
            kind: SearchResultKind::Calculation { expression, result },
        }
    }
}
