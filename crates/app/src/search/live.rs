use compact_str::CompactString;
use winsp_core::calc;
use winsp_core::models::SearchResult;

pub trait LiveSource {
    fn query(&self, input: &str) -> Option<SearchResult>;
}

pub struct CalcSource;

impl LiveSource for CalcSource {
    fn query(&self, input: &str) -> Option<SearchResult> {
        let result = calc::eval(input)?;
        Some(SearchResult::calculation(CompactString::new(input), result))
    }
}
