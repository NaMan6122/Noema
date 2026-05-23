# Dev Changelog

> Immutable audit trail of intentional spec deviations. Append-only — entries are never edited or deleted.

---

## [2026-05-23 10:30] — DCL-001

**Task Reference:** T-007
**Spec Affected:** specs/03_index.spec.md
**Type:** REDUCTIVE

**Original Spec:**
The indexing pipeline includes an `EmbeddingEngine` using ONNX Runtime (`ort` crate) with BGE-small model (384-dim vectors). The pipeline embeds chunks in batches of 32 and stores vectors for semantic/vector search.

**Deviation:**
Embedding engine omitted from initial implementation. Pipeline stores parsed chunks and populates FTS5 for keyword search only. The `EmbeddingEngine` struct, `ort`/`tokenizers` dependencies, and vector storage are deferred.

**Reason:**
- ONNX Runtime adds ~200MB model download and complex native build dependencies
- FTS5 keyword search is independently useful and unblocks the search module (spec 04)
- Embeddings can be layered on without changing the existing pipeline architecture
- Keeps the first iteration small and verifiable

**Impact:**
- `noema-search` currently operates in BM25-only mode (no vector KNN, no RRF merge)
- `find_similar` (cosine similarity) not yet functional
- Near-duplicate detection not yet functional (exact hash dupes work)
- When embeddings are added: pipeline gains an embed step between chunk and store; search gains vector KNN + RRF fusion

**Spec Updated:** NO — spec remains as the target; implementation will converge once embeddings are added in a future task.
