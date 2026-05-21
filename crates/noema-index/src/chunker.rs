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
        todo!("Implement structure-aware chunking")
    }
}
