# Technical Design Document (TDD)

## Noema — Semantic File Explorer v1.0

**Status:** Draft  
**Owner:** Engineering  
**Last Updated:** 2025-05-21  

---

# 1. System Overview

Noema is a native desktop file explorer with integrated semantic intelligence. It is built as a single Rust binary distributed via Tauri, with a Svelte frontend for the UI layer. The system operates as two logical layers sharing one process:

1. **Explorer Layer** — File browsing, CRUD operations, thumbnails, previews, OS integration
2. **Intelligence Layer** — Indexing, embeddings, search, LLM context generation, relationship inference

Both layers communicate through a shared event bus and access a common SQLite-based storage engine.

---

# 2. Architecture

## 2.1 High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Tauri Process                               │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    Svelte Frontend (WebView)                     │    │
│  │  ┌──────────┐ ┌─────────┐ ┌────────┐ ┌───────┐ ┌───────────┐  │    │
│  │  │ Browser  │ │ Search  │ │Preview │ │ Graph │ │ Annotate  │  │    │
│  │  │ Views    │ │ Bar     │ │ Pane   │ │ View  │ │ Panel     │  │    │
│  │  └──────────┘ └─────────┘ └────────┘ └───────┘ └───────────┘  │    │
│  └────────────────────────────┬────────────────────────────────────┘    │
│                               │ IPC (Tauri Commands)                    │
│  ┌────────────────────────────┴────────────────────────────────────┐    │
│  │                     Command Router                               │    │
│  └──┬──────────┬──────────┬──────────┬──────────┬─────────────────┘    │
│     │          │          │          │          │                       │
│  ┌──▼───┐  ┌──▼───┐  ┌──▼───┐  ┌──▼───┐  ┌──▼──────────┐            │
│  │ File │  │Watch │  │Index │  │Search│  │  AI Engine  │            │
│  │ Ops  │  │  er  │  │  er  │  │Engine│  │(LLM+Embed) │            │
│  └──┬───┘  └──┬───┘  └──┬───┘  └──┬───┘  └──┬──────────┘            │
│     │          │          │          │          │                       │
│  ┌──▼──────────▼──────────▼──────────▼──────────▼──────────────────┐   │
│  │                       Event Bus (tokio broadcast)                │   │
│  └──┬──────────────────────────────────────────────────────────────┘   │
│     │                                                                   │
│  ┌──▼──────────────────────────────────────────────────────────────┐   │
│  │                       Storage Engine                             │   │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────────┐  │   │
│  │  │ SQLite   │ │sqlite-vec │ │Thumbnail │ │  Config Store    │  │   │
│  │  │ (FTS5)   │ │ (vectors) │ │  Cache   │ │  (YAML/TOML)    │  │   │
│  │  └──────────┘ └───────────┘ └──────────┘ └──────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    Plugin Host (WASM runtime)                    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

## 2.2 Process Model

Single process, multi-threaded:

| Thread Pool | Purpose | Config |
|---|---|---|
| Main (tokio) | IPC handling, command routing, event dispatch | Default tokio runtime |
| File I/O | Blocking file operations (copy, move, read) | `spawn_blocking`, bounded pool (8 threads) |
| Indexer | Parsing, chunking, embedding generation | Dedicated pool (2-4 threads based on CPU) |
| AI Inference | LLM context generation | Single thread (llama.cpp manages its own threads internally) |
| Thumbnail | Image/PDF rendering for previews | Bounded pool (4 threads), priority queue |

## 2.3 IPC Design (Tauri Commands)

Frontend communicates with backend via Tauri's command system. Commands are grouped by domain:

```rust
// File operations
#[tauri::command] fn list_directory(path: PathBuf, sort: SortOrder) -> Result<Vec<FileEntry>>
#[tauri::command] fn copy_files(sources: Vec<PathBuf>, dest: PathBuf) -> Result<OperationId>
#[tauri::command] fn move_files(sources: Vec<PathBuf>, dest: PathBuf) -> Result<OperationId>
#[tauri::command] fn delete_files(paths: Vec<PathBuf>, trash: bool) -> Result<OperationId>
#[tauri::command] fn rename_file(path: PathBuf, new_name: String) -> Result<()>
#[tauri::command] fn get_metadata(path: PathBuf) -> Result<FileMetadata>

// Search
#[tauri::command] fn search(query: SearchQuery) -> Result<Vec<SearchResult>>
#[tauri::command] fn search_suggest(partial: String) -> Result<Vec<Suggestion>>

// Intelligence
#[tauri::command] fn get_context(file_id: FileId) -> Result<FileContext>
#[tauri::command] fn update_annotation(file_id: FileId, annotation: Annotation) -> Result<()>
#[tauri::command] fn generate_context(file_id: FileId) -> Result<()> // async, emits events
#[tauri::command] fn get_similar_files(file_id: FileId, limit: usize) -> Result<Vec<SimilarFile>>
#[tauri::command] fn get_duplicates(path: PathBuf) -> Result<Vec<DuplicateGroup>>

// Index management
#[tauri::command] fn get_index_status() -> Result<IndexStatus>
#[tauri::command] fn add_watch_path(path: PathBuf) -> Result<()>
#[tauri::command] fn pause_indexing() -> Result<()>
#[tauri::command] fn resume_indexing() -> Result<()>
```

Events (backend → frontend) via Tauri event system:

```rust
enum AppEvent {
    FileChanged { path: PathBuf, change: ChangeType },
    IndexProgress { total: u64, processed: u64, current_file: String },
    ContextGenerated { file_id: FileId },
    OperationProgress { op_id: OperationId, progress: f32 },
    OperationComplete { op_id: OperationId, result: OpResult },
    SearchResultsUpdated { query_id: QueryId },
}
```

---

# 3. Data Model

## 3.1 SQLite Schema

```sql
-- Core file tracking
CREATE TABLE files (
    id          INTEGER PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    filename    TEXT NOT NULL,
    extension   TEXT,
    size_bytes  INTEGER NOT NULL,
    created_at  TEXT NOT NULL,       -- ISO 8601
    modified_at TEXT NOT NULL,
    accessed_at TEXT,
    content_hash TEXT NOT NULL,      -- BLAKE3 hash
    parent_dir  TEXT NOT NULL,
    mime_type   TEXT,
    is_indexed  INTEGER DEFAULT 0,
    index_version INTEGER DEFAULT 0,
    deleted_at  TEXT                 -- soft delete for undo
);
CREATE INDEX idx_files_path ON files(path);
CREATE INDEX idx_files_parent ON files(parent_dir);
CREATE INDEX idx_files_hash ON files(content_hash);
CREATE INDEX idx_files_ext ON files(extension);

-- Semantic chunks (one file → many chunks)
CREATE TABLE chunks (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,    -- order within file
    content     TEXT NOT NULL,       -- raw text of chunk
    token_count INTEGER NOT NULL,
    heading     TEXT,                -- nearest heading context
    chunk_type  TEXT DEFAULT 'text', -- text, code, table, heading
    UNIQUE(file_id, chunk_index)
);
CREATE INDEX idx_chunks_file ON chunks(file_id);

-- Vector embeddings (sqlite-vec virtual table)
CREATE VIRTUAL TABLE chunk_embeddings USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    embedding FLOAT[384]             -- BGE-small dimension
);

-- Full-text search
CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    content='chunks',
    content_rowid='id',
    tokenize='porter unicode61'
);

-- User annotations and tags
CREATE TABLE annotations (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    note        TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE tags (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE file_tags (
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    source  TEXT DEFAULT 'user',     -- 'user' or 'ai'
    PRIMARY KEY (file_id, tag_id)
);

-- AI-generated context
CREATE TABLE ai_context (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    version     INTEGER NOT NULL DEFAULT 1,
    summary     TEXT,
    entities    TEXT,                 -- JSON array of extracted entities
    suggested_tags TEXT,             -- JSON array
    generated_at TEXT NOT NULL,
    model_id    TEXT,                -- which model produced this
    user_edited INTEGER DEFAULT 0,
    UNIQUE(file_id, version)
);

-- Relationships between files
CREATE TABLE relationships (
    id          INTEGER PRIMARY KEY,
    source_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    target_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    rel_type    TEXT NOT NULL,        -- 'similar', 'references', 'duplicate', 'user_linked'
    strength    REAL DEFAULT 0.0,     -- 0.0–1.0
    created_at  TEXT NOT NULL,
    UNIQUE(source_id, target_id, rel_type)
);
CREATE INDEX idx_rel_source ON relationships(source_id);
CREATE INDEX idx_rel_target ON relationships(target_id);

-- Virtual folders (saved searches)
CREATE TABLE virtual_folders (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    query_json  TEXT NOT NULL,        -- serialized SearchQuery
    icon        TEXT,
    position    INTEGER,
    created_at  TEXT NOT NULL
);

-- File access history (for temporal search)
CREATE TABLE access_log (
    id          INTEGER PRIMARY KEY,
    file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    action      TEXT NOT NULL,        -- 'open', 'modify', 'preview', 'search_hit'
    timestamp   TEXT NOT NULL
);
CREATE INDEX idx_access_time ON access_log(timestamp);

-- Undo stack for file operations
CREATE TABLE undo_stack (
    id          INTEGER PRIMARY KEY,
    operation   TEXT NOT NULL,        -- JSON: {type, sources, dest, timestamp}
    created_at  TEXT NOT NULL,
    expired     INTEGER DEFAULT 0
);
```

## 3.2 Index Storage Layout

```
~/.noema/                          (macOS/Linux)
%APPDATA%\Noema\                   (Windows)
├── config.toml                    Main configuration
├── index.db                       Primary SQLite database
├── index.db-wal                   WAL mode for concurrent reads
├── thumbnails/                    LRU-cached thumbnails
│   ├── ab/                        First 2 chars of hash
│   │   └── abcdef1234.webp
│   └── ...
├── models/
│   └── bge-small-en-v1.5.onnx    Bundled embedding model
├── plugins/
│   └── *.wasm                     Installed plugins
└── volumes/                       Per-volume indexes
    └── {volume-uuid}/
        └── index.db

# XDG compliance on Linux:
# ~/.local/share/noema/            (data)
# ~/.config/noema/                 (config)
# ~/.cache/noema/                  (thumbnails)
```

## 3.3 Core Data Types

```rust
pub struct FileEntry {
    pub id: FileId,
    pub path: PathBuf,
    pub filename: String,
    pub extension: Option<String>,
    pub size: u64,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub mime_type: Option<String>,
    pub is_dir: bool,
    pub thumbnail: Option<ThumbnailRef>,
    pub tags: Vec<Tag>,
    pub has_context: bool,
}

pub struct SearchQuery {
    pub text: String,                // natural language or structured
    pub filters: Vec<Filter>,
    pub sort: SortOrder,
    pub limit: usize,
    pub offset: usize,
}

pub enum Filter {
    FileType(Vec<String>),           // extension or mime
    DateRange { after: Option<DateTime<Utc>>, before: Option<DateTime<Utc>> },
    SizeRange { min: Option<u64>, max: Option<u64> },
    HasTag(String),
    InPath(PathBuf),
    ContentHash(String),
}

pub struct SearchResult {
    pub file: FileEntry,
    pub score: f32,
    pub snippet: Option<String>,     // highlighted text match
    pub match_type: MatchType,       // semantic, keyword, metadata, hybrid
}

pub enum MatchType {
    Semantic,
    Keyword,
    Metadata,
    Hybrid { semantic_score: f32, keyword_score: f32 },
}
```

---

# 4. Component Design

## 4.1 File Operations Engine

Handles all filesystem mutations with undo support.

```rust
pub struct FileOpsEngine {
    undo_stack: Arc<Mutex<VecDeque<UndoableOperation>>>,
    event_tx: broadcast::Sender<AppEvent>,
    max_undo_depth: usize, // default 50
}

impl FileOpsEngine {
    pub async fn copy(&self, sources: Vec<PathBuf>, dest: PathBuf) -> Result<OperationId>;
    pub async fn r#move(&self, sources: Vec<PathBuf>, dest: PathBuf) -> Result<OperationId>;
    pub async fn delete(&self, paths: Vec<PathBuf>, use_trash: bool) -> Result<OperationId>;
    pub async fn rename(&self, path: PathBuf, new_name: String) -> Result<()>;
    pub async fn undo(&self) -> Result<()>;
    pub async fn redo(&self) -> Result<()>;
}
```

Design decisions:
- All destructive ops go to OS trash by default (configurable)
- Operations emit progress events for UI (% complete, current file, speed)
- Conflict resolution: prompt user (rename, skip, overwrite)
- After any mutation, emits `FileChanged` event → watcher picks up and re-indexes

## 4.2 File Watcher

Uses the `notify` crate with debouncing.

```rust
pub struct WatcherService {
    watcher: RecommendedWatcher,
    debouncer: Debouncer,            // 500ms debounce window
    watched_paths: Vec<WatchedPath>,
    event_tx: broadcast::Sender<AppEvent>,
}

pub struct WatchedPath {
    pub path: PathBuf,
    pub recursive: bool,
    pub exclude_patterns: Vec<GlobPattern>, // e.g., .git, node_modules
}
```

Behavior:
- Debounces rapid changes (e.g., saves that write temp + rename)
- Batches changes and emits to indexer queue
- Handles volume mount/unmount events
- Respects `.noemaignore` files (gitignore syntax)

## 4.3 Indexer Pipeline

Processes files through a staged pipeline:

```
File Change Event
    │
    ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Filter    │────▶│    Parse    │────▶│    Chunk    │────▶│    Embed    │
│  (skip?)    │     │  (extract)  │     │  (split)    │     │  (vectorize)│
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                                                                    │
                                                                    ▼
                                                             ┌─────────────┐
                                                             │    Store    │
                                                             │  (SQLite)   │
                                                             └─────────────┘
```

```rust
pub struct IndexerPipeline {
    queue: Arc<SegQueue<IndexJob>>,   // lock-free concurrent queue
    parser_registry: ParserRegistry,
    chunker: SemanticChunker,
    embedder: EmbeddingEngine,
    store: StorageEngine,
    config: IndexerConfig,
}

pub struct IndexerConfig {
    pub max_file_size: u64,          // skip files > 100MB by default
    pub batch_size: usize,           // embed N chunks at once (default 32)
    pub idle_cpu_target: f32,        // throttle to this CPU% when idle
    pub priority_extensions: Vec<String>, // index these first
}
```

### Parser Registry

```rust
pub trait Parser: Send + Sync {
    fn supported_extensions(&self) -> &[&str];
    fn supported_mimes(&self) -> &[&str];
    fn parse(&self, path: &Path) -> Result<ParsedDocument>;
}

pub struct ParsedDocument {
    pub text: String,
    pub structure: Vec<DocumentSection>,
    pub metadata: HashMap<String, String>, // title, author, etc.
}

pub struct DocumentSection {
    pub heading: Option<String>,
    pub content: String,
    pub section_type: SectionType, // Paragraph, Code, Table, List, Heading
    pub level: u8,
}
```

Built-in parsers:

| Format | Crate | Notes |
|---|---|---|
| PDF | `lopdf` + `pdf-extract` | Falls back to OCR via `tesseract` CLI if available |
| DOCX | `docx-rs` | Extracts text + structure |
| XLSX/CSV | `calamine` | Sheet names + cell content, preserves table structure |
| Markdown | `pulldown-cmark` | Heading-aware chunking |
| Code (all) | `tree-sitter` | Language-aware: functions, classes, imports |
| Plain text | Built-in | Line-based chunking |
| Images | EXIF + OCR (optional) | Metadata extraction; OCR if enabled |
| HTML | `scraper` | Strip tags, preserve structure |

### Semantic Chunker

```rust
pub struct SemanticChunker {
    target_size: Range<usize>,       // 256–768 tokens
    overlap: usize,                  // 64 tokens overlap between chunks
}

impl SemanticChunker {
    pub fn chunk(&self, doc: &ParsedDocument) -> Vec<Chunk> {
        // 1. Respect document structure (never split mid-heading section if possible)
        // 2. Code blocks are atomic up to max size
        // 3. Tables are atomic up to max size
        // 4. Paragraphs split at sentence boundaries when too large
        // 5. Add overlap for context continuity
    }
}
```

## 4.4 Embedding Engine

```rust
pub struct EmbeddingEngine {
    session: ort::Session,           // ONNX Runtime session
    tokenizer: Tokenizer,           // from tokenizers crate
    dimension: usize,                // 384 for BGE-small
    batch_size: usize,               // 32 default
}

impl EmbeddingEngine {
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>>;
}
```

Model choice: `BAAI/bge-small-en-v1.5`
- 384 dimensions
- ~130MB ONNX file
- Good balance of quality vs. speed on CPU
- Supports query prefixing for asymmetric search

Users can swap to any ONNX embedding model via config or connect to Ollama for larger models.

## 4.5 Search Engine

Hybrid search combining three signals:

```rust
pub struct SearchEngine {
    store: StorageEngine,
    embedder: EmbeddingEngine,
    query_parser: QueryParser,
}

impl SearchEngine {
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        // 1. Parse query → extract filters + semantic text + keyword text
        // 2. Run in parallel:
        //    a) Vector search (sqlite-vec KNN on query embedding)
        //    b) BM25 search (FTS5 on keyword portion)
        //    c) Metadata filter (SQL WHERE on files table)
        // 3. Merge results with RRF (Reciprocal Rank Fusion)
        // 4. Apply metadata filters as post-filter
        // 5. Return ranked, deduplicated results with snippets
    }
}
```

### Query Parser

Structured query syntax:

```
type:pdf,docx after:2024-01-01 before:2024-06 in:/Users/me/projects
has:tag:contract size:>1mb "exact phrase" semantic meaning here
```

```rust
pub struct QueryParser;

impl QueryParser {
    pub fn parse(&self, input: &str) -> ParsedQuery {
        // Extracts:
        // - Filters (type, date, path, tag, size)
        // - Quoted exact phrases → BM25 only
        // - Remaining text → both semantic + BM25
    }
}

pub struct ParsedQuery {
    pub semantic_text: Option<String>,
    pub keyword_text: Option<String>,
    pub exact_phrases: Vec<String>,
    pub filters: Vec<Filter>,
}
```

### Ranking: Reciprocal Rank Fusion (RRF)

```rust
fn rrf_score(ranks: &[Option<usize>], k: f32) -> f32 {
    // k = 60 (standard RRF constant)
    ranks.iter()
        .filter_map(|r| r.map(|rank| 1.0 / (k + rank as f32)))
        .sum()
}
```

Each result gets its rank from vector search and BM25 independently, then RRF produces the final score. Metadata matches boost score by 1.2x multiplier.

## 4.6 AI Context Engine

```rust
pub struct AIEngine {
    llm: LlamaModel,                // llama-cpp-rs
    config: AIConfig,
}

pub struct AIConfig {
    pub model_path: PathBuf,         // path to GGUF model
    pub context_size: usize,         // 2048 default
    pub max_tokens: usize,           // 512 for summaries
    pub temperature: f32,            // 0.3 for factual extraction
    pub threads: usize,              // CPU threads for inference
}

impl AIEngine {
    pub async fn generate_context(&self, file_id: FileId, content: &str) -> Result<AIContext> {
        // Prompt: "Summarize this document. Extract key entities. Suggest tags."
        // Returns structured output parsed into AIContext
    }

    pub async fn suggest_filename(&self, content: &str) -> Result<String>;
    pub async fn extract_entities(&self, content: &str) -> Result<Vec<Entity>>;
}
```

LLM runs in a dedicated thread. Jobs are queued and processed sequentially to avoid memory pressure. Users can:
- Disable AI context entirely
- Choose model size (small/medium via config)
- Edit/override any AI-generated content

## 4.7 Thumbnail Generator

```rust
pub struct ThumbnailService {
    cache_dir: PathBuf,
    max_cache_size: u64,             // 500MB default, LRU eviction
    queue: PriorityQueue<ThumbnailJob>,
}

impl ThumbnailService {
    pub async fn get_thumbnail(&self, path: &Path, size: ThumbSize) -> Option<PathBuf> {
        // 1. Check cache (hash of path + mtime + size)
        // 2. If miss, queue generation job
        // 3. Return placeholder until ready, emit event when done
    }
}

pub enum ThumbSize {
    Small,   // 64x64
    Medium,  // 128x128
    Large,   // 256x256
    Preview, // 512x512 (for Quick Look)
}
```

Supported thumbnail sources:
- Images: `image` crate (resize + webp encode)
- PDFs: `pdfium` or `mupdf` bindings (first page render)
- Videos: extract frame via `ffmpeg` CLI if available
- Fonts: render sample text
- Code/text: syntax-highlighted first N lines rendered to image

## 4.8 Plugin System (WASM)

Plugins run in a sandboxed WASM runtime (wasmtime) with controlled capabilities.

```rust
pub trait PluginInterface {
    // Parser plugins
    fn parse(input: &[u8]) -> Result<ParsedDocument>;

    // Embedder plugins (alternative models)
    fn embed(texts: &[String]) -> Result<Vec<Vec<f32>>>;

    // UI plugins (additional panels/views)
    fn render(context: &PluginContext) -> Html;

    // Search reranker plugins
    fn rerank(query: &str, results: &[SearchResult]) -> Vec<f32>;
}
```

Plugin manifest (`plugin.toml`):

```toml
[plugin]
name = "epub-parser"
version = "0.1.0"
type = "parser"
extensions = ["epub"]
permissions = ["fs:read"]
```

Capabilities granted to plugins:
- `fs:read` — read files (only those passed to it)
- `network` — make HTTP requests (for remote model plugins)
- `ui:panel` — render a UI panel

Plugins cannot: write to filesystem, access index directly, run arbitrary code outside WASM sandbox.

---

# 5. Search Pipeline (Detailed)

```
User Input: "budget reports from Q1 that mention revenue targets"
    │
    ▼
┌─────────────────────────────────────────────────────┐
│                  Query Parser                         │
│  Extracts: semantic="budget reports mention revenue  │
│            targets"                                   │
│            filters=[date:2024-01..2024-03]           │
└─────────────────────────┬───────────────────────────┘
                          │
              ┌───────────┼───────────┐
              ▼           ▼           ▼
     ┌──────────┐  ┌──────────┐  ┌──────────┐
     │  Vector  │  │   BM25   │  │ Metadata │
     │  Search  │  │  Search  │  │  Filter  │
     │(top 100) │  │(top 100) │  │          │
     └────┬─────┘  └────┬─────┘  └────┬─────┘
          │              │              │
          └──────────────┼──────────────┘
                         ▼
              ┌─────────────────┐
              │   RRF Merge     │
              │  + Filter Pass  │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │ Snippet Extract │
              │ + Highlight     │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │  Return Top 20  │
              └─────────────────┘
```

### Performance Budget

| Step | Target Latency | Notes |
|---|---|---|
| Query parse | <5ms | Regex + string ops |
| Embedding generation | <50ms | Single query, BGE-small on CPU |
| Vector KNN (50k chunks) | <200ms | sqlite-vec ANN search |
| BM25 FTS5 | <100ms | SQLite FTS5 is fast |
| Metadata filter | <50ms | Indexed SQL |
| RRF merge | <5ms | In-memory sort |
| Snippet extraction | <50ms | Substring + highlight |
| **Total** | **<500ms** | Well under 2s target |

---

# 6. File Watcher & Sync Design

## 6.1 Event Flow

```
Filesystem Change (external)          In-App Operation
        │                                    │
        ▼                                    ▼
   notify crate                        FileOpsEngine
        │                                    │
        ▼                                    ▼
   Debouncer (500ms)              Immediate event emission
        │                                    │
        └──────────────┬─────────────────────┘
                       ▼
              ┌─────────────────┐
              │   Index Queue   │
              │  (priority)     │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │    Indexer       │
              │   Pipeline      │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │   Update DB     │
              │ + Emit Events   │
              └─────────────────┘
```

## 6.2 Consistency Guarantees

| Scenario | Behavior |
|---|---|
| File renamed externally | Watcher detects delete+create → correlate by hash → update path, preserve annotations |
| File moved externally | Same as rename — hash match preserves identity |
| File modified externally | Re-parse affected chunks only (diff against stored content hash) |
| File deleted externally | Mark as deleted in DB, preserve annotations for 30 days (configurable), remove from search |
| File renamed via Noema | Immediate index update, no re-parse needed |
| Volume unmounted | Pause indexing for that volume, show files as "offline" |

## 6.3 Ignore Rules

`.noemaignore` (gitignore syntax):

```
# Default ignores (built-in)
.git/
node_modules/
__pycache__/
.DS_Store
Thumbs.db
*.tmp
*.swp
```

---

# 7. Platform-Specific Design

## 7.1 macOS

| Feature | Implementation |
|---|---|
| Default file manager | Register via `LSHandlerURLScheme` for `file://`; replace Finder in Dock |
| Quick Look | Custom QL generator plugin + built-in preview pane |
| Spotlight integration | Export indexed metadata to Spotlight via `CSSearchableIndex` (optional) |
| Trash | Use `NSFileManager.trashItem` |
| Menu bar | Native menu via Tauri's menu API |
| Drag-drop | `NSPasteboard` integration via Tauri |
| Thumbnails | System `QLThumbnailGenerator` as fallback |
| Notarization | Required for distribution outside App Store |

## 7.2 Windows

| Feature | Implementation |
|---|---|
| Default file manager | Register as shell namespace extension (complex; phased approach) |
| Shell integration | Context menu entries via registry |
| Trash | `IFileOperation` → Recycle Bin |
| Thumbnails | `IThumbnailProvider` as fallback |
| File associations | Register via `HKEY_CLASSES_ROOT` |
| Jump lists | Recent files in taskbar |

## 7.3 Linux

| Feature | Implementation |
|---|---|
| Default file manager | `xdg-mime default noema.desktop inode/directory` |
| Desktop integration | `.desktop` file, D-Bus file manager interface |
| Trash | FreeDesktop Trash spec (`$XDG_DATA_HOME/Trash/`) |
| Thumbnails | Freedesktop thumbnail spec (shared cache) |
| File manager D-Bus | Implement `org.freedesktop.FileManager1` |
| Packaging | AppImage, .deb, .rpm, Flatpak |

---

# 8. Performance Design

## 8.1 Memory Budget

| Component | Idle | Active | Peak |
|---|---|---|---|
| Tauri + WebView | 80MB | 120MB | 200MB |
| SQLite + indexes | 50MB | 100MB | 200MB |
| Embedding model (loaded) | 0 | 150MB | 150MB |
| LLM model (loaded) | 0 | 500MB–2GB | 2GB |
| Thumbnail cache (in-memory) | 20MB | 50MB | 100MB |
| **Total (no LLM)** | **150MB** | **420MB** | **650MB** |
| **Total (with LLM)** | **150MB** | **920MB** | **2.6GB** |

Strategy:
- Embedding model: load on first search, unload after 5min idle
- LLM model: load only when user requests context generation, unload after 2min idle
- Thumbnail cache: LRU with 50MB memory cap, spill to disk

## 8.2 Startup Performance

Target: <500ms warm, <2s cold

```
Cold start sequence:
  0ms   — Process launch
  50ms  — Load config
  100ms — Open SQLite (WAL mode, no full integrity check)
  200ms — Initialize Tauri + WebView
  400ms — Render initial UI (last directory from state)
  500ms — Directory listing ready (from cache or fs)
  800ms — File watcher started
  1500ms — Embedder model loaded (background)
  2000ms — Full system ready
```

Warm start (WebView cached):
- Target <500ms to interactive UI

## 8.3 Large Directory Handling

For directories with 100k+ files:
- Virtualized list rendering (only DOM nodes for visible items)
- Paginated directory reads (1000 entries at a time)
- Sort performed in Rust, not JS
- Thumbnails loaded on-scroll with priority queue
- Metadata fetched lazily on selection

## 8.4 Indexing Throttling

```rust
pub struct ThrottleController {
    target_cpu: f32,           // 15% when user is active
    max_cpu: f32,              // 80% when idle (no input for 5min)
    current_batch_size: usize,
    last_user_input: Instant,
}
```

Behavior:
- When user is actively browsing: index at 15% CPU max
- After 5 minutes of no interaction: ramp up to 80%
- Pause completely during file operations (copy/move)
- Respect system "Low Power Mode" / battery status

---

# 9. Security & Privacy

## 9.1 Threat Model

| Threat | Mitigation |
|---|---|
| Index leaks sensitive file content | Encrypted index option (SQLCipher); index stored in user-only directory (0700) |
| Plugin reads unauthorized files | WASM sandbox; plugins only receive file content explicitly passed to them |
| LLM exfiltrates data | Model runs 100% locally; no network access from AI engine |
| Malicious GGUF model | Validate model hash against known-good list; user must explicitly approve model changes |
| Path traversal in file ops | Canonicalize all paths; reject ops that escape watched directories |
| Undo stack exposes deleted content | Undo entries expire after 24h; encrypted at rest |

## 9.2 Privacy Guarantees

- No telemetry by default. Opt-in anonymous usage stats only.
- No file content ever transmitted over network.
- Index database: user-readable only (Unix 0600).
- Encrypted index mode: SQLCipher with user-provided passphrase.
- Clipboard capture: explicit opt-in, never automatic.
- Browser extension context: stored locally, never synced.

---

# 10. Configuration

```toml
# ~/.noema/config.toml (or platform equivalent)

[general]
theme = "system"                     # system, light, dark
default_view = "list"                # list, grid, column, tree
show_hidden = false
confirm_delete = true
use_trash = true

[indexing]
enabled = true
watch_paths = ["~/Documents", "~/Projects", "~/Desktop"]
exclude_patterns = [".git", "node_modules", "__pycache__", "*.tmp"]
max_file_size_mb = 100
idle_cpu_percent = 15
active_cpu_percent = 80
idle_threshold_seconds = 300

[search]
default_mode = "hybrid"              # hybrid, semantic, keyword
max_results = 50
snippet_length = 200
enable_temporal = true

[ai]
enabled = true
embedding_model = "builtin"          # builtin, ollama:<model>, path:<onnx>
llm_model = ""                       # empty = disabled, path to GGUF
llm_threads = 4
auto_generate_context = false        # if true, generates context on index
context_max_tokens = 512

[thumbnails]
enabled = true
max_cache_mb = 500
sizes = [64, 128, 256]

[plugins]
enabled = true
directory = "~/.noema/plugins"
allowed_permissions = ["fs:read"]

[keybindings]
# Vim-style defaults, fully customizable
open = "Enter"
back = "Backspace"
search = "Cmd+P"
command_palette = "Cmd+K"
quick_preview = "Space"
new_tab = "Cmd+T"
close_tab = "Cmd+W"
```

---

# 11. Testing Strategy

| Layer | Approach | Tools |
|---|---|---|
| Unit | Core logic: parsers, chunker, query parser, RRF | `#[cfg(test)]`, proptest for edge cases |
| Integration | Pipeline: file → index → search → results | Temp directories with fixture files |
| Platform | OS integration: trash, file watching, thumbnails | Platform-specific CI (macOS, Windows, Linux) |
| Performance | Benchmarks: indexing throughput, search latency, memory | `criterion` benchmarks, 50k file synthetic corpus |
| E2E | Full UI flows: browse, search, preview, tag | Playwright/WebDriver via Tauri's test utilities |
| Fuzz | Parser inputs, query parser, path handling | `cargo-fuzz` on all external input boundaries |

---

# 12. Build & Distribution

```
cargo build --release
├── noema (single binary, ~50MB)
└── resources/
    └── bge-small-en-v1.5.onnx (~130MB, downloaded on first launch)
```

| Platform | Distribution |
|---|---|
| macOS | `.dmg` installer, Homebrew cask, notarized |
| Windows | `.msi` installer, WinGet, Scoop |
| Linux | AppImage, `.deb`, `.rpm`, Flatpak, AUR |

CI: GitHub Actions matrix (macOS-latest, windows-latest, ubuntu-22.04)

---

# 13. Milestones & Dependencies

## Phase 1: MVP File Explorer (Weeks 1–6)

- [ ] Tauri + Svelte project scaffolding
- [ ] Directory browsing (list view, tree sidebar)
- [ ] File operations (copy, move, rename, delete with undo)
- [ ] Thumbnail generation (images, PDFs)
- [ ] Quick preview pane
- [ ] Tabs and multi-window
- [ ] Keyboard navigation
- [ ] File watcher (notify)
- [ ] Basic SQLite schema

## Phase 2: Intelligence Layer (Weeks 7–12)

- [ ] Parser registry (PDF, DOCX, MD, code via tree-sitter)
- [ ] Semantic chunker
- [ ] Embedding engine (ONNX + BGE-small)
- [ ] FTS5 indexing
- [ ] Hybrid search (vector + BM25 + RRF)
- [ ] Search UI integration
- [ ] Query parser (structured syntax)
- [ ] Virtual folders

## Phase 3: Context & Polish (Weeks 13–18)

- [ ] AI context generation (llama-cpp-rs)
- [ ] User annotations and tags
- [ ] Duplicate detection
- [ ] Temporal search
- [ ] Relevance feedback
- [ ] Similar files
- [ ] Relationship graph view
- [ ] Performance tuning (throttling, large dirs)

## Phase 4: Platform & Distribution (Weeks 19–24)

- [ ] OS registration (default file manager)
- [ ] Platform-specific integration (Trash, thumbnails, menus)
- [ ] Plugin system (WASM)
- [ ] Encrypted index option
- [ ] Cross-platform installers
- [ ] Documentation
- [ ] Beta release

---

# 14. Open Technical Questions

| Question | Options | Recommendation |
|---|---|---|
| SQLite vs. separate vector DB | sqlite-vec vs. qdrant-embedded vs. lancedb | sqlite-vec (simplicity, single file, good enough for 50k files) |
| Thumbnail format | WebP vs. AVIF vs. PNG | WebP (broad support, good compression, fast encode) |
| WASM runtime | wasmtime vs. wasmer | wasmtime (Mozilla-backed, better Rust integration) |
| Tree-sitter grammar loading | Bundle all vs. download on demand | Bundle top 20 languages (~5MB), download rest |
| Embedding model hot-swap | Restart required vs. live swap | Live swap (unload old session, load new) |
| Windows shell integration depth | Basic (context menu) vs. full namespace extension | Basic for v1, namespace extension for v2 |

---

Document maintained by Engineering. Refer to PRD for product context and requirements.
