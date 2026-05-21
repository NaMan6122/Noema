use std::path::PathBuf;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub semantic_text: Option<String>,
    pub keyword_text: Option<String>,
    pub exact_phrases: Vec<String>,
    pub filters: Vec<Filter>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub enum Filter {
    FileType(Vec<String>),
    DateRange {
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    },
    SizeRange {
        min: Option<u64>,
        max: Option<u64>,
    },
    HasTag(String),
    InPath(PathBuf),
}

pub struct QueryParser;

impl QueryParser {
    pub fn parse(&self, input: &str) -> ParsedQuery {
        todo!("Implement query parser with filter extraction")
    }
}
