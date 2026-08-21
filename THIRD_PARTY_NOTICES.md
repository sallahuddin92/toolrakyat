# Third-Party Software Notices and Information

This product (SmartPDF / StarPDF) includes software developed by third parties under open source licenses. This document contains the notices, acknowledgments, and license terms applicable to these components.

---

## 1. Primary Runtime JavaScript / TypeScript Dependencies

### PDF.js (`pdfjs-dist`)
- **Version**: 5.6.205
- **License**: Apache License, Version 2.0
- **Copyright**: Mozilla and individual contributors
- **Notice**:
```
Copyright 2012 Mozilla Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

### `pdf-lib`
- **Version**: 1.17.1
- **License**: MIT License
- **Copyright**: (c) 2019 Andrew Dillon
- **Notice**:
```
MIT License

Copyright (c) 2019 Andrew Dillon

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

### React & React DOM
- **Version**: 19.2.4
- **License**: MIT License
- **Copyright**: (c) Meta Platforms, Inc. and affiliates.

---

### Next.js
- **Version**: 16.2.4
- **License**: MIT License
- **Copyright**: (c) 2024 Vercel, Inc.

---

### Lucide React (`lucide-react`)
- **Version**: 1.9.0
- **License**: ISC License
- **Copyright**: (c) 2022 Lucide Contributors

---

### Radix UI (`@radix-ui/*`)
- **License**: MIT License
- **Copyright**: (c) 2022 WorkOS

---

### UI & State Utilities (`clsx`, `tailwind-merge`, `class-variance-authority`, `zustand`)
- **License**: MIT License

---

## 2. WebAssembly & Rust Engine Shipped Dependencies (`starpdf`)

| Package / Crate | Version | License |
| :--- | :--- | :--- |
| `starpdf` | 0.1.0 | MIT OR Apache-2.0 |
| `miniz_oxide` | 0.9.1 | MIT OR Apache-2.0 OR Zlib |
| `adler2` | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| `serde` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 |
| `serde-wasm-bindgen` | 0.6.5 | MIT OR Apache-2.0 |
| `wasm-bindgen` | 0.2.127 | MIT OR Apache-2.0 |
| `wasm-bindgen-macro` | 0.2.127 | MIT OR Apache-2.0 |
| `wasm-bindgen-macro-support` | 0.2.127 | MIT OR Apache-2.0 |
| `wasm-bindgen-shared` | 0.2.127 | MIT OR Apache-2.0 |
| `js-sys` | 0.3.104 | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | MIT OR Apache-2.0 |
| `syn` | 2.0.119 / 3.0.3 | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.24 | MIT OR Apache-2.0 |
| `bumpalo` | 3.20.3 | MIT OR Apache-2.0 |
| `cfg-if` | 1.0.4 | MIT OR Apache-2.0 |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 |
| `futures-core`, `futures-util`, `futures-task` | 0.3.34 | MIT OR Apache-2.0 |
| `pin-project-lite` | 0.2.17 | MIT OR Apache-2.0 |
| `slab` | 0.4.12 | MIT |
| `rustversion` | 1.0.23 | MIT OR Apache-2.0 |

---

## 3. Production Dependency Graph Audit Summary

- **Total Transitive Production NPM Packages**: 623
- **Total Transitive Rust Crates**: 23
- **Unknown Licenses**: 0
- **Known License Blockers**: NONE FOUND
- **Manual Review Required**: 0 (all components operate under standard permissive open source licenses: MIT, Apache-2.0, ISC, BSD-2-Clause, BSD-3-Clause, 0BSD, BlueOak-1.0.0, Zlib, or dual-license with MIT).
