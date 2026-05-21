# Module: noema-search

## Purpose

Provides hybrid search combining vector similarity (semantic), BM25 keyword matching (FTS5), and metadata filtering. Includes query parsing, result ranking via Reciprocal Rank Fusion, snippet extraction, and relevance feedback learning.

---

## Public Interface

### Query Parser

```rust
pub struct QueryParser;

impl QueryParser {
    pub fn parse(&self, input: &str) -> ParsedQuery;
}

pub struct ParsedQuery {
    pub semantic_text: Option<String>,    // text for embedding
    pub keyword_text: Option<String>,     // text for BM25
    pub exact_phrases: Vec<String>,       // quoted phrases → BM25 only
    pub filters: Vec<Filter>,
    pub sort: Option<SortOrder>,
    pub raw: String,                      // original input preserved
}

pub enum Filter {
    FileType(Vec<String>),               // type:pdf,docx
    DateRange {
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    },
    SizeRange {
        min: Option<u64>,
        max: Option<u64>,
    },
    HasTag(String),                       // has:tag:contract
    InPath(PathBuf),                      // in:/Users/me/projects
    IsIndexed(bool),
    HasContext(bool),
}
```

### Search Engine

```rust
pub struct SearchEngine {
    db: Arc<Database>,
    embedder: Arc<Mutex<EmbeddingEngine>>,
}

impl SearchEngine {
    pub fn new(db: Arc<Database>, embedder: Arc<Mutex<EmbeddingEngine>>) -> Self;

    pub async fn search(&self, query: SearchQuery) -> Result<SearchResults, NoemaError>;

    pub async fn suggest(&self, partial: &str) -> Result<Vec<Suggestion>, NoemaError>;

    pub async fn find_similar(
        &self,
        file_id: FileId,
        limit: usize,
    ) -> Result<Vec<SimilarFile>, NoemaError>;

    pub async fn find_duplicates(
        &self,
        path: Option<&Path>,    // scope to directory, or None for all
    ) -> Result<Vec<DuplicateGroup>, NoemaError>;
}

pub struct SearchQuery {
    pub parsed: ParsedQuery,
    pub limit: usize,           // default 20
    pub offset: usize,          // for pagination
}

pub struct SearchResults {
    pub query_id: QueryId,
    pub results: Vec<SearchResult>,
    pub total_estimate: u64,    // approximate total matches
    pub took_ms: u64,
}

pub struct SearchResult {
    pub file: FileEntry,
    pub score: f32,
    pub snippet: Option<HighlightedSnippet>,
    pub match_type: MatchType,
}

pub struct HighlightedSnippet {
    pub text: String,
    pub highlights: Vec<Range<usize>>,  // byte ranges of matched terms
}

pub enum MatchType {
    Semantic { score: f32 },
    Keyword { score: f32 },
    Hybrid { semantic_score: f32, keyword_score: f32 },
    Metadata,
}

pub struct Suggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
}

pub enum SuggestionType {
    RecentQuery,
    TagName,
    FileName,
    FilterHint,     // e.g., "type:" "after:" autocomplete
}

pub struct SimilarFile {
    pub file: FileEntry,
    pub similarity: f32,        // 0.0–1.0
}

pub struct DuplicateGroup {
    pub hash: String,
    pub files: Vec<FileEntry>,
    pub is_exact: bool,         // true = identical hash, false = near-duplicate
}
```

### Relevance Feedback

```rust
pub struct FeedbackEngine {
    db: Arc<Database>,
}

impl FeedbackEngine {
    pub fn record_feedback(
        &self,
        query_id: QueryId,
        file_id: FileId,
        signal: FeedbackSignal,
    ) -> Result<(), NoemaError>;

    pub fn get_boost(&self, file_id: FileId) -> f32;  // learned boost factor
}

pub enum FeedbackSignal {
    ThumbsUp,
    ThumbsDown,
    Clicked,
    Opened,
}
```

---

## Behavior

### Search Pipeline

1. **Parse** query string → `ParsedQuery`
2. **Parallel execution** (tokio::join!):
   - **Vector search:** embed query → sqlite-vec KNN (top 100)
   - **BM25 search:** FTS5 MATCH query (top 100)
3. **Merge** via Reciprocal Rank Fusion (RRF):
   ```
   score(doc) = Σ 1/(k + rank_i) for each retrieval method where doc appears
   k = 60 (standard constant)
   ```
4. **Apply metadata filters** as post-filter (remove non-matching)
5. **Apply boosts:**
   - Metadata match (exact type/date/tag): 1.2x
   - Recent access (within 7 days): 1.1x
   - Positive feedback history: learned weight
6. **Extract snippets** for top results (find matching chunk, highlight terms)
7. **Return** top N results

### Query Parser Syntax

```
"exact phrase"           → exact_phrases (BM25 only, must match)
type:pdf                 → Filter::FileType(["pdf"])
type:pdf,docx            → Filter::FileType(["pdf", "docx"])
after:2024-01-01         → Filter::DateRange { after: ... }
before:2024-06           → Filter::DateRange { before: ... }
size:>10mb               → Filter::SizeRange { min: 10MB }
size:<1kb                → Filter::SizeRange { max: 1KB }
has:tag:contract         → Filter::HasTag("contract")
in:/path/to/dir          → Filter::InPath(...)
remaining free text      → semantic_text AND keyword_text
```

Filters are extracted first; remaining text goes to both semantic and keyword search.

### Suggestions

- Recent queries (last 50, stored in DB)
- Tag names matching prefix
- File names matching prefix (top 10 by access frequency)
- Filter syntax hints when user types `type:` or `after:` etc.

### Duplicate Detection

- **Exact:** Group by `content_hash` in files table where count > 1
- **Near-duplicate:** For each file's embedding, find others with cosine similarity > 0.95

---

## Edge Cases & Error Handling

- **Empty query:** Return recent files (sorted by last accessed)
- **Only filters, no text:** Run metadata-only search (no vector/BM25)
- **Embedding model not loaded:** Fall back to BM25-only search, log warning
- **FTS5 syntax error from user input:** Escape special chars, retry
- **Zero results:** Suggest broader query (remove filters, expand terms)
- **Very long query (>500 chars):** Truncate for embedding, use full for BM25

---

## Dependencies

```toml
[dependencies]
noema-core = { path = "../noema-core" }
noema-index = { path = "../noema-index" }  # for EmbeddingEngine
tokio = { version = "1", features = ["rt", "sync"] }
```

---

## Performance Constraints

| Step | Budget |
|---|---|
| Query parse | <5ms |
| Query embedding | <50ms |
| Vector KNN (500k vectors) | <200ms |
| BM25 FTS5 | <100ms |
| RRF merge | <5ms |
| Snippet extraction (20 results) | <50ms |
| **Total** | **<500ms p95** |

---

## Example Usage

```rust
let search_engine = SearchEngine::new(db.clone(), embedder.clone());

// Simple search
let results = search_engine.search(SearchQuery {
    parsed: QueryParser.parse("budget reports Q1 revenue"),
    limit: 20,
    offset: 0,
}).await?;

for result in &results.results {
    println!("{} (score: {:.2})", result.file.filename, result.score);
}

// Find similar files
let similar = search_engine.find_similar(file_id, 10).await?;

// Find duplicates
let dupes = search_engine.find_duplicates(Some(Path::new("/Users/me/Documents"))).await?;
```
