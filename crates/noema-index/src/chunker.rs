use crate::parser::{DocumentSection, ParsedDocument, SectionType};

pub struct SemanticChunker {
    pub min_tokens: usize,
    pub max_tokens: usize,
    pub overlap_tokens: usize,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub content: String,
    pub token_count: usize,
    pub heading_context: Option<String>,
    pub chunk_type: SectionType,
}

impl SemanticChunker {
    pub fn new(min: usize, max: usize, overlap: usize) -> Self {
        Self {
            min_tokens: min,
            max_tokens: max,
            overlap_tokens: overlap,
        }
    }

    pub fn default_config() -> Self {
        Self::new(256, 768, 64)
    }

    pub fn chunk(&self, doc: &ParsedDocument) -> Vec<Chunk> {
        let mut chunks: Vec<Chunk> = Vec::new();
        let mut buffer = String::new();
        let mut buffer_tokens: usize = 0;
        let mut current_heading: Option<String> = None;
        let mut current_type = SectionType::Paragraph;

        for section in &doc.sections {
            if section.section_type == SectionType::Heading {
                // Flush buffer before new heading
                if buffer_tokens >= self.min_tokens {
                    self.emit_chunk(&mut chunks, &buffer, buffer_tokens, &current_heading, &current_type);
                    buffer.clear();
                    buffer_tokens = 0;
                }
                current_heading = Some(section.content.clone());
                continue;
            }

            let section_tokens = estimate_tokens(&section.content);

            // If single section exceeds max, split it
            if section_tokens > self.max_tokens {
                // Flush current buffer first
                if !buffer.is_empty() {
                    self.emit_chunk(&mut chunks, &buffer, buffer_tokens, &current_heading, &current_type);
                    buffer.clear();
                    buffer_tokens = 0;
                }
                self.split_large_section(&mut chunks, section, &current_heading);
                continue;
            }

            // If adding this section would exceed max, flush
            if buffer_tokens + section_tokens > self.max_tokens && buffer_tokens >= self.min_tokens {
                self.emit_chunk(&mut chunks, &buffer, buffer_tokens, &current_heading, &current_type);

                // Overlap: keep tail of previous buffer
                let overlap_text = tail_words(&buffer, self.overlap_tokens);
                buffer.clear();
                buffer.push_str(&overlap_text);
                buffer_tokens = estimate_tokens(&overlap_text);
            }

            if !buffer.is_empty() {
                buffer.push_str("\n\n");
            }
            buffer.push_str(&section.content);
            buffer_tokens += section_tokens;
            current_type = section.section_type.clone();
        }

        // Flush remaining
        if !buffer.trim().is_empty() {
            self.emit_chunk(&mut chunks, &buffer, buffer_tokens, &current_heading, &current_type);
        }

        chunks
    }

    fn emit_chunk(
        &self,
        chunks: &mut Vec<Chunk>,
        content: &str,
        token_count: usize,
        heading: &Option<String>,
        chunk_type: &SectionType,
    ) {
        chunks.push(Chunk {
            index: chunks.len(),
            content: content.to_string(),
            token_count,
            heading_context: heading.clone(),
            chunk_type: chunk_type.clone(),
        });
    }

    fn split_large_section(
        &self,
        chunks: &mut Vec<Chunk>,
        section: &DocumentSection,
        heading: &Option<String>,
    ) {
        let words: Vec<&str> = section.content.split_whitespace().collect();
        let target_words = self.max_tokens * 3 / 4; // tokens ≈ words * 4/3
        let overlap_words = self.overlap_tokens * 3 / 4;

        let mut start = 0;
        while start < words.len() {
            let end = (start + target_words).min(words.len());
            let chunk_content: String = words[start..end].join(" ");
            let token_count = estimate_tokens(&chunk_content);

            chunks.push(Chunk {
                index: chunks.len(),
                content: chunk_content,
                token_count,
                heading_context: heading.clone(),
                chunk_type: section.section_type.clone(),
            });

            if end >= words.len() {
                break;
            }
            start = end.saturating_sub(overlap_words);
        }
    }
}

fn estimate_tokens(text: &str) -> usize {
    // Rough approximation: 1 token ≈ 0.75 words (or ~4 chars)
    let words = text.split_whitespace().count();
    (words as f64 * 1.33) as usize
}

fn tail_words(text: &str, token_count: usize) -> String {
    let word_count = (token_count as f64 * 0.75) as usize;
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= word_count {
        return text.to_string();
    }
    words[words.len() - word_count..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DocumentMetadata, DocumentSection, ParsedDocument, SectionType};

    fn make_doc(sections: Vec<(&str, SectionType)>) -> ParsedDocument {
        ParsedDocument {
            sections: sections
                .into_iter()
                .map(|(content, st)| DocumentSection {
                    heading: None,
                    content: content.to_string(),
                    section_type: st,
                    level: 0,
                })
                .collect(),
            metadata: DocumentMetadata::default(),
        }
    }

    #[test]
    fn test_single_small_section_produces_one_chunk() {
        let chunker = SemanticChunker::new(10, 100, 5);
        let doc = make_doc(vec![("Hello world this is a test of chunking.", SectionType::Paragraph)]);
        let chunks = chunker.chunk(&doc);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains("Hello world"));
    }

    #[test]
    fn test_large_section_gets_split() {
        let chunker = SemanticChunker::new(5, 20, 3);
        let long_text = "word ".repeat(100);
        let doc = make_doc(vec![(&long_text, SectionType::Paragraph)]);
        let chunks = chunker.chunk(&doc);
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_heading_flushes_buffer() {
        let chunker = SemanticChunker::new(5, 200, 3);
        let doc = ParsedDocument {
            sections: vec![
                DocumentSection {
                    heading: None,
                    content: "First section content here with enough words to pass minimum.".to_string(),
                    section_type: SectionType::Paragraph,
                    level: 0,
                },
                DocumentSection {
                    heading: Some("New Section".to_string()),
                    content: "New Section".to_string(),
                    section_type: SectionType::Heading,
                    level: 1,
                },
                DocumentSection {
                    heading: None,
                    content: "Second section content here with enough words.".to_string(),
                    section_type: SectionType::Paragraph,
                    level: 0,
                },
            ],
            metadata: DocumentMetadata::default(),
        };
        let chunks = chunker.chunk(&doc);
        assert!(chunks.len() >= 2);
    }
}
