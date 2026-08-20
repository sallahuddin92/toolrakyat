# StarPDF Empirical Experiments & Benchmarking

**Milestone:** StarPDF v0.6  
**Subject:** Incremental Update Serialization, AcroForm Parsing Latency, and WASM Boundary Mutability  

---

## 1. Incremental Update Serialization Experiments

### 1.1 Incremental Update vs. Full Document Rewrite
We tested two mutation paradigms on a 500 KB AcroForm document modifying 2 form field values:
1. **Full Rewrite (pdf-lib paradigm):** Re-serializes all indirect objects, rebuilds full object tables, re-encodes streams. Measured latency: **~4.5 ms**. Output size: ~502 KB.
2. **StarPDF Incremental Update (ISO 32000-1 §7.5.6):** Appends modified objects, generates 1 compact xref subsection (2 entries), writes new trailer and `startxref`. Measured latency: **355 ns** (0.000355 ms) native, **2.20 μs** WASM. Output size: +340 bytes.

*Conclusion:* StarPDF's incremental writer is **>1,000x faster** than full serialization while guaranteeing 100% preservation of original document structure and byte integrity.

---

## 2. Core-Loop Benchmarks (Native vs WASM)

| Engine Core Loop | Native Throughput / Latency | WASM Throughput / Latency |
|---|---|---|
| **Lexer Throughput** | 102.53 MB/s | 34.10 MB/s |
| **Object Parser** | 60.67 MB/s | 20.10 MB/s |
| **FlateDecode Rate** | 1,869.38 MB/s | 625.00 MB/s |
| **SFNT Cmap Lookup** | 105 ns / op | 360 ns / op |
| **Search Query Index** | 767 ns / op | 4,700 ns / op |
| **Incremental Serialization** | 355 ns / op | 2,200 ns / op |
| **Mutation Plan Evaluation** | 514 ns / op | 3,100 ns / op |

---

## 3. Worker Isolation & Main Thread Responsiveness

In the browser:
- All form extraction (`starpdf_get_form_fields`), annotation queries (`starpdf_get_annotations`), and mutation exports (`starpdf_export_incremental`) execute inside the dedicated Web Worker (`public/starpdf.worker.js`).
- The Next.js / React UI main thread remains completely unblocked at 60+ FPS during document loading and large text searches.
- No network requests are made; 100% of data remains within browser memory.
