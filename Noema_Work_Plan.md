# Work Plan

## Noema — Semantic File Explorer v1.0

**Created:** 2025-05-21  
**Total Timeline:** 28 weeks to v1.0 GA  
**Team Assumption:** 1–2 engineers (adjust timelines linearly for more)

---

# Phase 1: MVP File Explorer (Weeks 1–6)

The goal is a fully functional file explorer that feels native and fast — no AI yet, just the foundation.

---

## Week 1: Project Scaffolding & Core Infrastructure

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 1.1 | Tauri + Svelte project init | `cargo create-tauri-app` with Svelte template, configure build pipeline | Building skeleton app | 4h |
| 1.2 | Rust workspace structure | Set up cargo workspace with crates: `noema-core`, `noema-fs`, `noema-index`, `noema-search`, `noema-ai`, `noema-app` | Compiling workspace | 3h |
| 1.3 | SQLite integration | Add `rusqlite` with WAL mode, connection pool, run initial migrations (files table, undo_stack) | DB opens on launch | 3h |
| 1.4 | Config system | TOML config loading with defaults, platform-appropriate paths (~/.noema, %APPDATA%, XDG) | Config loads/saves | 3h |
| 1.5 | Event bus | `tokio::broadcast` event system, define core event types | Events flowing between components | 2h |
| 1.6 | CI setup | GitHub Actions: build matrix (macOS, Windows, Ubuntu), clippy, tests, format check | Green CI on push | 3h |
| 1.7 | Dev environment docs | CONTRIBUTING.md, dev setup instructions, architecture overview pointing to diagrams | Onboarding ready | 2h |

**Week 1 Exit Criteria:** App launches, shows empty window, SQLite DB created, config loaded, CI green.

---

## Week 2: Directory Browsing & File Listing

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 2.1 | `list_directory` command | Rust: read dir entries, return metadata (name, size, dates, type), handle permissions errors | IPC command working | 4h |
| 2.2 | List view component | Svelte: virtualized list (svelte-virtual-list or custom), columns (name, size, date, type) | Scrolls 100k items smoothly | 6h |
| 2.3 | Column sorting | Click header to sort by name/size/date/type, ascending/descending | Sort persists per directory | 2h |
| 2.4 | Navigation | Click to enter directory, breadcrumb path bar, back/forward history | Navigate full filesystem | 4h |
| 2.5 | Sidebar - Favorites | Hardcoded favorites (Home, Desktop, Documents, Downloads), volume list | Sidebar renders, clickable | 3h |
| 2.6 | File icons | Map extensions/mime types to icons (use a free icon set or emoji fallback) | Visual file type identification | 3h |
| 2.7 | Hidden files toggle | Cmd+Shift+. to toggle hidden files, respect config default | Toggle works | 1h |

**Week 2 Exit Criteria:** Browse any directory, smooth scrolling, sort by columns, navigate via breadcrumb + sidebar.

---

## Week 3: File Operations & Undo

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 3.1 | Copy/Move engine | Async file copy with progress tracking, same-volume move (rename), cross-volume move (copy+delete) | Operations complete correctly | 6h |
| 3.2 | Delete (Trash) | Platform-specific trash: `trash` crate (macOS/Windows/Linux Freedesktop) | Files go to OS trash | 3h |
| 3.3 | Rename | Inline rename in list view (click selected filename), validate name | Rename works inline | 3h |
| 3.4 | Create new folder/file | Context menu + keyboard shortcut to create, inline name editing | New items appear immediately | 2h |
| 3.5 | Undo/Redo stack | Track last 50 operations, Cmd+Z to undo, Cmd+Shift+Z to redo | Undo restores from trash / reverses move | 5h |
| 3.6 | Progress UI | Modal/toast showing copy/move progress (%, speed, ETA, current file) | User sees progress for large ops | 3h |
| 3.7 | Conflict resolution | Dialog: Skip / Replace / Rename when destination exists | User can resolve all conflicts | 3h |
| 3.8 | Multi-select | Cmd+Click, Shift+Click, Cmd+A for multi-select; drag selection rectangle | Standard selection behavior | 4h |

**Week 3 Exit Criteria:** Full CRUD on files, undo works, progress shown, conflicts handled, multi-select functional.

---

## Week 4: Tabs, Keyboard Navigation & Drag-Drop

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 4.1 | Tab system | Tab bar UI, Cmd+T new tab, Cmd+W close, Cmd+1-9 switch, drag to reorder | Tabbed browsing | 5h |
| 4.2 | Keyboard navigation | Arrow keys to move selection, Enter to open, Backspace to go up, / to search | Fully keyboard navigable | 5h |
| 4.3 | Command palette | Cmd+K opens palette: recent dirs, actions, search | Quick access to any action | 4h |
| 4.4 | Drag and drop (internal) | Drag files between panes/tabs/sidebar; modifier keys (hold Opt = copy) | Drag-drop moves/copies files | 5h |
| 4.5 | Drag and drop (OS) | Accept drops from other apps; drag files to other apps | Interop with OS | 4h |
| 4.6 | Context menu | Right-click menu: Open, Open With, Copy, Move, Rename, Delete, Copy Path, Get Info | Standard context menu | 3h |
| 4.7 | Workspace save/restore | Save current tab state on quit, restore on launch | Persists between sessions | 3h |

**Week 4 Exit Criteria:** Tabs, full keyboard nav, command palette, drag-drop within app and with OS, context menus.

---

## Week 5: Thumbnails, Preview & Visual Polish

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 5.1 | Thumbnail engine | Async generation with LRU disk cache, WebP output, priority queue (visible items first) | Thumbnails generated in background | 6h |
| 5.2 | Image thumbnails | Resize images to 128x128/256x256, handle JPEG/PNG/WebP/GIF/SVG | Image previews in grid/list | 3h |
| 5.3 | PDF thumbnails | Render first page via `mupdf` or `pdfium` bindings | PDF preview in grid | 4h |
| 5.4 | Grid view | Switch between list and grid view (icons + thumbnails) | Cmd+1 list, Cmd+2 grid | 4h |
| 5.5 | Quick Preview pane | Spacebar toggles preview panel: images full-size, text/code with syntax highlighting, PDF rendered | Preview without opening external app | 6h |
| 5.6 | Syntax highlighting | Use `syntect` for code file preview with theme support | Code files highlighted in preview | 3h |
| 5.7 | File info panel | Cmd+I shows detailed metadata: size, dates, permissions, path, hash | Info panel | 3h |
| 5.8 | Status bar | Bottom bar: item count, selected count, free disk space, indexing status | Always-visible status | 2h |

**Week 5 Exit Criteria:** Grid view with thumbnails, Quick Preview for images/PDFs/code/text, file info panel.

---

## Week 6: File Watcher, Basic Search & Integration

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 6.1 | File watcher service | `notify` crate, debounced (500ms), watch configured paths, .noemaignore support | Detects external changes | 5h |
| 6.2 | Basic filename search | Cmd+F opens search bar, filters current directory by filename (instant) | Type-to-filter works | 3h |
| 6.3 | Global search (filename) | Cmd+P opens global search across all watched paths (SQLite LIKE on files table) | Find any file by name | 4h |
| 6.4 | Recent files | Track file access, show in sidebar and command palette | Quick access to recent work | 3h |
| 6.5 | Native menus | Tauri menu API: File, Edit, View, Go, Window, Help with appropriate shortcuts | Native menu bar | 4h |
| 6.6 | Theme support | Light/dark mode following system preference, CSS custom properties | Respects OS appearance | 3h |
| 6.7 | Terminal integration | Action to open terminal at current directory (configurable terminal app) | "Open in Terminal" works | 2h |
| 6.8 | Polish & bug fixes | Address issues found during week 1-5 development, UX tweaks | Stable MVP | 8h |

**Week 6 Exit Criteria:** File watcher running, filename search works, native menus, theme support. **MVP complete — usable as a daily file explorer.**

---

# Phase 2: Intelligence Layer (Weeks 7–12)

Add semantic understanding on top of the working file explorer.

---

## Week 7: Parser Registry & Text Extraction

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 7.1 | Parser trait & registry | Define `Parser` trait, registry that selects parser by extension/mime | Pluggable parser system | 3h |
| 7.2 | Plain text / Markdown parser | Direct read + `pulldown-cmark` for structure extraction | .txt, .md indexed | 2h |
| 7.3 | PDF parser | `lopdf` + `pdf-extract` for text, structure from headings/font-size | .pdf indexed | 5h |
| 7.4 | DOCX parser | `docx-rs` for paragraphs, headings, tables | .docx indexed | 4h |
| 7.5 | Code parser (tree-sitter) | Tree-sitter grammars for top 10 languages, extract functions/classes/imports | Code files indexed with structure | 6h |
| 7.6 | XLSX/CSV parser | `calamine` for sheet names, cell content, preserve table context | Spreadsheets indexed | 3h |
| 7.7 | Fallback parser | Raw text extraction for unknown formats, basic encoding detection | Nothing silently fails | 2h |

**Week 7 Exit Criteria:** All major formats parsed into structured text.

---

## Week 8: Chunking & Embedding

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 8.1 | Semantic chunker | Structure-aware splitting: respect headings, code blocks, tables; 256-768 tokens; 64 overlap | Chunks table populated | 6h |
| 8.2 | ONNX Runtime integration | `ort` crate, load BGE-small model, batch inference | Embeddings generated | 5h |
| 8.3 | Embedding pipeline | Connect chunker output → batch embed (32) → store in sqlite-vec | Vectors stored | 4h |
| 8.4 | Indexer queue | Lock-free queue (crossbeam), dedup by path, priority scheduling | Background indexing running | 4h |
| 8.5 | Throttle controller | Monitor CPU, battery, user activity; throttle indexer accordingly | Indexing doesn't degrade UX | 4h |
| 8.6 | Index status UI | Progress bar, files indexed / total, current file, pause/resume button | User sees indexing progress | 3h |
| 8.7 | First-run indexing | On first launch, queue all files in watched paths, show progress | Initial index builds | 3h |

**Week 8 Exit Criteria:** Files are automatically indexed in background, embeddings stored, user can see progress.

---

## Week 9: Hybrid Search Engine

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 9.1 | FTS5 setup | Create FTS5 virtual table, triggers to sync with chunks table | Full-text search working | 3h |
| 9.2 | BM25 search | Query FTS5 with porter stemming, return ranked results | Keyword search works | 3h |
| 9.3 | Vector search | Embed query → sqlite-vec KNN (top 100) | Semantic search works | 3h |
| 9.4 | Query parser | Parse structured syntax (type:, after:, has:tag:, quoted phrases) | Filters extracted from query | 5h |
| 9.5 | RRF merge | Reciprocal Rank Fusion (k=60), metadata boost, deduplication | Hybrid results ranked | 4h |
| 9.6 | Snippet extraction | Find matching text region, highlight keywords, truncate to 200 chars | Results show context | 3h |
| 9.7 | Search UI | Replace basic filename search with full hybrid search bar, show results with snippets + scores | Semantic search in UI | 5h |
| 9.8 | Search performance | Benchmark against 50k file corpus, optimize queries, add indexes | <500ms p95 | 4h |

**Week 9 Exit Criteria:** Hybrid search working end-to-end in UI, <500ms latency, relevant results for both conceptual and exact queries.

---

## Week 10: User Annotations, Tags & Virtual Folders

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 10.1 | Annotation panel | UI panel to add/edit notes on any file | Notes persist | 4h |
| 10.2 | Tag system | Create, apply, remove tags; tag autocomplete; color-coded | Files can be tagged | 4h |
| 10.3 | Tag search integration | `has:tag:X` filter in search, tag facets in results | Tags are searchable | 3h |
| 10.4 | Virtual folders | Save search as virtual folder, auto-refresh on index changes, show in sidebar | Smart folders work | 5h |
| 10.5 | Bulk tagging | Multi-select → apply tag to all selected files | Batch operations | 2h |
| 10.6 | Custom metadata fields | User-defined key-value fields on files (e.g., "project: alpha", "status: review") | Extensible metadata | 4h |
| 10.7 | Temporal search | "What was I working on last Tuesday?" — query access_log by time window | Time-based retrieval | 4h |

**Week 10 Exit Criteria:** Users can annotate, tag, create smart folders, and search by time.

---

## Week 11: AI Context Generation

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 11.1 | llama-cpp-rs integration | Load GGUF model, configure threads/context size, basic inference | LLM generates text | 5h |
| 11.2 | Context generation prompt | Design prompt for: summary, entities, suggested tags | Quality output from LLM | 4h |
| 11.3 | Async generation flow | Queue-based, dedicated thread, emit events on completion | Non-blocking generation | 4h |
| 11.4 | Context UI | Display AI summary, entities, tags in file detail panel; edit/accept/discard buttons | User sees and edits AI context | 5h |
| 11.5 | Context versioning | Store multiple versions, show diff between AI-generated and user-edited | Edit history preserved | 3h |
| 11.6 | Smart rename suggestions | AI suggests filename based on content, user confirms | Rename suggestions work | 3h |
| 11.7 | Model management | Config for model path, download prompt on first use, Ollama connection option | Flexible model setup | 4h |
| 11.8 | Auto-context (optional) | Config flag to auto-generate context on index (disabled by default) | Opt-in automation | 2h |

**Week 11 Exit Criteria:** On-demand AI context generation working, editable, versioned. Smart rename suggests names.

---

## Week 12: Duplicate Detection & Relationships

| # | Task | Description | Deliverable | Est |
|---|---|---|---|---|
| 12.1 | Exact duplicates | Group files by content_hash, UI to view/resolve duplicate groups | Find exact copies | 3h |
| 12.2 | Near-duplicates | Embedding similarity > 0.95 threshold, cluster similar files | Find near-copies | 4h |
| 12.3 | Duplicate resolution UI | Side-by-side comparison, keep/delete/merge actions | User resolves duplicates | 4h |
| 12.4 | Similar files panel | "Files related to this one" in detail view, based on embedding proximity | Discovery feature | 3h |
| 12.5 | Relationship storage | Store relationships (similar, duplicate, references, user-linked) with strength | Graph data persists | 3h |
| 12.6 | File aging indicators | Visual badges/colors for stale files (6mo, 1yr, 2yr untouched) | Aging visible in list/grid | 3h |
| 12.7 | Phase 2 integration testing | End-to-end tests: index → search → annotate → context → relationships | Everything works together | 5h |
| 12.8 | Performance audit | Profile memory, CPU, search latency; fix bottlenecks | Meets NFR targets | 5h |

**Week 12 Exit Criteria:** Intelligence layer complete. Semantic search, annotations, AI context, duplicates, relationships all working.

---

# Phase 3: Productivity Features (Weeks 13–18)

Layer on productivity features that differentiate Noema from basic file explorers.

---

## Week 13–14: Collection Boards & Content Diffing

| # | Task | Description | Est |
|---|---|---|---|
| 13.1 | Board data model | Boards → Columns → Cards (file references), persist in SQLite | 3h |
| 13.2 | Board UI | Kanban view: drag cards between columns, create/rename/delete boards | 8h |
| 13.3 | File-to-board drag | Drag files from explorer into board as cards | 3h |
| 13.4 | Content diff engine | Text diff (standard) + semantic diff (embedding distance per paragraph) | 6h |
| 13.5 | Diff UI | Side-by-side view, highlight textual + semantic changes, summary | 6h |
| 13.6 | File timelines | Aggregate access_log + annotations + modifications into visual timeline | 5h |
| 13.7 | Inline markdown preview | Render markdown in preview pane with GFM support | 4h |

## Week 15–16: Contextual Clipboard & Relevance Feedback

| # | Task | Description | Est |
|---|---|---|---|
| 15.1 | Clipboard history service | Monitor clipboard, store entries with timestamp, index text content | 5h |
| 15.2 | Clipboard search UI | Cmd+Shift+V opens searchable clipboard history | 4h |
| 15.3 | Clipboard → file | Save clipboard entry as a file (auto-name, auto-index) | 3h |
| 15.4 | Relevance feedback | Thumbs up/down on search results, store feedback | 3h |
| 15.5 | Feedback-based reranking | Adjust RRF scores based on accumulated feedback (simple learned weights) | 5h |
| 15.6 | Search suggestions | Autocomplete based on popular queries, recent searches, tag names | 4h |
| 15.7 | Natural language actions | Parse "find all PDFs from 2024 and tag as archive" → confirm → execute | 6h |

## Week 17–18: Plugin System & Batch Operations

| # | Task | Description | Est |
|---|---|---|---|
| 17.1 | WASM runtime (wasmtime) | Initialize wasmtime, define host functions, load .wasm plugins | 6h |
| 17.2 | Parser plugin interface | Plugins can register for extensions, receive file bytes, return ParsedDocument | 5h |
| 17.3 | Plugin manifest & loading | Read plugin.toml, validate permissions, hot-reload on file change | 4h |
| 17.4 | Batch rename | Regex rename, template rename (date, counter, content-based), preview before apply | 5h |
| 17.5 | Batch operations UI | Multi-select → bulk tag, move, export file list (CSV/JSON) | 4h |
| 17.6 | Programmable hooks | Lua runtime (rlua) for file event hooks: on_create, on_modify, on_move | 6h |
| 17.7 | Performance tuning | Profile end-to-end, optimize hot paths, reduce memory, tune throttling | 6h |

**Phase 3 Exit Criteria:** Boards, diffing, clipboard, plugins, batch ops, hooks all functional. App is differentiated.

---

# Phase 4: Platform Polish & Beta (Weeks 19–24)

## Week 19–20: OS Integration

| # | Task | Description | Est |
|---|---|---|---|
| 19.1 | macOS: Default app registration | Register for file:// URIs, Dock integration | 4h |
| 19.2 | macOS: Trash & native menus | NSFileManager trash, native menu bar via Tauri | 3h |
| 19.3 | Windows: Context menu | Registry entries for shell context menu | 4h |
| 19.4 | Windows: Recycle Bin | IFileOperation integration for proper recycle | 3h |
| 19.5 | Linux: xdg-mime & .desktop | Register as default, D-Bus FileManager1 interface | 4h |
| 19.6 | Linux: Freedesktop trash & thumbnails | Spec-compliant trash and thumbnail sharing | 3h |
| 19.7 | System tray | Background mode with tray icon, quick search from tray | 4h |

## Week 21–22: Encryption, Workspaces & Graph View

| # | Task | Description | Est |
|---|---|---|---|
| 21.1 | Encrypted index (SQLCipher) | Optional passphrase-protected index database | 5h |
| 21.2 | Workspace manager | Save/load/switch named workspaces (full window state) | 5h |
| 21.3 | Workspace keyboard shortcuts | Cmd+Ctrl+1-9 to switch workspaces | 2h |
| 21.4 | Relationship graph view | Interactive force-directed graph of file relationships (D3.js or similar) | 8h |
| 21.5 | Graph filtering | Filter by relationship type, file type, date range, strength threshold | 4h |
| 21.6 | Quick Capture inbox | Hotkey → drop zone → auto-index → suggest filing location | 5h |

## Week 23–24: Beta Packaging & Docs

| # | Task | Description | Est |
|---|---|---|---|
| 23.1 | macOS .dmg + notarization | Tauri bundle, code sign, notarize, staple | 4h |
| 23.2 | Windows .msi + signing | Tauri bundle, code sign | 4h |
| 23.3 | Linux packages | AppImage, .deb, .rpm generation | 4h |
| 23.4 | First-launch onboarding | Welcome flow: pick watched dirs, download model, explain features | 5h |
| 23.5 | Documentation site | User guide, keyboard shortcuts reference, plugin development guide | 8h |
| 23.6 | Telemetry (opt-in) | Anonymous usage stats, crash reporting (sentry), explicit opt-in dialog | 4h |
| 23.7 | Beta testing pipeline | Feedback collection, auto-update mechanism (Tauri updater) | 4h |
| 23.8 | Release candidate | Full QA pass, performance benchmarks, fix critical issues | 8h |

**Phase 4 Exit Criteria:** Beta released on all three platforms. Signed, notarized, auto-updating. Docs available.

---

# Phase 5: GA (Weeks 25–28)

| # | Task | Description | Est |
|---|---|---|---|
| 25.1 | Plugin marketplace | List/install/update plugins from registry (simple JSON index) | 8h |
| 25.2 | Advanced keybinding customization | User-editable keybindings.json, conflict detection | 5h |
| 25.3 | Natural language automations UI | Rule builder: trigger + condition + action, preview, enable/disable | 8h |
| 25.4 | Search refinements | Query expansion, "did you mean?", search history | 5h |
| 25.5 | Accessibility audit | Screen reader support, high contrast, reduced motion, keyboard-only | 6h |
| 25.6 | Performance hardening | Handle 100k+ file directories, 500k+ indexed files, stress testing | 6h |
| 25.7 | Security audit | Review all file access, plugin sandbox, index encryption, path traversal | 5h |
| 25.8 | v1.0 release | Final QA, changelog, marketing site, release binaries | 8h |

**v1.0 GA Exit Criteria:** Production-quality file explorer with semantic intelligence, available on all platforms, documented, extensible.

---

# Summary

| Phase | Weeks | Focus | Key Outcome |
|---|---|---|---|
| 1 | 1–6 | File Explorer MVP | Usable daily driver |
| 2 | 7–12 | Intelligence Layer | Semantic search + AI context |
| 3 | 13–18 | Productivity | Boards, plugins, automation |
| 4 | 19–24 | Platform & Beta | OS integration, encryption, beta release |
| 5 | 25–28 | GA | Polish, accessibility, v1.0 ship |

**Total estimated hours:** ~550h (for a single engineer)  
**At 35h/week productive capacity:** ~16 weeks actual coding (remaining weeks are testing, docs, polish, buffer)

---

# Risk Buffer

Each phase has ~15% unallocated time for:
- Bug fixes discovered during development
- Platform-specific edge cases
- Performance optimization beyond initial implementation
- User feedback incorporation (from beta testers in Phase 4+)

---

# Dependencies & Prerequisites

| Dependency | Needed By | Notes |
|---|---|---|
| Rust 1.75+ | Week 1 | Stable async traits, workspace improvements |
| Tauri 2.x | Week 1 | v2 has better plugin system, multi-window support |
| Apple Developer account | Week 23 | Code signing + notarization |
| Windows code signing cert | Week 23 | EV cert recommended for SmartScreen |
| BGE-small ONNX model | Week 8 | Download from HuggingFace, host mirror |
| GGUF model (LLM) | Week 11 | User-provided or recommend specific model |
| Tree-sitter grammars | Week 7 | Bundle top 10 languages |
| Beta tester cohort | Week 23 | Recruit 20–50 users across platforms |

---

Document maintained by Engineering.
