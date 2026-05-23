# Agent Memory

## Active Task
T-009 — Adopt instruction.md operating protocol
State: IN_PROGRESS
Started: 2026-05-23 14:00
Last Updated: 2026-05-23 14:00

## Task Log

### [2026-05-23 14:00] — T-009: Adopt instruction.md operating protocol
**Goal:** Restructure Memory.md and create dev-changelog.md to comply with instruction.md
**Approach:** Rewrite Memory.md to required format, create dev-changelog.md with the one existing deviation (embeddings deferral), then proceed with next development task
**Checklist:**
  - [x] Restructure Memory.md
  - [x] Create dev-changelog.md with DCL-001 (embeddings deferral)
  - [ ] Confirm active task and next step
**Outcome:** PENDING
**Blockers:** NONE

### [2026-05-23 12:00] — T-008: Implement full-text search (Week 8)
**Goal:** Wire FTS5 search into noema-search crate with query parsing, filters, snippets, and frontend integration
**Approach:** Implement QueryParser (filter extraction), SearchEngine (FTS5 BM25), snippet highlighting, IPC commands, and dual-mode GlobalSearch UI
**Checklist:**
  - [x] QueryParser with filter syntax
  - [x] SearchEngine with FTS5 BM25 ranking
  - [x] Snippet extraction with term highlighting
  - [x] Filter-only and recent-files fallback modes
  - [x] Duplicate detection (hash-based)
  - [x] IPC: content_search, find_duplicates commands
  - [x] GlobalSearch dual-mode UI (Tab toggles)
**Outcome:** Complete. 5 query parser tests pass. Full workspace compiles. GlobalSearch supports file name and content search modes.
**Blockers:** NONE

### [2026-05-23 10:00] — T-007: Implement indexing pipeline (Week 7)
**Goal:** Build the background file indexing system per spec 03_index.spec.md (without embeddings)
**Approach:** FTS5 virtual table, Markdown/PlainText parsers, semantic chunker, IndexerPipeline with priority queues and blake3 dedup, file watcher integration, status bar UI
**Checklist:**
  - [x] FTS5 virtual table + sync triggers
  - [x] MarkdownParser + PlainTextParser
  - [x] SemanticChunker (section-aware, overlap)
  - [x] IndexerPipeline (priority queues, blake3 dedup, throttling)
  - [x] DB helpers (upsert, insert chunks, etc.)
  - [x] IPC commands (index_directory, get_index_status, pause/resume)
  - [x] File watcher → auto-enqueue
  - [x] Frontend status bar indicator
**Outcome:** Complete. 8 tests pass. Indexing pipeline processes files in background, deduplicates via blake3, respects user-activity throttling.
**Blockers:** NONE

### [2026-05-21 — 2026-05-23] — T-001 through T-006: Phase 1 MVP File Explorer (Weeks 1–6)
**Goal:** Build a fully functional file explorer with Tauri + Svelte + Rust
**Approach:** Spec-driven, incremental weekly delivery
**Checklist:**
  - [x] Week 1: Project scaffolding, SQLite, config, event bus
  - [x] Week 2: Directory browsing, virtualized list, navigation, sidebar
  - [x] Week 3: File operations (copy/move/delete/rename) + undo/redo
  - [x] Week 4: Tabs, keyboard nav, command palette, workspace save/restore
  - [x] Week 5: Global filename search, preview pane, info panel, recent files
  - [x] Week 6: UI polish, Material Symbols, Geist fonts, dark/light/system theming
**Outcome:** Complete. Full MVP file explorer with tabs, keyboard nav, command palette, theming, preview, and search.
**Blockers:** NONE

## Self-Corrections
_None recorded yet._

## Open Questions
_None._

---

## Architecture Reference

- **Stack:** All-Rust backend, Tauri 2 shell, Svelte frontend
- **Crates:** noema-core, noema-fs, noema-index, noema-search, noema-ai, noema-app
- **Spec files:** specs/00_overview through specs/06_app_ipc
- **Key decisions:**
  - Embeddings (ONNX/BGE-small) deferred — see DCL-001
  - FTS5 for keyword search, ready for hybrid once embeddings added
  - Theme system uses M3 Fidelity tokens with data-theme attribute

## Key Commits (Phase 2)
| Date | Commit | Description |
|------|--------|-------------|
| 2026-05-23 | 7bc9137 | Add FTS5 virtual table and implement file parsers |
| 2026-05-23 | 04ca67e | Implement semantic chunker and indexing pipeline |
| 2026-05-23 | a23d953 | Wire indexing pipeline into Tauri IPC layer |
| 2026-05-23 | 171d89b | Auto-enqueue index jobs from file watcher events |
| 2026-05-23 | 1e2a9a6 | Add indexing status indicator in status bar |
| 2026-05-23 | ef8a385 | Implement query parser and FTS5 search engine |
| 2026-05-23 | 109b8cb | Add content_search and find_duplicates IPC commands |
| 2026-05-23 | 93e9f50 | Add content search mode to GlobalSearch |

---
*Last updated: 2026-05-23 14:00*
