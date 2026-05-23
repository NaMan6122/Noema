use std::path::Path;

use noema_core::error::Result;
use pulldown_cmark::{Event, Parser as MdParser, Tag, TagEnd};

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

#[derive(Debug, Clone, PartialEq)]
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

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(MarkdownParser));
        registry.register(Box::new(PlainTextParser));
        registry
    }

    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    pub fn parse_file(&self, path: &Path, content: &[u8]) -> Result<ParsedDocument> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        for parser in &self.parsers {
            if parser.supported_extensions().contains(&ext) {
                return parser.parse(path, content);
            }
        }

        PlainTextParser.parse(path, content)
    }
}

pub struct MarkdownParser;

impl Parser for MarkdownParser {
    fn supported_extensions(&self) -> &[&str] {
        &["md", "markdown", "mdx"]
    }

    fn parse(&self, _path: &Path, content: &[u8]) -> Result<ParsedDocument> {
        let text = String::from_utf8_lossy(content);
        let parser = MdParser::new(&text);

        let mut sections: Vec<DocumentSection> = Vec::new();
        let mut current_text = String::new();
        let mut current_heading: Option<String> = None;
        let mut in_heading = false;
        let mut heading_text = String::new();
        let mut heading_level: u8 = 0;
        let mut in_code_block = false;
        let mut code_lang: Option<String> = None;
        let mut code_text = String::new();
        let mut in_list = false;
        let mut list_text = String::new();
        let mut title: Option<String> = None;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    flush_paragraph(&mut sections, &mut current_text, &current_heading);
                    in_heading = true;
                    heading_level = level as u8;
                    heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                    if title.is_none() {
                        title = Some(heading_text.clone());
                    }
                    sections.push(DocumentSection {
                        heading: Some(heading_text.clone()),
                        content: heading_text.clone(),
                        section_type: SectionType::Heading,
                        level: heading_level,
                    });
                    current_heading = Some(heading_text.clone());
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    flush_paragraph(&mut sections, &mut current_text, &current_heading);
                    in_code_block = true;
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            let l = lang.to_string();
                            if l.is_empty() { None } else { Some(l) }
                        }
                        _ => None,
                    };
                    code_text.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    sections.push(DocumentSection {
                        heading: current_heading.clone(),
                        content: code_text.clone(),
                        section_type: SectionType::Code { language: code_lang.take() },
                        level: 0,
                    });
                }
                Event::Start(Tag::List(_)) => {
                    flush_paragraph(&mut sections, &mut current_text, &current_heading);
                    in_list = true;
                    list_text.clear();
                }
                Event::End(TagEnd::List(_)) => {
                    in_list = false;
                    if !list_text.trim().is_empty() {
                        sections.push(DocumentSection {
                            heading: current_heading.clone(),
                            content: list_text.clone(),
                            section_type: SectionType::List,
                            level: 0,
                        });
                    }
                }
                Event::Text(t) | Event::Code(t) => {
                    let s = t.as_ref();
                    if in_heading {
                        heading_text.push_str(s);
                    } else if in_code_block {
                        code_text.push_str(s);
                    } else if in_list {
                        list_text.push_str(s);
                        list_text.push(' ');
                    } else {
                        current_text.push_str(s);
                    }
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        code_text.push('\n');
                    } else if in_list {
                        list_text.push('\n');
                    } else if !in_heading {
                        current_text.push('\n');
                    }
                }
                _ => {}
            }
        }

        flush_paragraph(&mut sections, &mut current_text, &current_heading);

        let word_count = sections.iter().map(|s| count_words(&s.content)).sum::<u32>();

        Ok(ParsedDocument {
            sections,
            metadata: DocumentMetadata {
                title,
                author: None,
                word_count,
            },
        })
    }
}

pub struct PlainTextParser;

impl Parser for PlainTextParser {
    fn supported_extensions(&self) -> &[&str] {
        &["txt", "text", "log", "csv", "tsv", "json", "toml", "yaml", "yml", "xml", "html", "htm", "rs", "py", "js", "ts", "go", "c", "cpp", "h", "hpp", "java", "rb", "sh", "bash", "zsh", "fish", "sql", "css", "scss", "svelte", "vue", "jsx", "tsx"]
    }

    fn parse(&self, _path: &Path, content: &[u8]) -> Result<ParsedDocument> {
        let text = String::from_utf8_lossy(content);
        let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();

        let sections: Vec<DocumentSection> = paragraphs
            .into_iter()
            .map(|p| DocumentSection {
                heading: None,
                content: p.to_string(),
                section_type: SectionType::Paragraph,
                level: 0,
            })
            .collect();

        let word_count = sections.iter().map(|s| count_words(&s.content)).sum::<u32>();

        Ok(ParsedDocument {
            sections,
            metadata: DocumentMetadata {
                title: None,
                author: None,
                word_count,
            },
        })
    }
}

fn flush_paragraph(
    sections: &mut Vec<DocumentSection>,
    text: &mut String,
    heading: &Option<String>,
) {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        sections.push(DocumentSection {
            heading: heading.clone(),
            content: trimmed.to_string(),
            section_type: SectionType::Paragraph,
            level: 0,
        });
    }
    text.clear();
}

fn count_words(s: &str) -> u32 {
    s.split_whitespace().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_parser_headings() {
        let md = b"# Title\n\nSome paragraph text.\n\n## Section 2\n\nMore content here.";
        let doc = MarkdownParser.parse(Path::new("test.md"), md).unwrap();

        assert_eq!(doc.metadata.title, Some("Title".to_string()));
        assert!(doc.sections.len() >= 4);
        assert_eq!(doc.sections[0].section_type, SectionType::Heading);
        assert_eq!(doc.sections[0].content, "Title");
    }

    #[test]
    fn test_markdown_parser_code_block() {
        let md = b"# Intro\n\n```rust\nfn main() {}\n```\n";
        let doc = MarkdownParser.parse(Path::new("test.md"), md).unwrap();

        let code_section = doc.sections.iter().find(|s| matches!(s.section_type, SectionType::Code { .. }));
        assert!(code_section.is_some());
        assert!(code_section.unwrap().content.contains("fn main()"));
    }

    #[test]
    fn test_plain_text_parser() {
        let text = b"First paragraph here.\n\nSecond paragraph here.";
        let doc = PlainTextParser.parse(Path::new("test.txt"), text).unwrap();

        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].content, "First paragraph here.");
    }

    #[test]
    fn test_registry_selects_parser() {
        let registry = ParserRegistry::with_defaults();
        let content = b"# Hello\n\nWorld";
        let doc = registry.parse_file(Path::new("file.md"), content).unwrap();
        assert_eq!(doc.metadata.title, Some("Hello".to_string()));
    }

    #[test]
    fn test_registry_falls_back_to_plain_text() {
        let registry = ParserRegistry::with_defaults();
        let content = b"Just some plain text here.";
        let doc = registry.parse_file(Path::new("file.unknown"), content).unwrap();
        assert_eq!(doc.sections.len(), 1);
    }
}
