# StarPDF Phase 8 Real-World Qualification Corpus Manifest

This corpus expands StarPDF testing beyond the original 52 baseline fixtures to qualify GPU-native rendering, text rasterization, and desktop interactive workflows on diverse real-world PDF documents.

All fixtures in this directory were generated locally using authenticated producer toolchains (`PyMuPDF/fitz` 1.24.10 and SIL Open Font License 1.1 fonts from Google Noto). They contain no proprietary, private, or copyrighted content.

## Fixture Registry & Structural Coverage

| Fixture ID | Category | Producer / Toolchain | License | Pages / Rotation | Font Structures | Vector & Graphic Structures | Forms & Annotations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `office-invoice-enterprise.pdf` | Office / Financial | PyMuPDF 1.24.10 | MIT / Public Domain | 1 page / 0° | Standard Type1 Helvetica, numeric tabular alignment | Slate header banner, item table grid lines, total highlight rect | None |
| `office-technical-report.pdf` | Office / Engineering | PyMuPDF 1.24.10 | MIT / Public Domain | 3 pages / 0° | Helvetica, Courier, bold titles, code snippets | Multi-column layout boxes, running headers/footers, divider lines | None |
| `scanned-multires-archive.pdf` | Scan / Archival | PyMuPDF 1.24.10 + PIL | MIT / Public Domain | 1 page / 0° | Synthetic typewriter + OCR stamp | High-res full-page JPEG scan (800×1131), paper noise, overlaid vector stamp | Mixed raster/vector z-order verification |
| `multilingual-arabic-jawi.pdf` | Multilingual Text | PyMuPDF + NotoSansArabic | SIL OFL 1.1 | 1 page / 0° | Embedded TrueType NotoSansArabic, complex glyph outlines | Card background, accent borders | RTL text flow, Arabic/Jawi ligatures |
| `multilingual-cjk-unicode.pdf` | Multilingual Text | PyMuPDF + NotoSansSC | SIL OFL 1.1 | 1 page / 0° | Embedded CID TrueType NotoSansSC, high-density kanji/hanzi strokes | Rounded cards, section dividers | Simplified Chinese, Traditional Chinese, Japanese, Korean |
| `multilingual-hebrew-bidi.pdf` | Multilingual Text | PyMuPDF + NotoSansHebrew | SIL OFL 1.1 | 1 page / 0° | Embedded TrueType NotoSansHebrew | Warm tone background cards, line dividers | BiDi text with inline LTR numerals and English technical terms |
| `forms-employment-application.pdf` | Interactive Forms | PyMuPDF 1.24.10 | MIT / Public Domain | 1 page / 0° | Helvetica, Courier | Section header bars, styled input borders, signature bounding box | ISO 32000-1 AcroForm: 4 Text fields, 3 Checkboxes, Signature line |
| `annotations-markup-review.pdf` | Markup & Legal | PyMuPDF 1.24.10 | MIT / Public Domain | 1 page / 0° | Helvetica | Legal paragraph boundaries, redaction blackout box | Highlight, Underline, FreeText callout, Ink signature, Redaction square |
| `large-manual-20page.pdf` | Large Document Stress | PyMuPDF 1.24.10 | MIT / Public Domain | 20 pages / 0° | Helvetica, Courier | Running headers, telemetry boxes, decorative multi-tone vector bars | Stress test: 20 independent pages, rapid scrolling, bounded memory |
| `complex-infographic-graphics.pdf` | Vector Graphics | PyMuPDF 1.24.10 | MIT / Public Domain | 1 page / 0° | Helvetica | Concentric alpha circles, cubic Bezier splines, 10-point star, stacked rounded rects | Alpha blending, non-zero/even-odd fills, translucent stacking |

## Verification Objectives
1. **GPU Rendering Parity:** Every page must render directly through `WgpuRenderer` with zero crashes or hangs.
2. **Text & Font Fidelity:** Outlines extracted from embedded TrueType/CID tables must be tessellated directly on the GPU without CPU fallback when supported.
3. **Compositing Order:** Layering across vector shapes, text spans, annotations, and bitmap scans must strictly follow document z-order.
4. **Desktop Compatibility:** All fixtures must load, scroll, zoom, and select cleanly within the Slint desktop interface.
