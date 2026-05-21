use std::path::Path;

use noema_core::error::Result;

pub trait Parser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn parse(&self, path: &Path, content: &[u8]) -> Result<ParsedDocument>;
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub sections: Vec<DocumentSection>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone)]
pub struct DocumentSection {
    pub heading: Option<String>,
    pub content: String,
    pub section_type: SectionType,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub enum SectionType {
    Paragraph,
    Heading,
    Code { language: Option<String> },
    Table,
    List,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub word_count: u32,
}

pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self { parsers: vec![] }
    }

    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    pub fn parse_file(&self, path: &Path) -> Result<ParsedDocument> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let content = std::fs::read(path)?;

        for parser in &self.parsers {
            if parser.supported_extensions().contains(&ext) {
                return parser.parse(path, &content);
            }
        }

        Ok(ParsedDocument {
            sections: vec![DocumentSection {
                heading: None,
                content: String::from_utf8_lossy(&content).to_string(),
                section_type: SectionType::Paragraph,
                level: 0,
            }],
            metadata: DocumentMetadata {
                word_count: content.len() as u32 / 5,
                ..Default::default()
            },
        })
    }
}
