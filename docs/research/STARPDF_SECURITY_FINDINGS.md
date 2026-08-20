# StarPDF Security & Hostile Input Analysis

**Document Status:** Security Audit & Verification Log  
**Current Milestone:** StarPDF v0.5  

---

## 1. Threat Model & Sandboxing

StarPDF is designed to process untrusted PDF files entirely within the client browser. The attack surface encompasses:
1. **Decompression Bombs:** Highly compressed streams expanding to gigabytes in memory.
2. **Infinite Recursion / Reference Cycles:** Cyclic `/Parent` trees, `/Prev` xref chains, or recursive indirect objects.
3. **Integer Overflows & Malformed Offsets:** Extreme table offsets, negative widths, out-of-bounds stream lengths in xref and object streams.
4. **Font Table Exploits:** Corrupt SFNT table directories, overlapping tables, out-of-bounds glyph counts, malformed cmap format 4/12 segment structures.
5. **Memory Exhaustion via Object Flooding:** Absurd `/Size` (e.g. $10^9$) or `/N` object counts allocating huge hash maps.

---

## 2. Mitigation Matrix & Hardened Limits

| Vector | Failure Mode | Mitigation in StarPDF | Status |
|---|---|---|---|
| **Deflate Bombs** | OOM | `DecompressLimits::max_decoded_bytes = 64 MB`, `max_expansion_ratio = 100x` | Verified |
| **Cyclic /Prev** | Infinite loop | `max_xref_chain_depth = 64` + cycle detection set | Verified |
| **Object Stream Flood** | Memory exhaustion | `max_object_stream_objects = 10,000` | Verified |
| **XRef Table Overflow**| Allocation spike | `max_xref_entries = 1,000,000` | Verified |
| **Parser Nesting** | Stack overflow | `max_parser_recursion = 64` | Verified |
| **Hostile Font Tables** | Panic / out-of-bounds | Zero-copy slice bounds checking, maximum segment limits (4096 fmt 4, 65536 fmt 12) | Verified |
| **Unwrap/Expect** | Uncaught panic | **0** `.unwrap()` or `.expect()` in production library code | Verified |

---

## 3. Fuzzing History & Vulnerability Resolution Log

- **Fuzz Target:** `fuzz_text_search` (libFuzzer / cargo-fuzz)
- **Symptom:** Non-ASCII / multi-byte UTF-8 string slicing panicked with `byte index is not a char boundary`.
- **Root Cause:** Slicing byte offsets on `&str` when searching across multi-byte characters.
- **Resolution:** Replaced byte-offset search with char-indexed arrays (`Vec<char>` and `char_map`).
- **Regression Test:** Added in `tests/fuzz_hardening_tests.rs::test_fuzz_search_engine_hostile_inputs`.
