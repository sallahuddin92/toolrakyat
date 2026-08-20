# StarPDF Dependency Audit

**Status:** v0.2 Modern PDF Container Compatibility  
**External Runtime Dependencies:** 1 (`miniz_oxide`)  
**External Dev Dependencies:** 0  

---

## Direct Runtime Dependencies

| Crate | Purpose | Why std is Insufficient | Binary Impact | In Hot Path? | Can Be Removed? |
|---|---|---|---|---|---|
| `miniz_oxide` (0.9.1) | RFC 1950 / 1951 Deflate decompression for `/FlateDecode` and PNG predictor pipelines | Rust `std` contains no compression/decompression algorithms | ~45 KB | Yes (compressed streams & xref streams) | No (standards-compliant Deflate is required for modern PDF 1.5+ containers) |

---

## Architectural Principles on Dependencies

1. **Zero External Crates on Core Parsing Path:**  
   StarPDF parses byte streams directly from memory buffers using native Rust slices `&[u8]`, standard collections (`std::collections::BTreeMap`, `std::vec::Vec`), and standard formatting/conversion.
2. **Panic Safety by Construction:**  
   No third-party parser macros or unsafe C/C++ FFI bindings are involved.
3. **Deterministic Memory Footprint:**  
   By avoiding heavy web or async frameworks (Tokio, Actix, Serde in core), StarPDF compiles to an ultra-light binary suitable for embedded systems, CLI tooling, WebAssembly (Wasm), and serverless environments.
