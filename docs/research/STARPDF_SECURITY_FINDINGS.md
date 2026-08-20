# StarPDF Security & Hostile Input Analysis

**Document Status:** Security Audit & Verification Log  
**Current Milestone:** StarPDF v0.6  

---

## 1. Threat Model & Sandboxing

StarPDF is designed to process untrusted PDF files entirely within the client browser. The attack surface encompasses:
1. **Decompression Bombs:** Highly compressed streams expanding to gigabytes in memory.
2. **Infinite Recursion / Reference Cycles:** Cyclic `/Parent` trees, `/Kids` field trees, `/Prev` xref chains, or recursive indirect objects.
3. **Integer Overflows & Malformed Offsets:** Extreme table offsets, negative widths, out-of-bounds stream lengths in xref and object streams.
4. **Font Table Exploits:** Corrupt SFNT table directories, overlapping tables, out-of-bounds glyph counts, malformed cmap format 4/12 segment structures.
5. **Memory Exhaustion via Object Flooding:** Absurd `/Size` (e.g. $10^9$) or `/N` object counts allocating huge hash maps.
6. **Mutation Injection Attacks:** Oversized field values, invalid target object types, corrupt trailer parameters during incremental updates.

---

## 2. Mitigation Matrix & Hardened Limits

| Vector | Failure Mode | Mitigation in StarPDF | Status |
|---|---|---|---|
| **Deflate Bombs** | OOM | `DecompressLimits::max_decoded_bytes = 64 MB`, `max_expansion_ratio = 100x` | Verified |
| **Cyclic /Prev** | Infinite loop | `max_xref_chain_depth = 64` + cycle detection set per trailer dictionary | Verified |
| **Cyclic /Kids** | Infinite loop | `max_field_tree_depth = 32` + `HashSet<ObjectRef>` visited set | Verified |
| **AcroForm Flood** | Memory exhaustion | `max_acroform_fields = 1,000`, `max_options_count = 5,000` | Verified |
| **Page Annotation Flood** | Memory exhaustion | `max_page_annotations = 2,000` | Verified |
| **Mutation Injection** | Buffer exhaustion | `max_mutations_per_plan = 1,000`, `max_field_value_len = 1 MB` | Verified |
| **Unwrap/Expect** | Uncaught panic | **0** `.unwrap()` or `.expect()` in production library code | Verified |

---

## 3. Fuzzing History & Vulnerability Resolution Log

- **Target:** `fuzz_xref_stream` & `fuzz_incremental_writer`
- **Symptom:** Cyclic xref false positive in multi-level `/Prev` incremental update chains.
- **Root Cause:** Reading `/Prev` from merged trailer map rather than the specific current trailer dictionary being evaluated during chain traversal.
- **Resolution:** Updated `XrefResolver` to query `current_dict.get("Prev")` directly.
- **Regression Test:** Added in `tests/incremental_writer_tests.rs::test_incremental_writer_roundtrip_minimal_doc`.

## 4. v0.7 Hostile-Input Findings

- Four new and four affected legacy libFuzzer campaigns completed 18,292,731 executions with no crash, hang, OOM, or unique reproducer.
- Annotation strings, geometry arrays, InkLists, generated appearances, widget collections, mutation queues, object numbers, and incremental growth now have explicit bounds.
- Invalid/non-finite geometry and oversized contents are refused atomically with typed errors.
- The production source contains zero `.unwrap()`/`.expect()` calls and retains `#![forbid(unsafe_code)]`.

Classification: `SECURITY_HARDENING`.

## 5. v0.8 Hostile-Input Findings

- Embedded SFNT input is capped at 16 MiB and 64 tables; every table offset/length is checked before access. Subsets are capped at 4,096 glyphs/16 MiB, include bounded composite closure, rebuild checksums, and are reparsed before use.
- Appearance resources, ancestry, stream bytes, comb cells, multiline lines, list options/indexes, annotation geometry, widgets, generated objects, and mutation growth have explicit limits.
- Missing glyphs, CFF subsetting, unsupported font filters, non-identity Type0 mappings, malformed `/Opt` or `/I`, non-finite geometry, and impossible line/font data return typed errors and abort the full batch.
- Four bounded libFuzzer targets, including a final affected resolver rerun, recorded 2,214,760 clean executions with no crash, hang, OOM, or regression artifact. The production source retains zero exact `.unwrap()`/`.expect()` calls and forbids unsafe Rust.

Classification: `SECURITY_HARDENING`.
