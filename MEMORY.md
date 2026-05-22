# Noema — Project Memory

## Progress Tracker

### Phase 1: MVP File Explorer (Weeks 1–6)

#### Week 1: Project Scaffolding & Core Infrastructure — DONE
- [x] 1.1 Tauri + Svelte project init
- [x] 1.2 Rust workspace structure (noema-core, noema-fs, noema-index, noema-search, noema-ai, noema-app)
- [x] 1.3 SQLite integration (rusqlite, WAL mode, migrations)
- [x] 1.4 Config system (TOML, platform paths)
- [x] 1.5 Event bus (tokio::broadcast)
- [ ] 1.6 CI setup (GitHub Actions)
- [ ] 1.7 Dev environment docs

**Completed:** 2026-05-21 (initial scaffold + core crate)

#### Week 2: Directory Browsing & File Listing — DONE
- [x] 2.1 `list_directory` command
- [x] 2.2 List view component (virtualized) — FileList.svelte with virtual scrolling
- [x] 2.3 Column sorting — click headers to sort name/size/modified
- [x] 2.4 Navigation — clickable breadcrumb + back/forward/up buttons
- [x] 2.5 Sidebar - Favorites — Sidebar.svelte with Home/Desktop/Documents/Downloads/Pictures/Music/Videos + volumes
- [x] 2.6 File icons — emoji-based icon mapping by extension
- [x] 2.7 Hidden files toggle — checkbox in toolbar

**Completed:** 2026-05-22 (sidebar, breadcrumb, virtualized list, layout restructure)

#### Week 3: File Operations & Undo — DONE
- [x] 3.1 Copy/Move engine (IPC commands implemented)
- [x] 3.2 Delete (Trash) — uses `trash` crate, working via context menu
- [x] 3.3 Rename
- [x] 3.4 Create new folder/file
- [x] 3.5 Undo/Redo stack — full undo + redo for Copy/Move/Delete/Rename
- [x] 3.6 Progress UI (ProgressToast component)
- [x] 3.7 Conflict resolution (ConflictDialog component)
- [x] 3.8 Multi-select — basic drag-drop done

**Completed:** 2026-05-22

#### Week 4: Tabs, Keyboard Navigation & Command Palette — DONE
- [x] 4.1 Tab system — TabBar.svelte with per-tab state, drag-to-reorder
- [x] 4.2 Keyboard navigation — Arrow keys, Enter, Space, Home/End, type-ahead search
- [x] 4.3 Command palette — Cmd+K overlay with filterable commands
- [x] 4.4 Drag and drop (internal) — basic implementation
- [ ] 4.5 Drag and drop (OS)
- [x] 4.6 Context menu — basic implementation
- [x] 4.7 Workspace save/restore — auto-save on close, restore on launch

**Completed:** 2026-05-22

#### Weeks 5–6: Not started

### Key Commits
| Date | Commit | Description |
|------|--------|-------------|
| 2026-05-21 | 4602e5f | Initial scaffold: specs, architecture, compiling workspace |
| 2026-05-21 | 2511ee2 | Add Tauri + Svelte frontend and implement file operations |
| 2026-05-21 | 55192bd | Implement WatcherService with notify-debouncer-full |
| 2026-05-21 | 55030cf | Add file operation IPC commands and context menu UI |
| 2026-05-21 | 9f15925 | Add progress toasts, conflict resolution, and drag-drop |
| 2026-05-22 | 3c4c6c1 | Implement redo for file operations and add redo IPC command |
| 2026-05-22 | 326df9c | Add tab system with per-tab state and drag-to-reorder |
| 2026-05-22 | 87e8feb | Add keyboard navigation to file list |
| 2026-05-22 | f56233c | Add command palette with Cmd+K shortcut |
| 2026-05-22 | ee17728 | Add workspace save/restore for session persistence |

### Architecture Decisions
- All-Rust backend, Tauri shell, Svelte frontend
- Spec-driven development: specs are source of truth, code satisfies specs
- 7 spec files: overview, core, fs, index, search, ai, app_ipc
- 6 crates: noema-core, noema-fs, noema-index, noema-search, noema-ai, noema-app

### Current Branch
`feature/noema_fs_core` — File system and core infrastructure work

---
*Last updated: 2026-05-22*
