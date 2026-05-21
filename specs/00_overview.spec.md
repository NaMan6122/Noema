# Noema — Module Specification Overview

## Conventions

### Spec Format

Each module spec describes the **contract** that implementation must fulfill. Specs are the source of truth — code is written to satisfy specs, not the other way around.

### Module Map

```
noema-core     → Shared types, error types, event bus, config loading
noema-fs       → File operations (CRUD + undo), file watcher
noema-index    → Parser registry, semantic chunker, embedding engine, indexer pipeline
noema-search   → Query parser, BM25 search, vector search, RRF ranker, result presentation
noema-ai       → LLM inference, context generation, smart suggestions
noema-app      → Tauri commands (IPC), frontend event bridge, app lifecycle
```

### Dependency Graph

```
noema-app → noema-fs, noema-search, noema-ai, noema-index, noema-core
noema-search → noema-index, noema-core
noema-index → noema-core
noema-ai → noema-core
noema-fs → noema-core
```

No circular dependencies. `noema-core` depends on nothing internal.

### Error Handling Convention

All public functions return `Result<T, NoemaError>`. Errors are categorized:

```rust
enum NoemaError {
    Io(std::io::Error),
    Database(rusqlite::Error),
    Parser { format: String, detail: String },
    Config { key: String, detail: String },
    Search { detail: String },
    Ai { detail: String },
    PluginError { plugin: String, detail: String },
    PermissionDenied { path: PathBuf },
    NotFound { path: PathBuf },
    Cancelled,
}
```

### Async Convention

- All I/O-bound operations are `async` (tokio)
- CPU-bound work (parsing, embedding) uses `spawn_blocking` or dedicated thread pools
- IPC commands are async and return serializable types

### ID Types

- `FileId(i64)` — SQLite rowid for files table
- `ChunkId(i64)` — SQLite rowid for chunks table
- `OperationId(Uuid)` — unique ID for trackable file operations
- `QueryId(Uuid)` — unique ID for search queries

### Serialization

- IPC types: `serde::Serialize + Deserialize` (JSON over Tauri bridge)
- DB types: manual `FromRow` impls (rusqlite)
- Config: TOML via `toml` crate

### Testing Convention

- Unit tests: `#[cfg(test)]` in each module
- Integration tests: `tests/` directory per crate
- Fixture files: `tests/fixtures/` (sample PDFs, DOCX, code files)
- Property tests: `proptest` for parser and chunker edge cases
