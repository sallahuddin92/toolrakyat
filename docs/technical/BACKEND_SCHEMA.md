# SmartPDF Local-First State Architecture & Data Schemas

**Document ID:** DOC-SCH-001  
**Status:** CANONICAL QUALIFIED (Phase 0 / Pre-Implementation)  
**System:** SmartPDF Application Core State & StarPDF In-Memory Models  
**Repository:** `sallahuddin92/toolrakyat`  
**Current Baseline:** `f7ff85464b190b74c89c1f8be9862be37090dd7f` (`arch/integrated`)  

---

## 1. Architectural Scope: No Central Database

SmartPDF is a **local-first, privacy-native application**. It does **not** rely on a centralized SQL or NoSQL database server for document state, mutation records, or user documents.

Authoritative document state resides entirely in the user workstation's memory within the **`starpdf-app-core`** Rust engine, persisted locally to the filesystem or origin-private storage.

This document details the exact Rust data structures, schemas, and serialization formats governing:
1. In-Memory Document Execution State (`StarExecutionState`, `StarGraph`, `ObjectState`).
2. Structural Mutations & Invariants (`DocumentDelta`, `MutationEnvelope`, `MutationCertificate`).
3. Local Client-Side Persistence (Autosave Journals, History, Recent Files, Preferences).

> [!NOTE]
> Execution contracts (`StarNodeId`, `RenderNodeId`, `EvidenceLevel`, `ObjectState`, `DocumentDelta`, `MutationEnvelope`, `MutationCertificate`) are fully implemented and qualified in `engine/starpdf/src/execution/` as shadow contracts. Their promotion to authoritative production mutation gating occurs in Phase 3. Persistent JSON journal schemas are the target specification for Phase 2.

---

## 2. In-Memory Document Execution State Schemas

### 2.1 Identity Schemas: `StarNodeId` & `RenderNodeId`

```rust
/// Canonical structural node identity in StarPDF.
/// Format: "sn1/<doc_hex>/<obj_num>/<generation>/<occurrence_path_hex>"
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StarNodeId {
    /// 16-byte cryptographically random document session UUID (lowercase hex)
    pub document_id: String,
    /// PDF Indirect Object Number (u64 decimal, no leading zeros)
    pub object_number: u64,
    /// PDF Object Generation Number (u16 decimal)
    pub generation: u16,
    /// Canonical occurrence path in content stream / tree (lowercase hex)
    pub occurrence_path: String,
}

/// Visual primitive identity linking directly to a structural StarNodeId.
/// Format: "rn1/<canonical_starnodeid_hex>/<primitive_key_hex>"
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderNodeId {
    pub star_node_id: StarNodeId,
    /// Opaque primitive identifier (e.g. glyph index, subpath run, image quadrant)
    pub primitive_key: String,
}
```

### 2.2 Provenance & Evidence Level

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvidenceLevel {
    /// Proven by direct, undamaged xref table/stream entry and matching checksum.
    Verified = 0,
    /// Reconstructed via fault-tolerant xref recovery with validated page reachability.
    RecoveredVerified = 1,
    /// Inferred via heuristic stream / catalog parsing (unverified linkage).
    Inferred = 2,
    /// Unchecked, malformed, or corrupt; prohibited from participating in certified edits.
    Untrusted = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub level: EvidenceLevel,
    pub proof_reason: String,
    pub source_revision: Option<u32>,
    pub source_byte_offset: Option<u64>,
}
```

### 2.3 ObjectState & Effective Object Index

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectState {
    pub reference: ObjectRef,
    pub storage_kind: StorageKind, // Direct, Indirect, CompressedInObjStm
    pub source_revision: Option<u32>,
    pub source_offset: Option<u64>,
    pub evidence: EvidenceRecord,
    /// SHA-256 semantic digest using domain StarPDF.semantic.v1
    pub semantic_hash: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Direct,
    IndirectTable,
    IndirectStream,
    CompressedObjectStream { stream_obj: u64, index: u32 },
}
```

### 2.4 StarExecutionState & StarGraph

```rust
pub struct StarExecutionState {
    /// Document session identifier
    pub document_id: String,
    /// Monotonically increasing revision counter within this editing session
    pub session_revision: u32,
    /// Effective object map reflecting current resolved state
    pub effective_objects: BTreeMap<ObjectRef, ObjectState>,
    /// Evidence registry tracking structural confidence per object
    pub evidence_registry: BTreeMap<ObjectRef, EvidenceRecord>,
    /// Pending, uncommitted DocumentDelta
    pub pending_delta: Option<DocumentDelta>,
    /// Active document security and capability descriptor
    pub security_info: StarSecurityInfo,
}

pub struct StarGraph {
    /// Catalog root dictionary
    pub catalog_ref: ObjectRef,
    /// Page tree index mapping 0-based page index to ObjectRef
    pub pages: Vec<ObjectRef>,
    /// Inherited resource dictionaries merged down to page level
    pub page_resources: BTreeMap<ObjectRef, PageResources>,
    /// Interactive form fields indexed by fully-qualified field name
    pub form_fields: BTreeMap<String, FormFieldNode>,
    /// Document outline tree root
    pub outlines: Vec<OutlineNode>,
}
```

---

## 3. Structural Mutation & Invariant Schemas

### 3.1 DocumentDelta

Every document modification is expressed as a discrete, verifiable `DocumentDelta`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentDelta {
    pub delta_id: String,
    pub parent_state_hash: [u8; 32],
    pub resulting_state_hash: [u8; 32],
    
    /// Added indirect objects (disjoint from modified_objects)
    pub added_objects: BTreeMap<ObjectRef, PdfObject>,
    
    /// Modified indirect objects with pre-mutation and post-mutation semantic hashes
    pub modified_objects: BTreeMap<ObjectRef, ModifiedObjectRecord>,
    
    /// Deleted indirect objects
    pub deleted_objects: BTreeSet<ObjectRef>,
    
    /// Content stream byte range edits: (owner_ref, [start, end), replacement_bytes)
    pub range_edits: Vec<SourceRangeEdit>,
    
    /// Pages structurally or visually affected by this mutation
    pub affected_pages: BTreeSet<u32>,
    
    /// Structural closure dependencies: Cs(Δ)
    pub structural_closure: StructuralClosure,
    
    /// Visual closure screen/page bounds: Cv(Δ)
    pub visual_closure: VisualClosure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifiedObjectRecord {
    pub before_hash: [u8; 32],
    pub after_hash: [u8; 32],
    pub new_object: PdfObject,
}
```

### 3.2 MutationEnvelope & MutationCertificate

```rust
/// Declared boundary contract specifying what a mutation is permitted to alter.
#[derive(Debug, Clone, PartialEq)]
pub struct MutationEnvelope {
    /// Objects explicitly authorized for creation or replacement
    pub allowed_target_objects: BTreeSet<ObjectRef>,
    /// Content stream byte ranges permitted to be modified
    pub allowed_byte_ranges: Vec<ByteRange>,
    /// Permitted visual extent bounds on each page
    pub allowed_visual_bounds: BTreeMap<u32, PageRect>,
    /// Structural dependencies strictly forbidden from being altered
    pub forbidden_dependencies: BTreeSet<ObjectRef>,
}

/// Verification certificate produced by comparing expected Envelope with actual Delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationCertificate {
    /// Zero unexplained changes detected; certified for promotion.
    Explained {
        certified_objects: Vec<ObjectRef>,
        visual_area_pts: f64,
    },
    /// Unexplained objects, ranges, or visual regions were altered.
    Unexplained {
        unexplained_objects: Vec<ObjectRef>,
        unexplained_ranges: Vec<ByteRange>,
        reason: String,
    },
    /// Explicitly refused due to security policy or forbidden dependency violation.
    Refused {
        code: RefusalCode,
        message: String,
    },
}
```

---

## 4. Local-First Persistence Schemas (Target Phase 2 Specification)

### 4.1 Session Journal (Autosave & Crash Recovery)
SmartPDF persists active uncommitted edits to local disk (or browser IndexedDB) in an append-only JSON journal:
- **Location (Desktop):** `~/Library/Application Support/SmartPDF/journals/<session_id>.json` (macOS), `%APPDATA%\SmartPDF\journals\<session_id>.json` (Windows).
- **Location (Web):** IndexedDB store `smartpdf_journals`.

```json
{
  "$schema": "https://smartpdf.app/schemas/journal-v1.json",
  "sessionId": "4f9b8c21-7e3a-4a5d-b8c9-0e1f2a3b4c5d",
  "documentHash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "sourcePath": "/Users/sallahuddin/Documents/TaxReport2026.pdf",
  "createdAt": "2026-10-18T01:10:00Z",
  "lastSavedAt": "2026-10-18T01:12:15Z",
  "deltas": [
    {
      "deltaId": "d1-9a7b",
      "timestamp": "2026-10-18T01:11:05Z",
      "action": "replace_text",
      "pageIndex": 0,
      "payload": {
        "targetNode": "sn1/4f9b8c21/12/0/text_header",
        "oldText": "2025",
        "newText": "2026"
      }
    }
  ]
}
```

### 4.2 Application Preferences Schema (`preferences.json`)

```json
{
  "$schema": "https://smartpdf.app/schemas/preferences-v1.json",
  "appearance": {
    "theme": "system",
    "uiDensity": "compact",
    "showPageThumbnails": true,
    "thumbnailWidthPx": 240
  },
  "editor": {
    "defaultZoomMode": "fitPage",
    "defaultZoomPercent": 100,
    "enableHarfRustShaping": true,
    "highlightEditableTextSpans": true,
    "autosaveIntervalSeconds": 5
  },
  "rendering": {
    "backend": "auto",
    "msaaSamples": 4,
    "enableDifferentialRepaint": true
  },
  "security": {
    "warnOnSignatureInvalidation": true,
    "enforceStrictEvidenceLevels": true
  }
}
```

### 4.3 Recent Documents Registry (`recent_documents.json`)

```json
{
  "$schema": "https://smartpdf.app/schemas/recent-v1.json",
  "documents": [
    {
      "path": "/Users/sallahuddin/Documents/TaxReport2026.pdf",
      "displayName": "TaxReport2026.pdf",
      "fileSizeBytes": 1428570,
      "pageCount": 14,
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "lastOpened": "2026-10-18T01:05:00Z",
      "hasUnsavedJournal": false
    }
  ]
}
```
