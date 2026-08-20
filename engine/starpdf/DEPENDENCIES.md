# StarPDF Dependency Audit

**Status:** v0.1 Foundation  
**External Runtime Dependencies:** 0  
**External Dev Dependencies:** 0  

---

## Direct Runtime Dependencies

| Crate | Purpose | Why std is Insufficient | Binary Impact | In Hot Path? | Can Be Removed? |
|---|---|---|---|---|---|
| *(None)* | — | N/A (std library is completely sufficient) | 0 KB | N/A | N/A |

---

## Architectural Principles on Dependencies

1. **Zero External Crates on Core Parsing Path:**  
   StarPDF parses byte streams directly from memory buffers using native Rust slices `&[u8]`, standard collections (`std::collections::BTreeMap`, `std::vec::Vec`), and standard formatting/conversion.
2. **Panic Safety by Construction:**  
   No third-party parser macros or unsafe C/C++ FFI bindings are involved.
3. **Deterministic Memory Footprint:**  
   By avoiding heavy web or async frameworks (Tokio, Actix, Serde in core), StarPDF compiles to an ultra-light binary suitable for embedded systems, CLI tooling, WebAssembly (Wasm), and serverless environments.
