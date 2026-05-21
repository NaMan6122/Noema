# Module: noema-index

## Purpose

Handles the ingestion pipeline: parsing files into structured text, splitting into semantic chunks, generating embeddings, and storing everything in SQLite. Manages the background indexing queue with throttling.

---

## Public Interface

### Parser Trait & Registry

```rust
pub trait Parser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn supported_mimes(&self) -> &[&str];
    fn parse(&self, path: &Path, content: &[u8]) -> Result<ParsedDocument, NoemaError>;
}

pub struct ParsedDocument {
    pub sections: Vec<DocumentSection>,
    pub metadata: DocumentMetadata,
}

pub struct DocumentSection {
    pub heading: Option<String>,
    pub content: String,
    pub section_type: SectionType,
    pub level: u8,                  // nesting depth
}

pub enum SectionType {
    Paragraph,
    Heading,
    Code { language: Option<String> },
    Table,
    List,
    Image { alt: Option<String>, path: Option<String> },
}

pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub page_count: Option<u32>,
    pub word_count: u32,
    pub language: Option<String>,
    pub extra: HashMap<String, String>,
}

pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, parser: Box<dyn Parser>);
    pub fn parse_file(&self, path: &Path) -> Result<ParsedDocument, NoemaError>;
}
```

### Semantic Chunker

```rust
pub struct SemanticChunker {
    pub min_tokens: usize,      // 256
    pub max_tokens: usize,      // 768
    pub overlap_tokens: usize,  // 64
}

pub struct Chunk {
    pub index: usize,
    pub content: String,
    pub token_count: usize,
    pub heading_context: Option<String>,  // nearest parent heading
    pub chunk_type: SectionType,
}

impl SemanticChunker {
    pub fn new(min: usize, max: usize, overlap: usize) -> Self;
    pub fn chunk(&self, doc: &ParsedDocument) -> Vec<Chunk>;
}
```

### Embedding Engine

```rust
pub struct EmbeddingEngine {
    session: ort::Session,
    tokenizer: tokenizers::Tokenizer,
    dimension: usize,           // 384 for BGE-small
}

impl EmbeddingEngine {
    pub fn load(model_path: &Path) -> Result<Self, NoemaError>;
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, NoemaError>;
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>, NoemaError>;
    pub fn dimension(&self) -> usize;
    pub fn unload(&mut self);
    pub fn is_loaded(&self) -> bool;
}
```

### Indexer Pipeline

```rust
pub struct IndexerPipeline {
    queue: Arc<IndexQueue>,
    parser_registry: Arc<ParserRegistry>,
    chunker: SemanticChunker,
    embedder: Arc<Mutex<EmbeddingEngine>>,
    db: Arc<Database>,
    event_bus: Arc<EventBus>,
    throttle: ThrottleController,
}

pub struct IndexQueue {
    pending: SegQueue<IndexJob>,
    in_progress: AtomicUsize,
}

pub struct IndexJob {
    pub path: PathBuf,
    pub priority: Priority,
    pub reason: IndexReason,
}

pub enum Priority { High, Normal, Low }
pub enum IndexReason { NewFile, Modified, Reindex, Manual }

impl IndexerPipeline {
    pub fn new(
        parser_registry: Arc<ParserRegistry>,
        embedder: Arc<Mutex<EmbeddingEngine>>,
        db: Arc<Database>,
        event_bus: Arc<EventBus>,
        config: &IndexingConfig,
    ) -> Self;

    pub async fn start(&self) -> Result<(), NoemaError>;
    pub async fn stop(&self);
    pub fn pause(&self);
    pub fn resume(&self);
    pub fn is_paused(&self) -> bool;

    pub fn enqueue(&self, job: IndexJob);
    pub fn enqueue_batch(&self, jobs: Vec<IndexJob>);
    
    pub fn status(&self) -> IndexStatus;
}

pub struct IndexStatus {
    pub state: IndexState,          // Idle, Running, Paused
    pub total_files: u64,
    pub indexed_files: u64,
    pub pending_jobs: u64,
    pub current_file: Option<String>,
    pub files_per_second: f32,
}

pub struct ThrottleController {
    pub target_cpu: f32,
    pub max_cpu: f32,
    pub idle_threshold: Duration,
    pub last_user_input: Arc<AtomicU64>,  // timestamp
}

impl ThrottleController {
    pub fn should_throttle(&self) -> bool;
    pub fn current_batch_size(&self) -> usize;
    pub fn notify_user_active(&self);
}
```

---

## Behavior

### Indexing Pipeline (per file)

1. **Dequeue** job from queue (priority-ordered)
2. **Check** if file still exists and hash differs from stored hash
3. **Read** file bytes
4. **Parse** via ParserRegistry → `ParsedDocument`
5. **Chunk** via SemanticChunker → `Vec<Chunk>`
6. **Embed** chunks in batches of 32 → `Vec<Vec<f32>>`
7. **Store** in SQLite:
   - Upsert `files` row (path, hash, metadata)
   - Delete old chunks for this file
   - Insert new chunks + embeddings
   - Update FTS5 index
8. **Emit** `IndexProgress` event

### Throttling

- When user is active (input within `idle_threshold`): process 1 file at a time, sleep between files
- When user is idle: process batch_size files, no sleep
- When on battery (macOS/Windows): reduce max CPU to 10%
- Pause during active file operations (copy/move)

### Hash-Based Deduplication

- BLAKE3 hash of full file content
- If hash matches existing record and path unchanged → skip (already indexed)
- If hash matches but path changed → update path only (file was moved)
- If hash differs → full re-index

### Parser Selection

1. Match by extension first (fastest)
2. If no match, detect MIME type (via `infer` crate)
3. If still no match, use fallback plain-text parser
4. If binary file detected and no parser → skip with log

---

## Edge Cases & Error Handling

- **File deleted between enqueue and processing:** Skip, emit warning, remove from DB if present
- **Parser fails on a file:** Log error, skip file, mark as `parse_error` in DB (retry on next modify)
- **Embedding model not loaded:** Queue jobs, load model on first embed request, process backlog
- **Out of memory during large file:** Limit file read to `max_file_size_mb`, skip oversized
- **Corrupt PDF/DOCX:** Parser returns partial result if possible, or error → fallback parser
- **FTS5 token limit:** Truncate chunk content to 1MB before FTS insertion

---

## Dependencies

```toml
[dependencies]
noema-core = { path = "../noema-core" }
tokio = { version = "1", features = ["rt", "sync"] }
crossbeam-queue = "0.3"     # lock-free queue
ort = "2"                   # ONNX Runtime
tokenizers = "0.19"         # HuggingFace tokenizers
lopdf = "0.32"              # PDF
docx-rs = "0.4"             # DOCX
calamine = "0.25"           # XLSX/XLS/ODS
pulldown-cmark = "0.11"     # Markdown
tree-sitter = "0.22"        # Code parsing
tree-sitter-rust = "0.21"   # + language grammars...
scraper = "0.19"            # HTML
blake3 = "1"
infer = "0.16"              # MIME detection
```

---

## Performance Constraints

- Parsing: ≥50 files/sec for average-sized documents (mid-tier CPU)
- Chunking: <10ms per document
- Embedding (batch of 32): <500ms on CPU (BGE-small)
- Index write (per file): <5ms
- Total per-file pipeline: <100ms (excluding embedding wait)

---

## Example Usage

```rust
// Setup
let parser_registry = Arc::new(ParserRegistry::new_with_defaults());
let embedder = Arc::new(Mutex::new(EmbeddingEngine::load(&model_path)?));
let pipeline = IndexerPipeline::new(parser_registry, embedder, db, event_bus, &config.indexing);

// Start background processing
pipeline.start().await?;

// Enqueue a file
pipeline.enqueue(IndexJob {
    path: "/Users/me/report.pdf".into(),
    priority: Priority::Normal,
    reason: IndexReason::NewFile,
});

// Check status
let status = pipeline.status();
println!("Indexed {}/{} files", status.indexed_files, status.total_files);
```
