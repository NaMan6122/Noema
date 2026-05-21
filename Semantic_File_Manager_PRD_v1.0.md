# Product Requirements Document (PRD)

## Noema — Semantic File Explorer v1.0

**Status:** Draft v2  
**Owner:** Product/Engineering  
**Target Release:** Q3 2025  
**Last Updated:** 2025-05-21  

---

# 1. Overview

Noema is a local-first, native file explorer that replaces Finder (macOS), Explorer (Windows), and Nautilus (Linux) with an intelligent, semantically-aware alternative. It provides full file management capabilities — browse, copy, move, rename, delete, preview — while layering semantic search, AI-generated context, and intent-driven discovery on top.

Users interact with their files the way they always have, but with the added ability to find anything by meaning, see relationships between documents, and let local AI surface context they'd otherwise forget.

**Core Differentiator:** A real file explorer (not a utility) with fully local AI processing, multi-format semantic indexing, hybrid search, and editable AI-generated context — zero cloud dependency.

---

# 2. Problem Statement

System file explorers have barely evolved in two decades. Users still rely on filenames, dates, and manual folder hierarchies. They frequently:

- Lose track of files saved with generic names or nested deeply
- Cannot search by meaning — only by name or date
- Face privacy risks when using cloud AI file assistants
- Waste time manually organizing, tagging, or recalling context about files
- Switch between multiple tools (file manager + search + notes + tagging apps)

Existing solutions either lack cross-format understanding, force cloud dependency, require heavy infrastructure, or are add-on utilities rather than a true file manager replacement.

Noema solves this by replacing the system file explorer entirely — bringing lightweight, local AI directly into the daily file interaction workflow.

---

# 3. Target Audience & Personas

| Persona | Primary Need | Usage Pattern |
|---|---|---|
| Researcher/Academic | Find papers, notes, datasets by concept | Bulk indexing PDFs/notes, semantic recall, citation linking |
| Developer/Engineer | Locate code, configs, logs by purpose | Cross-format search, editable AI context, plugin extensibility |
| Knowledge Worker | Manage contracts, reports, media across formats | Smart folders, hybrid search, privacy-first local storage |
| Creator/Designer | Retrieve assets, drafts, references by theme | Visual previews, AI tagging, virtual collections |
| Power User | Replace Finder/Explorer with something faster | Keyboard-first, tabs, split panes, scriptable |

---

# 4. Product Vision & Goals

**Vision:** Replace the system file explorer with one that understands your files as well as you do — entirely private, entirely local.

## v1 Goals

- Index 50k+ files with <2s average search latency
- Support 15+ core formats (PDF, DOCX, XLSX, TXT, MD, PY, JS, TS, JPG/PNG, MP3 metadata, etc.)
- 100% local processing by default; zero mandatory cloud calls
- Full file management operations at native speed
- Hybrid search with >80% relevance accuracy on benchmark queries
- Register as default file manager on macOS, Windows, and Linux
- Single binary install with no runtime dependencies

---

# 5. Core Features & Requirements

## 5.1 File Explorer Layer (The Foundation)

This is a fully functional file manager first. Every operation users expect from Finder/Explorer must work flawlessly.

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Directory Browsing | Tree view, column view, grid view, list view | Smooth scrolling in folders with 100k+ items (virtualized) |
| File Operations | Copy, move, rename, delete, create, compress/extract | Undo/redo for all destructive operations; progress indicators |
| Drag & Drop | Intra-app and OS-level drag-drop | Works with native apps; supports modifier keys (copy vs move) |
| Multi-Window & Tabs | Multiple windows, tabbed browsing, split panes | Keyboard shortcuts for all tab/pane operations |
| Thumbnail Generation | Image, PDF, video, font previews | Async generation; cached; <100ms display for cached thumbnails |
| Quick Preview | Spacebar preview pane (Quick Look equivalent) | Supports images, PDFs, text, markdown, code with syntax highlighting |
| Context Menus | Right-click with OS-standard and custom actions | Open With, Share, Compress, Copy Path, Semantic actions |
| Sidebar Navigation | Favorites, volumes, network mounts, recent locations, smart folders | Drag-to-add favorites; auto-detect mounted volumes |
| OS Integration | Default file manager registration, `file://` URI handling, system tray | Replaces native file manager in Open/Save dialogs where OS allows |
| Permissions & Metadata | Display file ownership, size, dates, ACLs | Editable where user has permissions |
| Keyboard Navigation | Vim-like shortcuts, quick command palette, instant search | Fully operable without mouse; customizable keybindings |
| Batch Operations | Multi-select → bulk rename, tag, move, export list | Regex rename, template rename, preview before apply |

## 5.2 File Ingestion & Indexing

| Feature | Description | Acceptance Criteria |
|---|---|---|
| File Watcher | Monitors user-defined directories + reacts to in-app file operations | Reacts within 3s of file change; supports recursive & symlinked dirs |
| Parser Registry | Format-specific extractors with fallback to OCR/raw text | 95% text extraction success rate across supported formats |
| Incremental Sync | Hash-based deduplication & dirty-flag queue | Re-indexes only changed chunks; <5% CPU overhead during idle |
| Semantic Chunking | Structure-aware splitting (headings, tables, code blocks, paragraphs) | Preserves context boundaries; avg chunk 256–768 tokens |
| Index Consistency | Sync index when files are moved/renamed/deleted via the explorer | Zero stale entries after in-app operations; eventual consistency for external changes |

## 5.3 Semantic Search & Retrieval

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Hybrid Search | Combines vector similarity, BM25 keyword matching, and metadata filters | Returns relevant results for both conceptual & exact queries |
| Query Understanding | Natural language parsing with intent extraction | Supports: `type:pdf after:2024-01 "budget proposal" has:tag:contract` |
| Result Ranking | Learn-to-rank using relevance feedback (thumbs up/down) | Click-through rate improves >15% after 100 feedback events |
| Virtual Folders | Saved searches that auto-update as new files match criteria | Refreshes within 5s of index update; appears in sidebar |
| Duplicate Detection | Content-hash and near-duplicate (similarity >0.95) detection | User-facing "Find Duplicates" with preview and merge/delete actions |
| Temporal Search | "What was I working on last Tuesday?" activity-aware queries | Tracks file access/modify patterns; queryable by time window |

## 5.4 Context & Intelligence Layer

| Feature | Description | Acceptance Criteria |
|---|---|---|
| User Annotations | Rich notes, custom fields, tags, relationships attached to files | Syncs with index; searchable; visible in file info panel |
| AI Context Generation | Local LLM drafts summaries, key entities, suggested tags | Runs async; editable; versioned; <10s for 50-page doc on mid-tier CPU |
| Context Persistence | Stores why a file was saved, how it's used, related items | User can pin/override AI suggestions; persists across sessions |
| Relationship Graph | Semantic/type/co-access links between documents | Interactive visualization; exportable to GraphML/CSV |
| Similar Files | "Files related to this one" in sidebar/panel | Based on embedding similarity + shared tags/entities |
| Natural Language Actions | "Find all contracts from 2024 and tag them as archive" | Parse intent → confirm with user → execute batch operation |

## 5.5 Capture & Ingest

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Quick Capture Inbox | Drop zone / hotkey to capture files for auto-indexing and triage | Configurable inbox folder; auto-tags on ingest; suggests filing location |
| Clipboard Capture | Save clipboard content (screenshots, text) as indexed files | Hotkey trigger; auto-names with timestamp + AI-suggested name |
| Contextual Clipboard | Searchable paste history with semantic search | Find content copied days ago by meaning; configurable retention |
| Browser Context (v1.1) | Extension captures download context (source page, search query) | Stores as metadata on downloaded file; searchable |

## 5.6 Workspace & Productivity

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Spaces / Workspaces | Save entire window state (tabs, layout, sidebar, open paths) per project | Switch workspaces in <500ms; persist across sessions; keyboard shortcut to switch |
| Smart Rename | AI suggests better filenames based on file content | User confirms before rename; batch mode for multiple files; undo supported |
| File Aging Indicators | Visual cues (color, icon badge) for files untouched in 6mo/1yr/2yr | Configurable thresholds; filterable ("show stale files") |
| Inline Markdown Preview | Render markdown files directly in preview pane with full formatting | Supports GFM, code blocks, images; toggle between raw and rendered |
| Terminal Integration | Open terminal at current directory; inline command output | Configurable shell; hotkey; output captured in session |
| Collection Boards | Kanban-style boards where cards are files dragged from explorer | Boards persist; cards link to files (not copies); exportable |
| Content Diffing | Compare two files semantically — what changed in meaning, not just text | Side-by-side view; highlights semantic vs. textual changes |
| File Timelines | Visual timeline of a file's modification history, annotations, access | Rendered in detail pane; exportable |

## 5.7 Automation & Intelligence (v1.1+)

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Natural Language Automations | "Every time a PDF lands in Downloads, tag it and move to Papers" | Rule builder UI; preview before enabling; pause/disable per rule |
| Programmable Hooks | Lua/WASM scripts triggered on file events (create, modify, move, tag) | Sandboxed execution; event filtering; logging |
| AI Research Assistant | "Summarize everything I have about topic X across all my files" | Cross-document synthesis; cites source files; editable output |
| Voice Search | Spoken natural language queries via system microphone | Optional; local whisper model or OS speech-to-text; privacy-first |

## 5.8 Collaboration & Sync (v2)

| Feature | Description | Acceptance Criteria |
|---|---|---|
| Multi-device Index Sync | E2E encrypted sync of index + annotations (not files) across machines | Zero plaintext leaves device; conflict resolution; selective sync |
| Collaborative Annotations | Share annotation layers with teammates without sharing files | Invite-based; permissions; real-time or async merge |

---

# 6. Non-Functional Requirements

| Category | Requirement |
|---|---|
| Privacy | All indexing, embeddings, and LLM inference run locally. No telemetry or file content leaves device unless explicitly configured. |
| Performance | <300MB RAM idle (explorer only); <800MB with full index loaded. <15% sustained CPU during background indexing. Search latency <2s p95 for 50k files. |
| Startup | Cold start <2s. Warm start <500ms. |
| Storage | Index size <10% of source data. Platform-appropriate dirs. Per-volume indexes for external drives. Encrypted index option. |
| Extensibility | Plugin API for custom parsers, embedders, and UI themes. YAML-based config. |
| Cross-Platform | Native builds for macOS 12+, Windows 10+, Ubuntu 22+. Tauri-based desktop shell with Svelte frontend. |
| Reliability | Graceful degradation if LLM unavailable (falls back to keyword + vector search). Auto-repair on index corruption. Crash-free rate ≥99.5%. |
| Distribution | Single binary/installer. No runtime dependencies (no Python, no Node, no Java). |

---

# 7. Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                        Tauri Shell (Rust)                        │
│  Native menus, dialogs, drag-drop, system tray, OS integration  │
├─────────────────────────────────────────────────────────────────┤
│                     Svelte Frontend (UI)                         │
│  File browser, search, preview, annotations, graph view         │
├─────────────────────────────────────────────────────────────────┤
│                     Rust Backend (tokio)                         │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │ File Ops  │ │ Watcher  │ │ Indexer  │ │ Search Engine    │  │
│  │ & Events  │ │ (notify) │ │ Pipeline │ │ (BM25 + Vector)  │  │
│  └───────────┘ └──────────┘ └──────────┘ └──────────────────┘  │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │ Thumbnail │ │ Parser   │ │ AI/LLM   │ │ Plugin Host      │  │
│  │ Generator │ │ Registry │ │ (llama-  │ │ (WASM sandbox)   │  │
│  │           │ │          │ │  cpp-rs) │ │                  │  │
│  └───────────┘ └──────────┘ └──────────┘ └──────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                        Storage Layer                             │
│  SQLite + sqlite-vec │ Thumbnail cache │ Config (YAML)          │
│  BGE-small (bundled) │ Per-volume indexes │ Encrypted option     │
└─────────────────────────────────────────────────────────────────┘
```

### Key Technology Choices

| Component | Choice | Rationale |
|---|---|---|
| Shell | Tauri (Rust) | Native performance, tiny binary, OS integration |
| Frontend | Svelte | Minimal overhead, fast rendering, good for list-heavy UI |
| Async runtime | tokio | Industry standard for Rust async; file I/O + networking |
| File watching | notify (crate) | Cross-platform, battle-tested |
| Parsers | lopdf, docx-rs, calamine, tree-sitter | Native Rust, no subprocess overhead |
| Embeddings | ort (ONNX Runtime bindings) + BGE-small | Fast inference, bundled model |
| LLM inference | llama-cpp-rs | GGUF models, CPU/GPU, quantized |
| Vector storage | sqlite-vec | Embedded, no external service |
| Full-text search | SQLite FTS5 (BM25) | Built-in, fast, well-understood |
| Thumbnails | image (crate), pdf-render | Async generation with LRU cache |

---

# 8. UX & Interaction Principles

- **It's a File Explorer First:** Every basic operation must feel as fast and reliable as the native OS explorer. Semantic features enhance; they never obstruct.
- **File System is Sacred (for AI):** The intelligence layer never auto-moves, renames, or deletes files. Only user-initiated operations modify the filesystem.
- **Progressive Disclosure:** Show standard file browser by default; expand to AI context, tags, relationships, and graph on demand.
- **Human-in-the-Loop:** AI suggestions are drafts. Users always edit, approve, or discard.
- **Transparent Indexing:** Clear status indicators, pause/resume controls, estimated completion times.
- **Keyboard-First:** Vim-like shortcuts, command palette (Cmd+K), instant search (Cmd+F / Cmd+P).
- **Native Feel:** Use OS-native menus, dialogs, and conventions where possible. Custom UI only for semantic features.

---

# 9. Release Roadmap

| Phase | Timeline | Deliverables |
|---|---|---|
| **MVP (Alpha)** | Weeks 1–6 | Full file explorer (browse, CRUD, tabs, sidebar, thumbnails, Quick Preview), file watcher, parser registry, embedder, basic semantic search, workspaces, terminal integration |
| **Intelligence Layer** | Weeks 7–12 | User annotations, LLM context generation, hybrid search (BM25 + vector), virtual folders, duplicate detection, temporal search, smart rename, file aging indicators |
| **Productivity** | Weeks 13–18 | Collection boards, content diffing, file timelines, inline markdown preview, contextual clipboard, similar files, relationship graph |
| **Scale & Polish** | Weeks 19–22 | Relevance feedback, plugin API (WASM), batch operations, performance tuning, programmable hooks |
| **Beta Release** | Week 24 | Cross-platform installers, encrypted index, OS registration as default file manager, docs, Quick Capture inbox |
| **v1.0 GA** | Week 28 | Plugin marketplace, advanced keyboard customization, natural language automations |
| **v1.1** | Week 32+ | Browser extension, AI research assistant, voice search, split pane views |
| **v2.0** | TBD | Multi-device index sync, collaborative annotations, optional E2E encrypted cloud |

---

# 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Replacing native file manager feels "off" | User rejection | Use native menus/dialogs/shortcuts; match OS conventions; allow fallback to native |
| Multi-format parsing failures | Low recall on niche files | Fallback to raw text + OCR; plugin registry for community parsers |
| Local LLM latency on low-end hardware | Poor UX for context generation | Async queue, model quantization (Q4/Q5), disable by default on low-end, CPU/GPU detection |
| Semantic search irrelevance | User distrust | Hybrid search + BM25 + metadata; feedback loop; query expansion |
| Index bloat over time | Storage/performance degradation | Chunk deduplication, periodic compaction, user-configurable retention |
| Large folder performance | UI jank on 100k+ item folders | Virtualized lists, async loading, thumbnail queue prioritization |
| Cross-platform inconsistency | Different behavior per OS | Abstract OS layer in Rust; platform-specific integration tests |
| Bundle size with embedded model | Large download | Ship base installer (~50MB) + model download on first launch (~130MB) |

---

# 11. Success Metrics & KPIs

| Metric | Target | Measurement |
|---|---|---|
| Search Relevance (NDCG@5) | ≥ 0.82 | Benchmark queries + user feedback |
| Indexing Throughput | ≥ 50 files/sec (mid-tier CPU) | Background worker telemetry |
| Memory Footprint (idle) | ≤ 300MB | Process monitoring |
| Startup Time (warm) | ≤ 500ms | Instrumentation |
| User Retention (30-day) | ≥ 60% DAU/MAU | Anonymous opt-in telemetry |
| AI Context Edit Rate | ≥ 40% | Context versioning logs |
| Crash-Free Sessions | ≥ 99.5% | Error reporting |
| File Operation Latency | ≤ native explorer | A/B benchmark vs Finder/Explorer |

---

# 12. Open Questions & Next Steps

- Finalize model distribution: bundle BGE-small in installer vs. download on first launch
- Define plugin API surface (parser, embedder, UI, search reranker) — likely WASM-based
- Validate chunking strategy against spreadsheet/code-heavy workflows
- Decide OS integration depth per platform (can we replace Open/Save dialogs?)
- Establish beta tester cohort & feedback pipeline
- Draft licensing model (open-core vs. fully open-source)
- Design sync strategy for optional E2E encrypted backup (v1.1)

---

# Appendices

- Parser Format Matrix (to be linked)
- Embedding Model Benchmark Results
- UX Wireframes & Flow Diagrams
- Security & Privacy Threat Model
- Platform Integration Matrix (macOS / Windows / Linux capabilities)

---

Document maintained by Product & Engineering.
