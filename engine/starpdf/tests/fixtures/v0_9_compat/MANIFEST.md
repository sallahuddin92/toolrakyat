# StarPDF v0.9 local producer corpus

These fixtures were authored locally from the adjacent `sources/` files. They contain no downloaded or private document content. The producer versions are the versions installed on the development machine on 2026-08-20.

Every row passed the automated compatibility sequence in `compatibility_v0_9_tests.rs`: open, page count, native text extraction, search, form/annotation inspection, generated Square annotation appearance, incremental export with original-prefix preservation, reopen, and semantic reinspection. PDF.js rendering is exercised by the browser suite, with feature-specific synthetic fixtures for forms and rotated widgets.

| Fixture id | Producer | PDF | Xref / object streams | Fonts | Pages / rotation | Forms / original annotations | Incremental input | Result | Differential |
|---|---|---:|---|---|---|---|---|---|---|
| chrome-simple | Google Chrome 151.0.7922.140 | 1.4 | classic / no | subset CID TrueType, Identity-H, ToUnicode | 1 / 0° | none / none | no | PASS | AGREE |
| chrome-unicode | Google Chrome 151.0.7922.140 | 1.4 | classic / no | subset CID TrueType, Identity-H, ToUnicode | 1 / 0° | none / none | no | PASS | AGREE |
| chrome-landscape | Google Chrome 151.0.7922.140 | 1.4 | classic / no | subset CID TrueType, Identity-H, ToUnicode | 1 landscape MediaBox / 0° | none / none | no | PASS | AGREE |
| chrome-multipage | Google Chrome 151.0.7922.140 | 1.4 | classic / no | subset CID TrueType, Identity-H, ToUnicode | 2 / 0° | none / none | no | PASS | AGREE |
| libreoffice-basic | LibreOfficeDev 26.8.0.0.alpha0 | 1.7 | classic / no | subset TrueType, WinAnsi | 1 / 0° | none / none | no | PASS | AGREE |
| libreoffice-styled | LibreOfficeDev 26.8.0.0.alpha0 | 1.7 | classic / no | subset TrueType, WinAnsi | 1 / 0° | none / none | no | PASS | AGREE |
| libreoffice-table | LibreOfficeDev 26.8.0.0.alpha0 | 1.7 | classic / no | subset TrueType, WinAnsi | 1 / 0° | none / none | no | PASS | AGREE |
| libreoffice-unicode | LibreOfficeDev 26.8.0.0.alpha0 | 1.7 | classic / no | subset TrueType, WinAnsi | 1 / 0° | none / none | no | PASS | AGREE |
| quartz-simple | macOS Quartz/CUPS 26.5 | 1.3 | classic / no | subset TrueType | 1 / 0° | none / none | no | PASS | AGREE |
| quartz-columns | macOS Quartz/CUPS 26.5 | 1.3 | classic / no | subset TrueType | 1 / 0° | none / none | no | PASS | AGREE |
| quartz-unicode | macOS Quartz/CUPS 26.5 | 1.3 | classic / no | subset TrueType, MacRoman/WinAnsi | 1 / 0° | none / none | no | PASS | AGREE |
| quartz-multipage | macOS Quartz/CUPS 26.5 | 1.3 | classic / no | subset TrueType | 2 / 0° | none / none | no | PASS | AGREE |

Supplemental existing fixture `test-assets/smartpdf-form.pdf` supplies PDF 1.7 xref-stream, object-stream, AcroForm, and widget-annotation coverage. It also passes open/form inspection. Deterministic Rust regressions supply actual `/Rotate`, widget `/MK /R` at 90°/180°/270°, text/checkbox/choice appearances, embedded subset resources, three sequential incremental generations, and non-widget annotations. These are called out as supplemental rather than attributed to an independent producer.

The real producer corpus gap is original producer-authored AcroForm fields, arbitrary annotations, and `/Rotate` page dictionaries. Installed GUI applications were not automated once three independent producers and the required parser/font structures were covered; the gap is explicit rather than represented by fabricated provenance.
